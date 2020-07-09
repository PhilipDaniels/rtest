use super::CompletionStatus;
use crate::{
    jobs::{JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
    source_directory_watcher::FileSyncEvent,
};
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct FileSyncJob {
    destination: ShadowCopyDestination,
    file_sync_event: FileSyncEvent,
}

impl Display for FileSyncJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (s, pathbuf) = match &self.file_sync_event {
            FileSyncEvent::FileUpdate(path_buf) => ("created/updated file", path_buf),
            FileSyncEvent::Remove(path_buf) => ("deleted file or directory", path_buf),
        };

        write!(f, "FileSync - {} {:?}", s, pathbuf)
    }
}

impl FileSyncJob {
    /// Create a new shadow copy job to copy from the `source` directory
    /// to the `destination` directory.
    pub fn new(
        destination_directory: ShadowCopyDestination,
        file_sync_event: FileSyncEvent,
    ) -> PendingJob {
        assert!(
            destination_directory.is_copying(),
            "A FileSyncJob should not be constructed if we are not actually copying elsewhere"
        );

        let kind = JobKind::FileSync(FileSyncJob {
            destination: destination_directory,
            file_sync_event,
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self) -> CompletionStatus {
        match &self.file_sync_event {
            FileSyncEvent::FileUpdate(path) => {
                if std::path::Path::is_file(path) {
                    if self.destination.copy_file(path) {
                        CompletionStatus::Ok
                    } else {
                        format!("Copying file {:?} failed", path).into()
                    }
                } else {
                    format!("The path {:?} is not a file", path).into()
                }
            }

            FileSyncEvent::Remove(path) => {
                if self.destination.remove_file_or_directory(path) {
                    CompletionStatus::Ok
                } else {
                    format!("Removing path {:?} failed", path).into()
                }
            }
        }
    }
}
