use crate::{
    jobs::{Job, JobKind},
    shadow_copy_destination::ShadowCopyDestination,
};
use ignore::WalkBuilder;
use log::info;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct ShadowCopyJob {
    destination: ShadowCopyDestination,
    num_files_copied: usize,
    succeeded: bool,
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
            succeeded: false,
        });

        Job::new(kind)
    }

    pub fn succeeded(&self) -> bool {
        self.succeeded
    }

    pub fn execute(&mut self) {
        let src = self.destination.source_directory();
        if !std::path::Path::is_dir(src) {
            self.succeeded = false;
            return;
        }

        let walker = WalkBuilder::new(src).build();
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

        // Even if 1 or more copies fail, we can still consider outself
        // to have succeeded.
        self.succeeded = true;

        info!("{} files copied", self.num_files_copied);
    }
}
