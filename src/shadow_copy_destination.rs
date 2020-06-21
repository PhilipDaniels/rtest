use crate::configuration::DestinationType;
use log::{error, info};
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

    /// Copies a file from the source directory to the destination directory.
    /// This is a no-op if we are not shadow copying at all (i.e. it is safe)
    /// to call if `DestinationKind` was `SourceDirectory`.
    pub fn copy_file(&self, file: &Path) {
        if self.destination_directory.is_none() {
            return;
        }

        let sub_path = self.get_source_sub_path(file);
        let dest_path = self.get_dest_path(sub_path);
        Self::copy_starting_message(file, &dest_path);

        match std::fs::copy(file, &dest_path) {
            Ok(_) => Self::copy_succeeded_message(file, &dest_path),
            Err(_) => {
                // Try again, probably the parent directory did not exist.
                Self::create_parent_dir(&dest_path);
                match std::fs::copy(file, &dest_path) {
                    Ok(_) => Self::copy_succeeded_message(file, &dest_path),
                    Err(err) => Self::copy_error_message(file, &dest_path, &err),
                }
            }
        }
    }

    fn copy_succeeded_message(source: &Path, destination: &Path) {
        info!("Copied   {} to {}", source.display(), destination.display());
    }

    fn copy_starting_message(source: &Path, destination: &Path) {
        info!("Copying  {} to {}", source.display(), destination.display());
    }

    fn copy_error_message(source: &Path, destination: &Path, err: &std::io::Error) {
        error!("COPYFAIL {} to {}, err = {}", source.display(), destination.display(), err);
    }

    /// Calculates the 'sub path' component of a file within the source directory.
    /// This is just the full path with the leading source directory stripped off.
    fn get_source_sub_path<'a>(&self, file: &'a Path) -> &'a Path {
        file.strip_prefix(&self.source_directory).unwrap()
    }

    fn get_dest_path(&self, file: &Path) -> PathBuf {
        let mut dest_path = self.destination_directory.clone()
            .expect("`get_dest_path` should only be called when there actually is a `destination_directory`");
        dest_path.push(file);
        dest_path
    }

    fn create_parent_dir(file: &Path) {
        let parent_dir = file.parent().unwrap();
        std::fs::create_dir_all(parent_dir);
    }
}
