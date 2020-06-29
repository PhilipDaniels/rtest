use crate::{jobs::Job, thread_clutch::ThreadClutch};
use log::info;
use std::collections::VecDeque;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Condvar, Mutex,
};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone)]
pub struct JobEngine {
    inner: Arc<JobEngineInner2>,
}

impl JobEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(JobEngineInner2::new()),
        }
    }
    pub fn start(&self) {
        self.inner.start();
    }

    pub fn pause(&self) {
        self.inner.pause();
    }

    pub fn restart(&self) {
        self.inner.restart();
    }

    pub fn add_job(&self, job: Job) {
        self.inner.add_job(job);
    }
}

type JobList = Arc<Mutex<VecDeque<Job>>>;

#[derive(Debug)]
struct JobEngineInner2 {
    // The list of pending (yet to be executed) jobs.
    pending_jobs: JobList,
    // The list of completed jobs.
    completed_jobs: JobList,

    // A clutch that allows us to pause and restart the JOB_STARTER thread.
    // This basically allows us to pause the entire job queue, because if we
    // don't start to execute new jobs, nothing happens. Yet we can still
    // add new jobs to the queue, because that is controlled by a different thread.
    job_starter_clutch: ThreadClutch,

    // A sender channel for communicating with the JOB_EXECUTOR thread.
    // It needs to be wrapped in a mutex so that we can clone
    // engines and send them across threads.
    job_exec_sender: Mutex<Sender<Job>>,

    // A channel to handle the addition of jobs to the queue.
    // The other end of this channel is monitored by the JOB_ADDER thread.
    job_adder_sender: Mutex<Sender<Job>>,
    job_adder_signal: Arc<Condvar>,
}

impl JobEngineInner2 {
    pub fn new() -> Self {
        let (job_exec_sender, job_exec_receiver) = Self::create_job_executor_thread();

        let pending_jobs: JobList = Default::default();
        let completed_jobs: JobList = Default::default();

        let job_adder_signal = Arc::new(Condvar::new());
        let job_adder_sender =
            Self::create_job_adder_thread(pending_jobs.clone(), job_adder_signal.clone());

        Self::create_job_completed_thread(
            pending_jobs.clone(),
            completed_jobs.clone(),
            job_exec_receiver,
        );

        Self {
            pending_jobs,
            completed_jobs,
            job_starter_clutch: Default::default(),
            job_exec_sender: Mutex::new(job_exec_sender),
            job_adder_sender: Mutex::new(job_adder_sender),
            job_adder_signal,
        }
    }

    pub fn start(&self) {}

    pub fn pause(&self) {
        self.job_starter_clutch.pause_threads();
    }

    pub fn restart(&self) {
        self.job_starter_clutch.release_threads();
    }

    pub fn add_job(&self, job: Job) {
        let lock = self.job_adder_sender.lock().unwrap();
        lock.send(job).expect("Could not send job to JOB_ADDER");
    }

    /// Create the JOB_EXECUTOR thread. This is the simplest thread, it just calls
    /// `Job.execute`, one job at a time. It receives jobs on a channel, and sends
    /// the results back on another channel.
    fn create_job_executor_thread() -> (Sender<Job>, Receiver<Job>) {
        let (job_exec_sender, job_exec_internal_receiver) = channel::<Job>();
        let (job_exec_internal_sender, job_exec_receiver) = channel::<Job>();

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());

        builder
            .spawn(move || {
                for mut job in job_exec_internal_receiver {
                    job.execute();
                    job_exec_internal_sender
                        .send(job)
                        .expect("Cannot return job from JOB_EXECUTOR");
                }
            })
            .expect("Cannot create JOB_EXECUTOR thread");

        (job_exec_sender, job_exec_receiver)
    }

    fn create_job_adder_thread(pending_jobs: JobList, signal: Arc<Condvar>) -> Sender<Job> {
        let (sender, receiver) = channel();

        let builder = thread::Builder::new().name("JOB_ADDER".into());

        builder
            .spawn(move || {
                for job in receiver {
                    let mut lock = pending_jobs.lock().unwrap();
                    info!(
                        "Added {}, there are now {} jobs in the queue",
                        job,
                        lock.len() + 1
                    );

                    lock.push_back(job);

                    // Tell everybody listening (really it's just us with one thread) that there
                    // is now a job in the pending queue.
                    signal.notify_all();
                }
            })
            .expect("Cannot create JOB_ADDER thread");

        sender
    }

    fn create_job_completed_thread(
        pending_jobs: JobList,
        completed_jobs: JobList,
        job_exec_receiver: Receiver<Job>,
    ) {
        let builder = thread::Builder::new().name("JOB_COMPLETED".into());

        builder
            .spawn(move || {
                for job in job_exec_receiver {
                    let mut pending_jobs = pending_jobs.lock().unwrap();
                    // Find this job by id in the list of pending jobs. It may not be there, if we
                    // 'tweaked' the job queue while this one was executing. But if we do
                    // find it, then remove it and add it to the list of completed jobs.
                    // If it's not found, just ignore it.
                    if let Some(index) = pending_jobs.iter().position(|j| j.id() == job.id()) {
                        pending_jobs.remove(index);

                        let mut completed_jobs = completed_jobs.lock().unwrap();

                        info!(
                            "Completed {}, there are now {} pending and {} completed jobs",
                            job,
                            pending_jobs.len(),
                            completed_jobs.len() + 1
                        );

                        completed_jobs.push_back(job);

                    }
                }
            })
            .expect("Cannot create JOB_COMPLETED thread");
    }
}

