use crate::jobs::Job;
use log::info;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, atomic::{Ordering, AtomicBool}};
use std::thread::{self, JoinHandle};


enum JobEngineState {
    Stopped,
    WaitingForWork,
    Running
}

pub struct JobEngine {
    jobs: Arc<Mutex<VecDeque<Job>>>,
    job_signal: Arc<Condvar>,
    //enabled: Arc<AtomicBool>,
}

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngine {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            job_signal: Arc::new(Condvar::new()),
      //      enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start(&self) {
        let jobs = self.jobs.clone();
        let job_signal = self.job_signal.clone();
        //let enabled = self.enabled.clone();

        thread::spawn({
            move || loop {
                // if !enabled.load(Ordering::SeqCst) {
                //     return;
                // }

                let mut jobs = jobs.lock().unwrap();
                let next_job = jobs.iter_mut().find(|job: &&mut Job|  job.is_pending());

                match next_job {
                    Some(job) => {
                        job.execute();
                    }
                    None => {
                        info!("All jobs processed, sleeping");
                        jobs = job_signal.wait(jobs).unwrap();
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
    }

    pub fn add_job(&self, job: Job) {
        assert!(job.is_pending());
        let mut job_lock = self.jobs.lock().unwrap();
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

    // pub fn num_completed_len(&self) -> usize {
    //     let job_lock = self.completed_jobs.lock().unwrap();
    //     job_lock.len()
    // }
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
