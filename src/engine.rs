use crate::jobs::Job;
use log::{error, info};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

pub struct JobEngine {
    jobs: Arc<Mutex<VecDeque<Job>>>,
    job_signal: Arc<Condvar>,
    worker: Option<JoinHandle<()>>,
}

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngine {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            job_signal: Arc::new(Condvar::new()),
            worker: None,
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
        let builder = thread::Builder::new().name("JobWorker".into());

        let join_handle = builder
            .spawn(move || loop {
                let mut jobs = jobs.lock().unwrap();
                let next_job = jobs.iter_mut().find(|job: &&mut Job| job.is_pending());

                match next_job {
                    Some(job) => {
                        job.execute();
                    }
                    None => {
                        info!("All jobs processed, sleeping");
                        jobs = job_signal.wait(jobs).unwrap();
                    }
                }
            })
            .expect("Expected to create the JobWorker thread");

        self.worker = Some(join_handle);
    }

    /// FIXME: You can't join a thread that has called `wait`.
    /// Need more sophisticated state transition management.
    /// Don't really need to kill the thread, just make it wait some more.
    pub fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(_) => info!("Successfully terminated the JobWorker thread"),
                Err(err) => error!("Error terminating worker thread {:?}", err),
            }
        }
    }

    pub fn add_job(&self, job: Job) {
        assert!(job.is_pending());
        let mut job_lock = self.jobs.lock().unwrap();
        info!(
            "{} added, there are now {} jobs in the queue",
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

Immediately create a watcher on the directory
Create shadow copy job and run it
Perform shadow copy
    Watcher events become new jobs that will execute after the shadow copy has finished
    All we care about are file-delete/update/create events
    We need to process them through .gitignore though

Some more concepts we have
    - Where the .gitignore files are and how to use them
*/
