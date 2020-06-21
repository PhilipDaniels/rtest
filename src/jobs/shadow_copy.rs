use crate::{
    shadow_copy_destination::ShadowCopyDestination,
    jobs::{Job, JobKind},
};
use ignore::WalkBuilder;
use log::info;
use std::fmt::Display;

#[derive(Debug)]
pub struct ShadowCopyJob {
    destination: ShadowCopyDestination,
    num_files_copied: usize,
}

impl Display for ShadowCopyJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Shadow copy from {:?} to {:?}",
            self.destination.source_directory(),
            self.destination.destination_directory()
        )
    }
}

impl ShadowCopyJob {
    /// Create a new shadow copy job to copy from the `source` directory
    /// to the `destination` directory.
    pub fn new(destination_directory: ShadowCopyDestination) -> Job {
        assert!(
            destination_directory.is_copying(),
            "A ShadowCopyJob should not be constructed if we are not actually copying elsewhere"
        );

        let kind = JobKind::ShadowCopy(ShadowCopyJob {
            destination: destination_directory,
            num_files_copied: 0,
        });

        Job::new(kind)
    }

    pub fn execute(&mut self) {
        let walker = WalkBuilder::new(self.destination.source_directory()).build();
        for result in walker {
            match result {
                Ok(entry) => {
                    if !entry.path().is_dir() {
                        self.destination.copy_file(entry.path());
                        self.num_files_copied += 1;
                    }
                }
                Err(err) => println!("ERROR: {}", err),
            }
        }

        info!("{} files copied", self.num_files_copied);
    }
}
