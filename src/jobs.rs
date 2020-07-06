mod build;
mod file_sync;
mod process;
mod shadow_copy;

pub use build::{BuildJob, BuildMode};
pub use file_sync::FileSyncJob;
pub use shadow_copy::ShadowCopyJob;

use chrono::{DateTime, Utc};
use log::{warn, info};
use logging_timer::stime;
use std::{
    fmt::{self, Display},
    sync::atomic::{AtomicUsize, Ordering},
};

pub trait JobTrait {
    fn succeeded(&self) -> bool;
    fn kind(&self) -> &JobKind;
}

#[derive(Debug, Clone)]
pub enum CompletionStatus {
    Ok,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Pending {
    creation_date: DateTime<Utc>,
}

impl Pending {
    pub fn new() -> Self {
        Self {
            creation_date: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Executing {
    creation_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Completed {
    creation_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
    completed_date: DateTime<Utc>,
    status: CompletionStatus,
}

#[derive(Debug, Clone)]
pub enum JobStatus {
    Pending(Pending),
    Executing(Executing),
    Completed(Completed),
}

impl JobStatus {
    fn begin_execution(&mut self) {
        match self {
            Self::Pending(pending) => {
                let executing = Executing {
                    creation_date: pending.creation_date,
                    start_date: Utc::now(),
                };
                *self = Self::Executing(executing);
            }
            _ => panic!("Bad state"),
        }
    }

    fn complete_execution(&mut self, status: CompletionStatus) {
        match self {
            Self::Executing(executing) => {
                let completed = Completed {
                    creation_date: executing.creation_date,
                    start_date: executing.start_date,
                    completed_date: Utc::now(),
                    status,
                };
                *self = Self::Completed(completed);
            }
            _ => panic!("Bad state"),
        }
    }
}

/// The `JobKind` specifies what type of job it is and the supporting data needed for that job.
#[derive(Debug, Clone)]
pub enum JobKind {
    /// Perform a shadow copy from the first directory (the source) to
    /// the second directory (the destination)
    ShadowCopy(ShadowCopyJob),
    /// Perform a file sync (copy or delete) of a file.
    FileSync(FileSyncJob),
    /// Perform a build of the destination directory.
    Build(BuildJob),
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.fmt(f),
            JobKind::FileSync(file_sync_job) => file_sync_job.fmt(f),
            JobKind::Build(build_job) => build_job.fmt(f),
        }
    }
}

impl JobKind {
    fn execute(&mut self) {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.execute(),
            JobKind::FileSync(file_sync_job) => file_sync_job.execute(),
            JobKind::Build(build_job) => build_job.execute(),
        }
    }
}

/// Every Job has a unique id.
/// Note that cloning theoretically creates a duplicate Id. In practice, this is only done
/// inside the engine when it is executing the job so it's safe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobId {
    id: usize,
}

impl Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Job #{}", self.id)
    }
}

impl JobId {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(1);

        Self {
            id: ID.fetch_add(1, Ordering::SeqCst),
        }
    }
}

/// A Job is a short-lived unit of work that can be executed by the `JobEngine`.
/// n.b. All jobs have a unique identifier, which means they cannot be copied or cloned.
#[derive(Debug, Clone)]
pub struct Job {
    id: JobId,
    status: JobStatus,
    kind: JobKind,
}

impl Job {
    pub fn new(kind: JobKind) -> Self {
        let pending = Pending::new();
        let status = JobStatus::Pending(pending);

        Self {
            id: JobId::new(),
            status,
            kind,
        }
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }

    /// Returns true if this is a pending job.
    pub fn is_pending(&self) -> bool {
        match self.status {
            JobStatus::Pending(_) => true,
            _ => false,
        }
    }

    pub fn kind(&self) -> &JobKind {
        &self.kind
    }

    #[stime]
    pub fn execute(&mut self) {
        info!("Executing {}", self);
        //self.status.begin_execution();
        self.kind.execute();
        self.status.complete_execution(CompletionStatus::Ok);
    }

    pub fn begin_execution(&mut self) {
        self.status.begin_execution();
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.kind)
    }
}

fn job_succeeded(log_entry: fmt::Arguments) {
    info!("JOB SUCCEEDED: {}", log_entry);
}

fn job_failed(log_entry: fmt::Arguments) {
    warn!("JOB FAILED: {}", log_entry);
}

#[macro_export]
macro_rules! succeeded {
   ($format:tt, $($arg:expr),*) => (
       crate::jobs::job_succeeded(format_args!($format, $($arg),*))
   )
}

#[macro_export]
macro_rules! failed {
   ($format:tt, $($arg:expr),*) => (
       crate::jobs::job_failed(format_args!($format, $($arg),*))
   )
}
