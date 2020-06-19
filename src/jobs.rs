pub mod shadow_copy;

use chrono::{DateTime, Utc};
use log::info;
use logging_timer::stime;
use shadow_copy::ShadowCopyJob;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

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

pub enum JobKind {
    /// Perform a shadow copy from the first directory (the source) to
    /// the second directory (the destination)
    ShadowCopy(ShadowCopyJob),
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.fmt(f),
        }
    }
}

impl JobKind {
    fn execute(&mut self) {
        match self {
            JobKind::ShadowCopy(shadow_copy_job) => shadow_copy_job.execute(),
        }
    }
}

#[derive(Debug)]
pub struct JobId {
    id: usize,
}

impl JobId {
    fn new() -> Self {
        Self {
            id: 0
        }
    }
}

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

    #[stime]
    pub fn execute(&mut self) {
        info!("Executing job: {}", self);
        self.status.begin_execution();
        self.kind.execute();
        self.status.complete_execution(CompletionStatus::Ok);
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}
