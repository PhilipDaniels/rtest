use chrono::{DateTime, Utc};
use log::info;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    thread::{self, JoinHandle}, fmt::Display,
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
                let job = pending_jobs.get_next_job();
                info!("Processing job: {}", job);
                // When the job is complete we need to send a message
                // so that the UI can be updated.
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

pub enum JobKind {
    /// Perform a shadow copy from the first directory (the source) to
    /// the second directory (the destination)
    ShadowCopy(PathBuf, PathBuf),
}

pub struct Job {
    creation_time: DateTime<Utc>,
    start_time: Option<DateTime<Utc>>,
    finish_time: Option<DateTime<Utc>>,
    kind: JobKind
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::ShadowCopy(src, dest) => write!(f, "Shadow copy from {:?} to {:?}", src, dest)
        }
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

/// A module of constructor functions for creating new jobs.
pub mod new_jobs {
    use std::path::PathBuf;
    use super::{Job, JobKind};
    use chrono::{Utc};

    fn make_job(kind: JobKind) -> Job {
        Job {
            creation_time: Utc::now(),
            start_time: None,
            finish_time: None,
            kind
        }
    }

    /// Create a new shadow copy job.
    pub fn shadow_copy<P>(source: P, destination: P) -> Job
    where P: Into<PathBuf>
    {
        let kind = JobKind::ShadowCopy(source.into(), destination.into());
        make_job(kind)
    }
}
