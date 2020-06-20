use crate::jobs::{Job, JobKind};
use ignore::WalkBuilder;
use log::info;
use std::{fmt::Display, path::PathBuf};

#[derive(Debug)]
pub struct ShadowCopyJob {
    source: PathBuf,
    destination: PathBuf,
    num_files_copied: usize,
}

impl Display for ShadowCopyJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Shadow copy from {:?} to {:?}",
            self.source, self.destination
        )
    }
}

impl ShadowCopyJob {
    // pub fn source(&self) -> &Path {
    //     &self.source
    // }

    // pub fn destination(&self) -> &Path {
    //     &self.destination
    // }

    /// Create a new shadow copy job to copy from the `source` directory
    /// to the `destination` directory.
    pub fn new<P>(source: P, destination: P) -> Job
    where
        P: Into<PathBuf>,
    {
        let details = ShadowCopyJob {
            source: source.into(),
            destination: destination.into(),
            num_files_copied: 0,
        };

        let kind = JobKind::ShadowCopy(details);
        Job::new(kind)
    }

    pub fn execute(&mut self) {
        let walker = WalkBuilder::new(&self.source).build();
        for result in walker {
            match result {
                Ok(entry) => {
                    if entry.path().is_dir() {
                        continue;
                    }

                    let sub_path = entry.path().strip_prefix(&self.source).unwrap();
                    let mut dest_path = self.destination.clone();
                    dest_path.push(sub_path);

                    info!(
                        "Copying {} to {}",
                        entry.path().display(),
                        dest_path.display()
                    );

                    let dest_sub_dir = dest_path.parent().unwrap();
                    std::fs::create_dir_all(dest_sub_dir).unwrap();
                    std::fs::copy(entry.path(), dest_path).unwrap();

                    self.num_files_copied += 1;
                }
                Err(err) => println!("ERROR: {}", err),
            }
        }

        info!("{} files copied", self.num_files_copied);
    }
}
