use std::path::PathBuf;

/// Watch a directory (nominally the source directory for the Shadow Copy)
/// and emit file created/deleted/changed events as they happen.
pub struct Watcher {
    directory: PathBuf,
}

impl Watcher {
    pub fn new<P>(directory: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            directory: directory.into(),
        }
    }
}
