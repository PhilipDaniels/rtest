mod build_all_tests;
mod build_workspace;
mod file_sync;
mod list_all_tests;
mod run_tests;
mod shadow_copy;

pub use build_all_tests::BuildAllTestsJob;
pub use build_workspace::BuildWorkspaceJob;
pub use file_sync::FileSyncJob;
pub use list_all_tests::ListAllTestsJob;
pub use run_tests::RunTestsJob;
pub use shadow_copy::ShadowCopyJob;

use chrono::{DateTime, Utc};
use log::{info, warn};
use logging_timer::{finish, stimer, Level};
use std::{
    fmt::Display,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

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

        let executing_job: ExecutingJob = self.into();
        let completed_job = executing_job.execute();

        finish!(tmr, "completed with status={:?}", completed_job.status);
        completed_job
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
    fn execute(mut self) -> CompletedJob {
        // Execute the job-specific data.
        let status = self.kind.execute(self.id().clone());
        CompletedJob::new(self, status)
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

    /// Perform a build **tests only**.
    BuildAllTests(BuildAllTestsJob),

    /// Perform a build **of the crate**.
    BuildWorkspace(BuildWorkspaceJob),

    /// List all tests.
    ListAllTests(ListAllTestsJob),

    RunTests(RunTestsJob),
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.fmt(f),
            JobKind::FileSync(file_sync_job) => file_sync_job.fmt(f),
            JobKind::BuildWorkspace(build_crate_job) => build_crate_job.fmt(f),
            JobKind::BuildAllTests(build_tests_job) => build_tests_job.fmt(f),
            JobKind::ListAllTests(list_tests_job) => list_tests_job.fmt(f),
            JobKind::RunTests(run_tests_job) => run_tests_job.fmt(f),
        }
    }
}

impl JobKind {
    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.execute(),
            JobKind::FileSync(file_sync_job) => file_sync_job.execute(),
            JobKind::BuildWorkspace(build_crate_job) => build_crate_job.execute(parent_job_id),
            JobKind::BuildAllTests(build_tests_job) => build_tests_job.execute(parent_job_id),
            JobKind::ListAllTests(list_tests_job) => list_tests_job.execute(parent_job_id),
            JobKind::RunTests(run_tests_job) => run_tests_job.execute(parent_job_id),
        }
    }
}

/// Every Job has a unique id.
/// Note that cloning theoretically creates a duplicate Id. In reality, this only happens
/// inside the engine when it is executing the job and when we are passing them down
/// the call stack so they can be printed out. It's not a problem in practice.
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

/// The output data of a process, converted to a slightly
/// friendlier string form.
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    exit_status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

impl From<std::process::Output> for ProcessOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            exit_status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).into(),
            stderr: String::from_utf8_lossy(&output.stderr).into(),
        }
    }
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        self.exit_status.success()
    }
}

fn gather_process_output(
    mut command: Command,
    description: &str,
    parent_job_id: JobId,
) -> Result<ProcessOutput, String> {
    let output: ProcessOutput = match command.output() {
        Ok(result) => result.into(),
        Err(e) => return Err(format!("{} process start failed, err={}", description, e)),
    };

    let msg = format!(
        "{} {} {}. ExitStatus={:?}, stdout={} bytes, stderr={} bytes",
        parent_job_id,
        description,
        if output.exit_status.success() {
            "succeeded"
        } else {
            "failed"
        },
        output.exit_status,
        output.stdout.len(),
        output.stderr.len()
    );

    if output.success() {
        info!("{}", msg);
        Ok(output)
    } else {
        warn!("{}", msg);
        Err(msg)
    }
}

/// Gathers the stdout of a duct command. If you want to gather both
/// the stdout and the stderr, call `stderr_to_stdout` on your command
/// before calling this function.
fn gather_process_stdout(
    cmd: duct::Expression,
    description: &str,
    parent_job_id: JobId,
) -> std::io::Result<String> {
    let output = cmd.read()?;

    info!(
        "{} {} succeeded, stdout={} bytes",
        parent_job_id,
        description,
        output.len()
    );

    Ok(output)
}
