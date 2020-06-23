use log::info;
use notify::{event::Event, EventKind, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::path::PathBuf;

/// Watch a directory (nominally the source directory for the Shadow Copy)
/// and emit file created/deleted/changed events as they happen.
pub struct SourceDirectoryWatcher {
    directory: PathBuf,
    watcher: RecommendedWatcher,
}

enum FileSyncEventKind {
    Create,
    Modify,
    Remove,
}

#[derive(Debug, Clone)]
enum FileSyncEvent {
    Create(Vec<PathBuf>),
    Modify(Vec<PathBuf>),
    Remove(Vec<PathBuf>),
}

impl SourceDirectoryWatcher {
    pub fn new<P>(directory: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let mut watcher: RecommendedWatcher =
            Watcher::new_immediate(Self::watch_event_handler).unwrap();

        let directory = directory.into();
        watcher.watch(&directory, RecursiveMode::Recursive).unwrap();

        let sdw = Self { directory, watcher };

        sdw
    }

    fn watch_event_handler(event_result: Result<Event>) {
        if let Err(e) = event_result {
            info!("WATCHER  error: {:?}", e);
            return;
        }

        let event = event_result.unwrap();

        // Convert the raw event into one of our simpler event types.
        // TODO: We only care about files. Check behaviour on directories.
        let file_sync_event = match event {
            Event {
                kind: EventKind::Create(_),
                paths,
                ..
            } => Self::make_event(FileSyncEventKind::Create, paths),

            Event {
                kind: EventKind::Modify(_),
                paths,
                ..
            } => Self::make_event(FileSyncEventKind::Modify, paths),

            Event {
                kind: EventKind::Remove(_),
                paths,
                ..
            } => Self::make_event(FileSyncEventKind::Remove, paths),

            _ => None,
        };

        if let Some(event) = file_sync_event {
            info!("GOT WATCHER EVENT {:?}", event);
        }
    }

    fn make_event(kind: FileSyncEventKind, paths: Vec<PathBuf>) -> Option<FileSyncEvent> {
        let paths = filter_out_ignored_files(paths);
        if paths.is_empty() {
            None
        } else {
            Some(match kind {
                FileSyncEventKind::Create => FileSyncEvent::Create(paths),
                FileSyncEventKind::Modify => FileSyncEvent::Modify(paths),
                FileSyncEventKind::Remove => FileSyncEvent::Remove(paths),
            })
        }
    }
}

fn filter_out_ignored_files(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
}
