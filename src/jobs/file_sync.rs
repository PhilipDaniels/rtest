use crate::{
    jobs::{Job, JobKind},
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
            FileSyncEvent::FileUpdate(pb) => ("created/updated file", pb),
            FileSyncEvent::Remove(pb) => ("deleted file or directory", pb),
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
    ) -> Job {
        assert!(
            destination_directory.is_copying(),
            "A FileSyncJob should not be constructed if we are not actually copying elsewhere"
        );

        let kind = JobKind::FileSync(FileSyncJob {
            destination: destination_directory,
            file_sync_event,
        });

        Job::new(kind)
    }

    pub fn execute(&mut self) {
        match &self.file_sync_event {
            FileSyncEvent::FileUpdate(path) => {
                if std::path::Path::is_file(path) {
                    self.destination.copy_file(path);
                }
            }
            FileSyncEvent::Remove(path) => {
                self.destination.remove_file_or_directory(path);
            }
        }
    }
}
