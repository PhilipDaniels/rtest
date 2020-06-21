use crate::jobs::Job;
use log::info;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

type JobQ = Arc<Mutex<VecDeque<Job>>>;

pub struct JobEngine {
    pending_jobs: JobQ,
    completed_jobs: JobQ,
    pending_jobs_signal_variable: Condvar,
    job_processor: JoinHandle<()>,
}

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngine {
    pub fn new() -> Self {
        let pending_jobs = Arc::new(Mutex::new(VecDeque::<Job>::new()));
        let completed_jobs = Arc::new(Mutex::new(VecDeque::<Job>::new()));

        let job_processor = thread::spawn({
            let pending_jobs = pending_jobs.clone();
            let completed_jobs = completed_jobs.clone();

            move || loop {
                let mut job_lock = pending_jobs.lock().unwrap();
                let next_job = job_lock.iter_mut().find(|job: &&mut Job|  job.is_pending());
                match next_job {
                    Some(job) => {
                        job.execute();
                        // Can't do this yet, need to remove the job from the pending_jobs queue.
                        // let mut cj = completed_jobs.lock().unwrap();
                        // cj.push_back(job);
                    }
                    None => {
                        info!("All jobs processed, sleeping");
                        //let jobs = pending_jobs_signal_variable.wait(jobs).unwrap();
                    }
                }
            }
        });

        Self {
            pending_jobs,
            completed_jobs,
            pending_jobs_signal_variable: Condvar::new(),
            job_processor,
        }
    }

    fn process_jobs(&self) {

    }

    pub fn add_job(&mut self, job: Job) {
        assert!(job.is_pending());
        let mut job_lock = self.pending_jobs.lock().unwrap();
        job_lock.push_back(job);
        // Tell everybody listening (really it's just us) that there is now
        // a job in the pending queue.
        self.pending_jobs_signal_variable.notify_all();
    }

    pub fn num_pending(&self) -> usize {
        let job_lock = self.pending_jobs.lock().unwrap();
        job_lock.len()
    }

    pub fn is_empty(&self) -> bool {
        let job_lock = self.pending_jobs.lock().unwrap();
        job_lock.is_empty()
    }

    pub fn num_completed_len(&self) -> usize {
        let job_lock = self.completed_jobs.lock().unwrap();
        job_lock.len()
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