#[derive(Debug, Clone, Default)]
struct JobEngineInner {
    pending_jobs: Arc<Mutex<VecDeque<Job>>>,
    completed_jobs: Arc<Mutex<VecDeque<Job>>>,
    pending_job_condvar: Arc<Condvar>,
    pause_queue_clutch: ThreadClutch,
    //queue_manager_join_handle: Option<JoinHandle<()>>,
}

const JOB_EXECUTOR_THREAD_NAME: &str = "JOB_EXECUTOR";

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngineInner {
    pub fn new() -> Self {
        Self {
            pending_jobs: Arc::new(Mutex::new(VecDeque::new())),
            completed_jobs: Arc::new(Mutex::new(VecDeque::new())),
            pending_job_condvar: Arc::new(Condvar::new()),
            //queue_manager_join_handle: None,
            pause_queue_clutch: Default::default(),
        }
    }

    pub fn start(&mut self) {
        // If there is a queue manager, we're already started.
        // if self.queue_manager_join_handle.is_some() {
        //     return;
        // }

        info!("Starting job engine");

        self.create_queue_mgr_thread();
    }

    fn create_queue_mgr_thread(&mut self) {
        const QUEUE_MGR_THREAD_NAME: &str = "QUEUE_MGR";

        let jobs = self.pending_jobs.clone();
        let pending_job_condvar = self.pending_job_condvar.clone();
        let clutch = self.pause_queue_clutch.clone();
        let builder = thread::Builder::new().name(QUEUE_MGR_THREAD_NAME.into());

        //self.queue_manager_join_handle =
        builder
            .spawn(move || {
                // The loop makes this thread run forever.
                loop {
                    clutch.wait_for_release();

                    // If we get to here then the worker thread is running. There may or may
                    // not be any pending jobs to process. Execute at most one job (so that
                    // there is a chance to pause between jobs).
                    let mut jobs_guard = jobs.lock().unwrap();
                    match jobs_guard
                        .iter_mut()
                        .find(|job: &&mut Job| job.is_pending())
                    {
                        Some(next_job) => next_job.execute(),
                        None => {
                            info!("{}: All jobs processed, sleeping", QUEUE_MGR_THREAD_NAME);
                            // Here we just wait for work to become available. We don't care about the
                            // return value of `wait`, because we will re-get `jobs_guard` above,
                            // when we come round the loop again.
                            let _ = pending_job_condvar.wait(jobs_guard).unwrap();
                        }
                    }
                }
            })
            .expect(&format!(
                "{} Failed to create the thread",
                QUEUE_MGR_THREAD_NAME
            ));

        info!("{}: Successfully spawned the thread", QUEUE_MGR_THREAD_NAME);
    }
}

/*
We need the following

* While a job is executing, the GUI needs to update to show the latest status.
* When a job is completed, we will still want to display details in the GUI.
  For example, a list of completed tests. So the GUI is based on Vec<Tests>,
  and each test is linked to a particular job. One job may be linked to
  several tests.
* We need to support cancellation of jobs.
* When a job finishes execution it may create N more jobs.

Alternative data structure
    - We maintain a list of jobs in a Vec but do not process in that Vec
    - A thread pulls jobs off and clones them, then executes them separately,
      perhaps using a channel.

Algorithm for adding file sync
FOR SOME PATH P
If a build is running, stop it
If OP is REMOVE, remove all file copy jobs and create a remove job
ELSE
    if there is a previous job for this file, remove it and insert a new COPY job (op is likely to be WRITE, CLOSE_WRITE, RENAME or CHMOD)


Flag for 'is a build required'?

While a job is executing, no other jobs are added to the queue. We need
to spawn a new thread to execute build jobs and test jobs.



THE LIST OF JOBS
  A QUEUE_MGR thread
    -- adds new jobs to the queue from the channel
    -- watches for PAUSE ENGINE command
    -- waits for jobs to execute and hands them off to the JOB_EXECUTOR thread
        to actually run (so as not to block this thread)

Jobs are removed from the queue after processing whether they failed or not
Move them to a DONE JOBS queue, along with their status and any output


*/
