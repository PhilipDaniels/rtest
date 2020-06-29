use crate::{jobs::Job, thread_clutch::ThreadClutch};
use log::info;
use std::collections::VecDeque;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Condvar, Mutex,
};
use std::thread;

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

        let job_starter_clutch = ThreadClutch::default();
        Self::create_job_starter_thread(
            job_starter_clutch.clone(),
            pending_jobs.clone(),
            job_exec_sender,
            job_adder_signal.clone(),
        );

        Self {
            pending_jobs,
            completed_jobs,
            job_starter_clutch,
            job_adder_sender: Mutex::new(job_adder_sender),
            job_adder_signal,
        }
    }

    pub fn start(&self) {}

    pub fn pause(&self) {
        info!("JobEngine paused");
        self.job_starter_clutch.pause_threads();
    }

    pub fn restart(&self) {
        info!("JobEngine restarting");
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
                    // TODO: Tidy up the JobState management.
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
                        "Added {}, there are now {} jobs in the pending queue",
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

    fn create_job_starter_thread(
        job_starter_clutch: ThreadClutch,
        mut pending_jobs: JobList,
        job_exec_sender: Sender<Job>,
        signal: Arc<Condvar>,
    ) {
        let builder = thread::Builder::new().name("JOB_STARTER".into());

        let dummy_mutex = Mutex::new(());

        builder
            .spawn(move || {
                loop {
                    job_starter_clutch.wait_for_release();

                    if let Some(job) = Self::get_next_job(&mut pending_jobs) {
                        job_exec_sender
                            .send(job)
                            .expect("Could not send job to JOB_EXECUTOR thread");
                    } else {
                        // No jobs exist, go to sleep waiting for a signal on the condition variable.
                        // This will be signaled by `add_job`.

                        // The idea here is that this will BLOCK and you are not allowed to touch the
                        // data guarded by the MUTEX until the signal happens.
                        let guard = dummy_mutex.lock().unwrap();
                        let _ = signal.wait(guard).unwrap();
                    }
                }
            })
            .expect("Cannot create JOB_STARTER thread");
    }

    fn get_next_job(pending_jobs: &mut JobList) -> Option<Job> {
        let mut pending_jobs = pending_jobs.lock().unwrap();
        for job in pending_jobs.iter_mut() {
            if job.is_pending() {
                // Mark the job while it remains in the queue, so that we
                // skip over it the next time.
                job.begin_execution();
                return Some(job.clone());
            }
        }

        None
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

Algorithm for adding file sync
FOR SOME PATH P
If a build is running, stop it
If OP is REMOVE, remove all file copy jobs and create a remove job
ELSE
    if there is a previous job for this file, remove it and insert a new COPY job (op is likely to be WRITE, CLOSE_WRITE, RENAME or CHMOD)

Flag for 'is a build required'?
*/
