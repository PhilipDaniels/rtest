use log::info;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::{
    ops::{Deref, DerefMut},
    thread::{self, JoinHandle},
};

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
    queue: Arc<JobQueue>,
    job_processor: JoinHandle<()>,
}

/// Based on https://www.poor.dev/posts/what-job-queue/.
impl JobEngine {
    pub fn new() -> Self {
        let queue = Arc::new(JobQueue::new());

        let job_processor = thread::spawn({
            let queue = queue.clone();
            move || loop {
                let job = queue.get_next_job();
                info!("TODO: Processing job");
                // When the job is complete we need to send a message
                // so that the UI can be updated.
            }
        });

        Self {
            queue,
            job_processor,
        }
    }

    pub fn add_job(&mut self, job: Job) {
        self.queue.push_back(job);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

pub enum Job {
    ShadowCopy,
}
