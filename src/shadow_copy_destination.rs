use crate::configuration::DestinationType;
use log::{error, info};
use remove_dir_all::remove_dir_all;
use std::path::{Path, PathBuf};

/// The temporary directory where we make the shadow copy and do the
/// compilations and test runs.
#[derive(Debug, Clone)]
pub struct ShadowCopyDestination {
    source_directory: PathBuf,
    destination_directory: Option<PathBuf>,
}

impl ShadowCopyDestination {
    pub fn new<P>(source_directory: P, destination_type: &DestinationType) -> Self
    where
        P: Into<PathBuf>,
    {
        match destination_type {
            DestinationType::SourceDirectory(_) => Self {
                source_directory: source_directory.into(),
                destination_directory: None,
            },
            DestinationType::NamedDirectory(dest_dir) => Self {
                source_directory: source_directory.into(),
                destination_directory: Some(dest_dir.into()),
            },
            DestinationType::TempDirectory(tempdir) => Self {
                source_directory: source_directory.into(),
                destination_directory: Some(tempdir.path().into()),
            },
        }
    }

    /// Returns true if we are not actually copying files into a destination directory.
    pub fn is_copying(&self) -> bool {
        self.destination_directory.is_some()
    }

    pub fn source_directory(&self) -> &Path {
        &self.source_directory
    }

    pub fn destination_directory(&self) -> Option<&PathBuf> {
        self.destination_directory.as_ref()
    }

    /// Copies a `source_file` from the source directory to the destination directory.
    pub fn copy_file(&self, source_file: &Path) {
        if self.destination_directory.is_none() {
            return;
        }

        let dest_file_path = self.get_path_in_destination(source_file);

        match std::fs::copy(source_file, &dest_file_path) {
            Ok(_) => Self::copy_succeeded_message(source_file, &dest_file_path),
            Err(_) => {
                // Try again, probably the parent directory did not exist.
                Self::create_destination_parent_dir_for_file(&dest_file_path);
                match std::fs::copy(source_file, &dest_file_path) {
                    Ok(_) => Self::copy_succeeded_message(source_file, &dest_file_path),
                    Err(err) => Self::copy_error_message(source_file, &dest_file_path, &err),
                }
            }
        }
    }

    /// Given a `source_file`, removes the corresponding file in the destination.
    pub fn remove_file_or_directory(&self, source_path: &Path) {
        if self.destination_directory.is_none() {
            return;
        }

        let dest_path = self.get_path_in_destination(source_path);

        if std::path::Path::is_dir(&dest_path) {
            match remove_dir_all(&dest_path) {
                Ok(_) => info!("Removed destination directory {}", dest_path.display()),
                Err(err) => error!(
                    "Error removing destination directory {}, err = {}",
                    dest_path.display(),
                    err
                ),
            }
        } else {
            match std::fs::remove_file(&dest_path) {
                Ok(_) => Self::remove_succeeded_message(&dest_path),
                Err(err) => Self::remove_failed_message(&dest_path, &err),
            }
        }
    }

    /// Converts a source path into a corresponding path in the destination directory.
    fn get_path_in_destination(&self, source_path: &Path) -> PathBuf {
        let source_sub_path = self.get_source_sub_path(source_path);
        let mut dest_path = self.destination_directory.clone()
            .expect("`get_path_in_destination` should only be called when there actually is a `destination_directory`");
        dest_path.push(source_sub_path);
        dest_path
    }

    fn copy_succeeded_message(source: &Path, destination: &Path) {
        info!("Copied {} to {}", source.display(), destination.display());
    }

    fn remove_succeeded_message(destination: &Path) {
        info!("Removed {}", destination.display());
    }

    fn remove_failed_message(destination: &Path, err: &std::io::Error) {
        info!("REMOVEFAIL {}, err = {}", destination.display(), err);
    }

    fn copy_starting_message(source: &Path, destination: &Path) {
        info!("Copying {} to {}", source.display(), destination.display());
    }

    fn copy_error_message(source: &Path, destination: &Path, err: &std::io::Error) {
        error!(
            "COPYFAIL {} to {}, err = {}",
            source.display(),
            destination.display(),
            err
        );
    }

    /// Calculates the 'sub path' component of a file within the source directory.
    /// This is just the full path with the leading source directory stripped off.
    fn get_source_sub_path<'a>(&self, file: &'a Path) -> &'a Path {
        file.strip_prefix(&self.source_directory).unwrap()
    }

    fn create_destination_parent_dir_for_file(destination_file: &Path) {
        let parent_dir = destination_file.parent().unwrap();
        Self::create_destination_dir(&parent_dir);
    }

    fn create_destination_dir(destination_directory: &Path) {
        match std::fs::create_dir_all(destination_directory) {
            Ok(_) => info!(
                "Created destination directory {}",
                destination_directory.display()
            ),
            Err(err) => error!(
                "Error creating destination directory {}, err = {}",
                destination_directory.display(),
                err
            ),
        }
    }

    /// Given a `source_directory`, creates the corresponding directory
    /// in the destination.
    pub fn create_directory(&self, source_directory: &Path) {
        if self.destination_directory.is_none() {
            return;
        }

        let dest_dir = self.get_path_in_destination(source_directory);
        Self::create_destination_dir(&dest_dir);
    }
}
