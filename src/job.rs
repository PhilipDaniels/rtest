use chrono::{DateTime, Utc};
use ignore::WalkBuilder;
use log::info;
use logging_timer::stime;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

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
        self.kind.execute();
        // TODO When the job is complete we need to send a message
        // so that the UI can be updated.
        self.end_time = Some(Utc::now());
    }
}

impl JobKind {
    fn execute(&mut self) {
        match &*self {
            JobKind::ShadowCopy(src, dest) => execute_shadow_copy(src, dest),
        }
    }
}

fn execute_shadow_copy(src: &Path, dest: &Path) {
    let mut num_files = 0;
    let walker = WalkBuilder::new(src).build();
    for result in walker {
        match result {
            Ok(entry) => {
                if entry.path().is_dir() {
                    continue;
                }

                let sub_path = entry.path().strip_prefix(src).unwrap();
                let mut dest_path = dest.to_path_buf();
                dest_path.push(sub_path);

                info!(
                    "Copying {} to {}",
                    entry.path().display(),
                    dest_path.display()
                );

                let dest_sub_dir = dest_path.parent().unwrap();
                std::fs::create_dir_all(dest_sub_dir).unwrap();
                std::fs::copy(entry.path(), dest_path).unwrap();

                num_files += 1;
            }
            Err(err) => println!("ERROR: {}", err),
        }
    }

    info!("{} files copied", num_files);
}
