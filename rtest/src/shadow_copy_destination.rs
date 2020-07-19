use log::{error, info};
use remove_dir_all::remove_dir_all;
use std::path::{Path, PathBuf};

/// Represents the destination directory for the shadow-copy operation.
/// If `UseSourceDirectory`, then no shadow copying is performed and
/// all operations are performed in the original (source) directory.
#[derive(Debug, Clone)]
pub enum DestinationDirectory {
    SameAsSource,
    NamedDirectory(PathBuf),
}

impl DestinationDirectory {
    /// Returns `true` if shadow-copy operations are actually being peformed.
    /// Alternatively, if we are doing everything in the source without shadow
    /// copying, then `false` is returned.
    pub fn is_copying(&self) -> bool {
        match self {
            DestinationDirectory::SameAsSource => false,
            DestinationDirectory::NamedDirectory(_) => true,
        }
    }
}

/// The directory where we (possibly) make the shadow copy and do the
/// compilations and test runs.
#[derive(Debug, Clone)]
pub struct ShadowCopyDestination {
    source_directory: PathBuf,
    destination: DestinationDirectory,
}

impl ShadowCopyDestination {
    pub fn new(source_directory: PathBuf, destination: Option<PathBuf>) -> Self
    {
        Self {
            source_directory,
            destination: match destination {
                Some(dir) => DestinationDirectory::NamedDirectory(dir.into()),
                None => DestinationDirectory::SameAsSource,
            }
        }
    }

    pub fn is_copying(&self) -> bool {
        self.destination.is_copying()
    }

    pub fn source_directory(&self) -> &Path {
        &self.source_directory
    }

    /// Returns the destination directory we are copying to.
    /// Returns `None` in the case that we are not actually doing any copying.
    pub fn destination_directory(&self) -> Option<&Path> {
        match &self.destination {
            DestinationDirectory::SameAsSource => None,
            DestinationDirectory::NamedDirectory(dir) => Some(dir),
        }
    }

    /// Copies a `source_file` from the source directory to the destination directory.
    /// This is a no-op if we are not actually shadow copying.
    pub fn copy_file(&self, source_file: &Path) -> bool {
        if !self.is_copying() {
            return false;
        }

        let dest_file_path = self.get_path_in_destination(source_file);

        match std::fs::copy(source_file, &dest_file_path) {
            Ok(_) => {
                Self::copy_succeeded_message(source_file, &dest_file_path);
                return true;
            }
            Err(_) => {
                // Try again, probably the parent directory did not exist.
                Self::create_destination_parent_dir_for_file(&dest_file_path);
                match std::fs::copy(source_file, &dest_file_path) {
                    Ok(_) => {
                        Self::copy_succeeded_message(source_file, &dest_file_path);
                        return true;
                    }
                    Err(err) => {
                        Self::copy_error_message(source_file, &dest_file_path, &err);
                        return false;
                    }
                }
            }
        }
    }

    /// Given a `source_path`, removes the corresponding file or directory in the destination.
    /// This is a no-op if we are not actually shadow copying.
    pub fn remove_file_or_directory(&self, source_path: &Path) -> bool {
        if !self.is_copying() {
            return false;
        }

        let dest_path = self.get_path_in_destination(source_path);

        if Path::is_dir(&dest_path) {
            match remove_dir_all(&dest_path) {
                Ok(_) => {
                    info!("Removed destination directory {}", dest_path.display());
                    return true;
                }
                Err(err) => {
                    error!(
                        "Error removing destination directory {}, err = {}",
                        dest_path.display(),
                        err
                    );
                    return false;
                }
            }
        } else {
            match std::fs::remove_file(&dest_path) {
                Ok(_) => {
                    Self::remove_succeeded_message(&dest_path);
                    return true;
                }
                Err(err) => {
                    Self::remove_failed_message(&dest_path, &err);
                    return false;
                }
            }
        }
    }

    /// Converts a source path into a corresponding path in the destination directory.
    fn get_path_in_destination(&self, source_path: &Path) -> PathBuf {
        let source_sub_path = self.get_source_sub_path(source_path);
        let mut dest_path = self.destination_directory()
                .expect("`get_path_in_destination` should only be called when there actually is a `destination_directory`")
                .to_owned();

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
}
