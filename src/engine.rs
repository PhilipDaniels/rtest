use crate::job::Job;
use log::info;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

pub struct JobQueue {
    jobs: Mutex<VecDeque<Job>>,
    cvar: Condvar,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(VecDeque::<Job>::new()),
            cvar: Condvar::new(),
        }
    }

    pub fn clear(&mut self) {
        let mut lock = self.jobs.lock().unwrap();
        lock.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let lock = self.jobs.lock().unwrap();
        lock.len()
    }

    pub fn push_back(&self, job: Job) {
        let mut lock = self.jobs.lock().unwrap();
        lock.push_back(job);
        self.cvar.notify_all();
    }

    // pub fn pop_front(&self, job: Job) -> Option<Job> {
    //     let mut lock = self.jobs.lock().unwrap();
    //     lock.pop_front()
    // }

    fn get_next_job(&self) -> Job {
        let mut jobs = self.jobs.lock().unwrap();
        loop {
            match jobs.pop_front() {
                Some(job) => return job,
                None => {
                    info!("All jobs processed, sleeping");
                    jobs = self.cvar.wait(jobs).unwrap();
                }
            }
        }
    }
}

pub struct JobEngine {
    pending_jobs: Arc<JobQueue>,
    job_processor: JoinHandle<()>,
}

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngine {
    pub fn new() -> Self {
        let pending_jobs = Arc::new(JobQueue::new());

        let job_processor = thread::spawn({
            let pending_jobs = pending_jobs.clone();
            move || loop {
                let mut job = pending_jobs.get_next_job();
                job.execute();
            }
        });

        Self {
            pending_jobs,
            job_processor,
        }
    }

    pub fn add_job(&mut self, job: Job) {
        self.pending_jobs.push_back(job);
    }

    pub fn is_empty(&self) -> bool {
        self.pending_jobs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pending_jobs.len()
    }
}
