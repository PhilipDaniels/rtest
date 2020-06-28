use crate::jobs::Job;
use log::info;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

pub struct JobEngine {
    jobs: Arc<Mutex<VecDeque<Job>>>,
    job_signal: Arc<Condvar>,
    worker: Option<JoinHandle<()>>,
    pause_flag: Arc<Mutex<bool>>,
    pause_signal: Arc<Condvar>,
}

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngine {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            job_signal: Arc::new(Condvar::new()),
            worker: None,
            pause_flag: Arc::new(Mutex::new(false)),
            pause_signal: Arc::new(Condvar::new()),
        }
    }

    pub fn start(&mut self) {
        // If there is a worker, we're already started.
        if self.worker.is_some() {
            return;
        }

        info!("Starting job engine");

        let jobs = self.jobs.clone();
        let job_signal = self.job_signal.clone();
        let pause_flag = self.pause_flag.clone();
        let pause_signal = self.pause_signal.clone();
        let builder = thread::Builder::new().name("JobWorker".into());

        let join_handle = builder
            .spawn(move || {
                // The loop makes this thread run forever.
                loop {
                    {
                        // Wait for pause_flag to go `false`. Do this by waiting using the Condvar,
                        // this means we are not using CPU cycles. The extra scope is
                        // so that the `pause_flag_guard` gets dropped and the lock released.
                        let mut pause_flag_guard = pause_flag.lock().unwrap();
                        while *pause_flag_guard {
                            info!("Pausing JobWorker thread inside itself");
                            pause_flag_guard = pause_signal.wait(pause_flag_guard).unwrap();
                        }
                    }

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
                            info!("All jobs processed, sleeping");
                            // Here we just wait for work to become available. We don't care about the
                            // return value of `wait`, because we will re-get `jobs_guard` above,
                            // when we come round the loop again.
                            let _ = job_signal.wait(jobs_guard).unwrap();
                        }
                    }
                }
            })
            .expect("Expected to create the JobWorker thread");

        info!("Successfully spawned JobWorker thread");

        self.worker = Some(join_handle);
    }

    pub fn pause(&mut self) {
        info!("Pausing JobWorker thread");
        let mut pause_flag = self.pause_flag.lock().unwrap();
        *pause_flag = true;
        self.pause_signal.notify_all();
    }

    pub fn restart(&mut self) {
        info!("Restarting JobWorker thread");
        let mut pause_flag = self.pause_flag.lock().unwrap();
        *pause_flag = false;
        self.pause_signal.notify_all();
    }

    pub fn add_job(&self, job: Job) {
        assert!(job.is_pending());
        let mut job_lock = self.jobs.lock().unwrap();
        info!(
            "Added {}, there are now {} jobs in the queue",
            job,
            job_lock.len() + 1
        );
        job_lock.push_back(job);

        // Tell everybody listening (really it's just us with one thread) that there is now
        // a job in the pending queue.
        self.job_signal.notify_all();
    }

    pub fn num_pending(&self) -> usize {
        let job_lock = self.jobs.lock().unwrap();
        job_lock.len()
    }

    pub fn is_empty(&self) -> bool {
        let job_lock = self.jobs.lock().unwrap();
        job_lock.is_empty()
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
