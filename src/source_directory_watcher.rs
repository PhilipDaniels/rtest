use log::info;
use notify::{RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::{
    path::{MAIN_SEPARATOR, PathBuf},
    sync::mpsc::{channel, Sender},
    time::Duration, thread,
};
use watchexec::{Args, Handler};
use watchexec::cli::ArgsBuilder;
/*
cargo-watch
fn main() -> watchexec::error::Result<()> {
    let matches = cargo_watch::args::parse();       // Parse using CLAP
    cargo_watch::change_dir();                              // Find root and change to it
    let quiet = matches.is_present("quiet");
    let debug = matches.is_present("debug");

    let opts = cargo_watch::get_options(debug, &matches);   // Takes CLAP matches and returns **watchexec::cli::Args** (the sub project)

    let handler = cargo_watch::watch::CwHandler::new(opts, quiet)?;         // Makes a **watchexec::run::Handler** implementation
    watchexec::run::watch(&handler);                                                    // Starts it all running
}
*/

pub struct FileEventHandler {
    args: Args
}

impl Handler for FileEventHandler {
    fn on_manual(&self) -> watchexec::error::Result<bool> {
        println!("OnMANUAL");
        Ok(true)
    }

    fn on_update(&self, ops: &[watchexec::pathop::PathOp]) -> watchexec::error::Result<bool> {
        println!("On_Update {:?}", ops);
        Ok(true)
    }

    fn args(&self) -> Args {
        println!("ARGS");
        self.args.clone()
    }
}

impl FileEventHandler {
    fn new(args: Args) -> Self {
        Self {
            args
        }
    }
}

pub fn start_watching<P>(path: P)
where P: Into<PathBuf>
{
    // Note that this list of ignores is a glob list, not a regex-list.
    // Taken from cargo-watch/lib.rs.
    let mut list = vec![
        // Mac
        format!("*{}.DS_Store", MAIN_SEPARATOR),
        // Vim
        "*.sw?".into(),
        "*.sw?x".into(),
        // Emacs
        "#*#".into(),
        ".#*".into(),
        // Kate
        ".*.kate-swp".into(),
        // VCS
        format!("*{s}.hg{s}**", s = MAIN_SEPARATOR),
        format!("*{s}.git{s}**", s = MAIN_SEPARATOR),
        format!("*{s}.svn{s}**", s = MAIN_SEPARATOR),
        // SQLite
        "*.db".into(),
        "*.db-*".into(),
        format!("*{s}*.db-journal{s}**", s = MAIN_SEPARATOR),
        // Rust
        format!("*{s}target{s}**", s = MAIN_SEPARATOR),
    ];

    let mut args = ArgsBuilder::default()
        .cmd(vec!["echo hello world".into()])
        .paths(vec![path.into()])
        .ignores(list)
        .run_initially(false)
        .debounce(2_000u64)
        .build()
        .expect("Construction of Args failed");

    println!("SETTINGS ARGS = {:?}", args);

    let handler = FileEventHandler::new(args);
    watchexec::run::watch(&handler).unwrap();
}

/// Watch a directory (nominally the source directory for the Shadow Copy)
/// and emit file created/deleted/changed events as they happen.
pub struct SourceDirectoryWatcher {
    //directory: PathBuf,
    watcher: RecommendedWatcher,
    //sender: Sender<FileSyncEvent>,
}

enum FileSyncEventKind {
    Create,
    Modify,
    Remove,
}

#[derive(Debug, Clone)]
pub enum FileSyncEvent {
    Create(Vec<PathBuf>),
    Modify(Vec<PathBuf>),
    Remove(Vec<PathBuf>),
}

impl SourceDirectoryWatcher {
    pub fn new<P>(directory: P, sender: Sender<FileSyncEvent>) -> Self
    where
        P: Into<PathBuf>,
    {
        let (tx, rx) = channel();
        let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))
            .expect("Expect to be able to create the SourceDirectoryWatcher");

        let directory = directory.into();
        watcher.watch(&directory, RecursiveMode::Recursive).unwrap();

        let _ = thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(event) => println!("{:?}", event),
                        Err(e) => println!("watch error: {:?}", e),
                    }
                }
            }
        );

        Self {
            //directory,
            watcher,
            //sender,
        }
    }

    // fn watch_event_handler(event_result: Result<Event>) {
    //     if let Err(e) = event_result {
    //         info!("WATCHER  error: {:?}", e);
    //         return;
    //     }

    //     let event = event_result.unwrap();

    //     // Convert the raw event into one of our simpler event types.
    //     // TODO: We only care about files. Check behaviour on directories.
    //     let file_sync_event = match event {
    //         Event {
    //             kind: EventKind::Create(_),
    //             paths,
    //             ..
    //         } => Self::make_event(FileSyncEventKind::Create, paths),

    //         Event {
    //             kind: EventKind::Modify(_),
    //             paths,
    //             ..
    //         } => Self::make_event(FileSyncEventKind::Modify, paths),

    //         Event {
    //             kind: EventKind::Remove(_),
    //             paths,
    //             ..
    //         } => Self::make_event(FileSyncEventKind::Remove, paths),

    //         _ => None,
    //     };

    //     if let Some(event) = file_sync_event {
    //         info!("GOT WATCHER EVENT {:?}", event);
    //     }
    // }

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
