use chrono::{DateTime, Utc};
use log::info;
use logging_timer::stime;
use std::{fmt::Display, path::PathBuf};

pub enum JobKind {
    /// Perform a shadow copy from the first directory (the source) to
    /// the second directory (the destination)
    ShadowCopy(PathBuf, PathBuf),
}

pub struct Job {
    creation_time: DateTime<Utc>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    kind: JobKind,
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::ShadowCopy(src, dest) => write!(f, "Shadow copy from {:?} to {:?}", src, dest),
        }
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl Job {
    pub fn new(kind: JobKind) -> Self {
        Job {
            creation_time: Utc::now(),
            start_time: None,
            end_time: None,
            kind,
        }
    }

    #[stime]
    pub fn execute(&mut self) {
        self.start_time = Some(Utc::now());
        info!("Executing job: {}", self);
        // When the job is complete we need to send a message
        // so that the UI can be updated.
        self.end_time = Some(Utc::now());
    }
}
