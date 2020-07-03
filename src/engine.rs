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
    inner: Arc<JobEngineInner>,
}

impl JobEngine {
    /// Creates a new job engine that is running and ready to process jobs.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(JobEngineInner::new()),
        }
    }

    /// Pauses the job engine.
    // This does not clear out the list of pending jobs, nor does it stop the
    // currently executing job, if any. However, after that job has completed
    // no new jobs will begin to execute.
    pub fn pause(&self) {
        self.inner.pause();
    }

    /// Restarts the job engine after a pause.
    pub fn restart(&self) {
        self.inner.restart();
    }

    /// Add a job to the end of the queue.
    pub fn add_job(&self, job: Job) {
        self.inner.add_job(job);
    }
}

type JobList = Arc<Mutex<VecDeque<Job>>>;

#[derive(Debug)]
struct JobEngineInner {
    // The list of pending (yet to be executed) jobs.
    pending_jobs: JobList,

    // The list of completed jobs.
    completed_jobs: JobList,

    // A clutch that allows us to pause and restart the JOB_STARTER thread.
    // This basically allows us to pause the entire job queue, because if we
    // don't start to execute new jobs, nothing happens. Yet we can still
    // add new jobs to the queue, because that is controlled by a different thread.
    job_starter_clutch: ThreadClutch,

    // The `job_added_signal` is notified when a new job is added to the pending queue.
    // This will cause the JOB_STARTER thread to wake up (it goes to sleep when
    // there are no pending jobs).
    job_added_signal: Arc<Condvar>,
}

impl JobEngineInner {
    pub fn new() -> Self {
        let (job_exec_sender, job_exec_receiver) = Self::create_job_executor_thread();

        let pending_jobs: JobList = Default::default();
        let completed_jobs: JobList = Default::default();

        let job_added_signal = Arc::new(Condvar::new());

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
            job_added_signal.clone(),
        );

        Self {
            pending_jobs,
            completed_jobs,
            job_starter_clutch,
            job_added_signal: job_added_signal,
        }
    }

    pub fn pause(&self) {
        info!("JobEngine paused");
        self.job_starter_clutch.pause_threads();
    }

    pub fn restart(&self) {
        info!("JobEngine restarting");
        self.job_starter_clutch.release_threads();
    }

    pub fn add_job(&self, job: Job) {
        // This lock won't block the caller much, because all other locks
        // on the `pending_jobs` are very short lived.
        let mut lock = self.pending_jobs.lock().unwrap();

        info!(
            "Added {}, there are now {} jobs in the pending queue",
            job,
            lock.len() + 1
        );

        lock.push_back(job);

        // Tell everybody listening (really it's just us with one thread) that there
        // is now a job in the pending queue.
        self.job_added_signal.notify_all();
    }

    /// Create the JOB_STARTER thread. This thread is responsible for checking the
    /// `pending_jobs` queue to see if there are any jobs that need executing, and if
    /// there are it clones them and sends them to the JOB_EXECUTOR thread.
    /// If there are no pending jobs then it goes to sleep, until it is woken up by
    /// `add_job` notifying the signal.
    /// Q: Why not use a channel with a for loop like the other threads, since that would
    /// allow us to dispense with the Condvar? A: We want the list of pending jobs to
    /// be observable / iterable, so we must maintain our own queue, we can't use the
    /// channels queue because iterating it consumes the items.
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

    /// Create the JOB_EXECUTOR thread. This thread just calls
    /// `Job.execute()`, one job at a time. It receives jobs on a channel, and sends
    /// the results back on another channel (where they are picked up by
    /// the JOB_COMPLETED thread).
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

    /// Create the JOB_COMPLETED thread. It is the job of this thread to listen
    /// for completed job messages which are sent by the JOB_EXECUTOR thread.
    fn create_job_completed_thread(
        pending_jobs: JobList,
        completed_jobs: JobList,
        job_exec_receiver: Receiver<Job>,
    ) {
        let builder = thread::Builder::new().name("JOB_COMPLETED".into());

        builder
            .spawn(move || {
                for job in job_exec_receiver {
                    let mut pending_jobs_lock = pending_jobs.lock().unwrap();

                    // Find this job by id in the list of pending jobs. It may not be there, if we
                    // 'tweaked' the job queue while this one was executing. But if we do
                    // find it, then remove it and add it to the list of completed jobs.
                    // If it's not found, just ignore it.
                    if let Some(index) = pending_jobs_lock.iter().position(|j| j.id() == job.id()) {
                        pending_jobs_lock.remove(index);
                        let pj_len = pending_jobs_lock.len();
                        // Release lock ASAP.
                        drop(pending_jobs_lock);

                        let mut completed_jobs_lock = completed_jobs.lock().unwrap();
                        let msg = format!(
                            "Completed {}, there are now {} pending and {} completed jobs",
                            job,
                            pj_len,
                            completed_jobs_lock.len() + 1
                        );
                        completed_jobs_lock.push_back(job);
                        drop(completed_jobs_lock);

                        info!("{}", msg);
                    }
                }
            })
            .expect("Cannot create JOB_COMPLETED thread");
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
