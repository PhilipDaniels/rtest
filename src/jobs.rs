mod build_tests;
mod file_sync;
mod run_test;
mod shadow_copy;

pub use build_tests::BuildTestsJob;
pub use file_sync::FileSyncJob;
pub use run_test::TestJob;
pub use shadow_copy::ShadowCopyJob;

use chrono::{DateTime, Utc};
use logging_timer::{finish, stimer, Level};
use std::{
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

/// The build mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

pub trait Job: Display {
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
        let tmr = stimer!(Level::Info; "execute()", "{}", self.id);
        let mut executing_job: ExecutingJob = self.into();
        let status = executing_job.execute();
        finish!(tmr, "completed with status={:?}", status);
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

impl ExecutingJob {
    fn execute(&mut self) -> CompletionStatus {
        let status = self.kind.execute(self.id().clone());
        status
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

    pub fn completion_status(&self) -> CompletionStatus {
        self.status.clone()
    }

    pub fn succeeded(&self) -> bool {
        self.status == CompletionStatus::Ok
    }
}

/// Specifies the completion status of a Job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionStatus {
    Unknown,
    Ok,
    Error(String),
}

impl<S: Into<String>> From<S> for CompletionStatus {
    fn from(msg: S) -> Self {
        Self::Error(msg.into())
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
    Build(BuildTestsJob),

    Test(TestJob),
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.fmt(f),
            JobKind::FileSync(file_sync_job) => file_sync_job.fmt(f),
            JobKind::Build(build_job) => build_job.fmt(f),
            JobKind::Test(test_job) => test_job.fmt(f),
        }
    }
}

impl JobKind {
    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    fn execute(&mut self, parent: JobId) -> CompletionStatus {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.execute(),
            JobKind::FileSync(file_sync_job) => file_sync_job.execute(),
            JobKind::Build(build_job) => build_job.execute(parent),
            JobKind::Test(test_job) => test_job.execute(parent),
        }
    }
}

/// Every Job has a unique id.
/// Note that cloning theoretically creates a duplicate Id. In practice, this only happens
/// inside the engine when it is executing the job.
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
