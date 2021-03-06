use crate::{
    jobs::{CompletionStatus, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use ignore::WalkBuilder;
use log::info;
use std::fmt::Display;

#[derive(Debug, Clone)]
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
            self.destination
                .destination_directory()
                .expect("Should always be Some because of `new` function")
        )
    }
}

impl ShadowCopyJob {
    /// Create a new shadow copy job to copy from the `source` directory
    /// to the `destination` directory.
    pub fn new(destination_directory: ShadowCopyDestination) -> PendingJob {
        assert!(
            destination_directory.is_copying(),
            "A ShadowCopyJob should not be constructed if we are not actually copying elsewhere"
        );

        let kind = JobKind::ShadowCopy(ShadowCopyJob {
            destination: destination_directory,
            num_files_copied: 0,
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self) -> CompletionStatus {
        let src = self.destination.source_directory();
        if !std::path::Path::is_dir(src) {
            return format!("Source directory {:?} is not a directory", src).into();
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
        info!("{} files copied", self.num_files_copied);
        CompletionStatus::Ok
    }
}
