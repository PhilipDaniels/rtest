mod build;
mod file_sync;
mod shadow_copy;

pub use build::{BuildJob, BuildMode};
pub use file_sync::FileSyncJob;
pub use shadow_copy::ShadowCopyJob;

use chrono::{DateTime, Utc};
use log::{info, warn};
use logging_timer::{stimer, stime, Level};
use std::{
    fmt::{self, Display},
    sync::atomic::{AtomicUsize, Ordering},
};

pub trait Job : Display {
    fn id(&self) -> &JobId;
    fn kind(&self) -> &JobKind;
}

#[derive(Debug, Clone)]
pub struct PendingJob {
    id: JobId,
    kind: JobKind,
    creation_date: DateTime<Utc>,
}

impl Display for PendingJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.kind)
    }
}

impl From<JobKind> for PendingJob {
    fn from(kind: JobKind) -> Self {
        Self {
            id: JobId::new(),
            kind,
            creation_date: Utc::now(),
        }
    }
}

impl Job for PendingJob {
    fn id(&self) -> &JobId {
        &self.id
    }

    fn kind(&self) -> &JobKind {
        &self.kind
    }
}

impl PendingJob {
    pub fn execute(self) -> CompletedJob {
        let tmr = stimer!(Level::Info; "execute");
        let executing_job: ExecutingJob = self.into();
        let status = CompletionStatus::Ok;
        CompletedJob::new(executing_job, status)
    }
}

#[derive(Debug, Clone)]
pub struct ExecutingJob {
    id: JobId,
    kind: JobKind,
    creation_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
}

impl Display for ExecutingJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.kind)
    }
}

impl From<PendingJob> for ExecutingJob {
    fn from(pending_job: PendingJob) -> Self {
        Self {
            id: pending_job.id,
            kind: pending_job.kind,
            creation_date: pending_job.creation_date,
            start_date: Utc::now(),
        }
    }
}

impl Job for ExecutingJob {
    fn id(&self) -> &JobId {
        &self.id
    }

    fn kind(&self) -> &JobKind {
        &self.kind
    }
}

#[derive(Debug, Clone)]
pub struct CompletedJob {
    id: JobId,
    kind: JobKind,
    creation_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
    completed_date: DateTime<Utc>,
    status: CompletionStatus,
}

impl Job for CompletedJob {
    fn id(&self) -> &JobId {
        &self.id
    }

    fn kind(&self) -> &JobKind {
        &self.kind
    }
}

impl Display for CompletedJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.kind)
    }
}

impl CompletedJob {
    pub fn new(executing_job: ExecutingJob, status: CompletionStatus) -> Self {
        Self {
            id: executing_job.id,
            kind: executing_job.kind,
            creation_date: executing_job.creation_date,
            start_date: executing_job.start_date,
            completed_date: Utc::now(),
            status,
        }
    }

    pub fn succeeded(&self) -> bool {
        todo!()
    }
}

/// Specifies the completion status of a Job.
#[derive(Debug, Clone)]
pub enum CompletionStatus {
    Ok,
    Error(String),
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






// A Job is a short-lived unit of work that can be executed by the `JobEngine`.
// n.b. All jobs have a unique identifier, which means they cannot be copied or cloned.
// #[derive(Debug, Clone)]
// pub struct Job {
//     id: JobId,
//     status: JobStatus,
//     kind: JobKind,
// }

// impl Job {
//     pub fn new(kind: JobKind) -> Self {
//         let pending = Pending::new();
//         let status = JobStatus::Pending(pending);

//         Self {
//             id: JobId::new(),
//             status,
//             kind,
//         }
//     }

//     pub fn id(&self) -> &JobId {
//         &self.id
//     }

//     /// Returns true if this is a pending job.
//     pub fn is_pending(&self) -> bool {
//         match self.status {
//             JobStatus::Pending(_) => true,
//             _ => false,
//         }
//     }

//     pub fn kind(&self) -> &JobKind {
//         &self.kind
//     }

//     #[stime]
//     pub fn execute(&mut self) {
//         info!("Executing {}", self);
//         //self.status.begin_execution();
//         self.kind.execute();
//         self.status.complete_execution(CompletionStatus::Ok);
//     }

//     pub fn begin_execution(&mut self) {
//         self.status.begin_execution();
//     }
// }

// impl Display for Job {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{} {}", self.id, self.kind)
//     }
// }















// TBD


// #[derive(Debug, Clone)]
// pub struct Pending {
//     creation_date: DateTime<Utc>,
// }

// impl Pending {
//     pub fn new() -> Self {
//         Self {
//             creation_date: Utc::now(),
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct Executing {
//     creation_date: DateTime<Utc>,
//     start_date: DateTime<Utc>,
// }

// impl Executing {
//     fn new(pending: &Pending) -> Self {
//         Self {
//             creation_date: pending.creation_date,
//             start_date: Utc::now(),
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct Completed {
//     creation_date: DateTime<Utc>,
//     start_date: DateTime<Utc>,
//     completed_date: DateTime<Utc>,
//     status: CompletionStatus,
// }

// impl Completed {
//     fn new(executing: &Executing, status: CompletionStatus) -> Self {
//         Self {
//             creation_date: executing.creation_date,
//             start_date: executing.start_date,
//             completed_date: Utc::now(),
//             status,
//         }
//     }
// }
// #[derive(Debug, Clone)]
// pub enum JobStatus {
//     Pending(Pending),
//     Executing(Executing),
//     Completed(Completed),
// }

// impl JobStatus {
//     fn begin_execution(&mut self) {
//         match self {
//             Self::Pending(pending) => {
//                 *self = Self::Executing(Executing::new(pending));
//             }
//             _ => panic!("Bad state"),
//         }
//     }

//     fn complete_execution(&mut self, status: CompletionStatus) {
//         match self {
//             Self::Executing(executing) => {
//                 let completed = Completed::new(executing, status);
//                 *self = Self::Completed(completed);
//             }
//             _ => panic!("Bad state"),
//         }
//     }
// }

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
