use log::info;
use std::{
    path::{PathBuf, MAIN_SEPARATOR},
    sync::mpsc::Sender,
    thread,
};
use watchexec::cli::ArgsBuilder;
use watchexec::{Args, Handler};

/// Start a 'cargo-watch-like' watch process on `path` (which will be the source directory).
/// The watch ignores everything that `.gitignore` would ignore, so that only changes relating
/// to files we need for compilation should be emitted. Events are emitted on the `sender`
/// channel.
///
/// The watch runs on a separate thread which runs until the end of the program.
/// This implies there is no way to change the source directory after the program
/// has started.
pub fn start_watching<P>(path: P, sender: Sender<FileSyncEvent>)
where
    P: Into<PathBuf>,
{
    let args = get_args(path);
    let handler = FileEventHandler::new(args, sender);

    let thread_builder = thread::Builder::new().name("DirectoryWatcher".into());
    thread_builder.spawn(move || {
        watchexec::run::watch(&handler).unwrap();
    }).expect("Cannot create background thread to run the directory watcher");
    info!("Successfully spawned DirectoryWatcher background thread");
}

/// Constructs the arguments to be passed to the `watchexec` crate.
fn get_args<P>(path: P) -> Args
where
    P: Into<PathBuf>,
{
    // Note that this list of ignores is a glob list, not a regex-list.
    // Taken from cargo-watch/lib.rs and edited a bit.
    let list = vec![
        // GEdit
        ".goutputstream*".into(),
        // -- My extras above.

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

    ArgsBuilder::default()
        .cmd(vec!["".into()]) // Execute nothing, just raise events.
        .paths(vec![path.into()])
        .ignores(list)
        .run_initially(false) // turns off the on_manual event.
        .debounce(1000_u64)
        .build()
        .expect("Construction of Args failed")
}


/// This struct is used to impl the `Handler` trait from `watchexec`.
/// File system events are raised as events on the `sender`.
struct FileEventHandler {
    args: Args,
    sender: Sender<FileSyncEvent>,
}

impl Handler for FileEventHandler {
    fn on_update(&self, ops: &[watchexec::pathop::PathOp]) -> watchexec::error::Result<bool> {
        for op in ops {
            info!("On_Update {:?}", op);
            let event = FileSyncEvent::Create(vec![]);
            self.sender.send(event).unwrap();
        }

        Ok(true)
    }

    fn on_manual(&self) -> watchexec::error::Result<bool> {
        Ok(true)
    }

    fn args(&self) -> Args {
        self.args.clone()
    }
}

impl FileEventHandler {
    fn new(args: Args, sender: Sender<FileSyncEvent>) -> Self {
        Self { args, sender }
    }
}

/*
THIS IS A RENAME FROM KATE
 { path: "/home/phil/repos/rtest/a.txt.h27685", op: Some(RENAME), cookie: Some(274698) }
 { path: "/home/phil/repos/rtest/a.txt", op: Some(RENAME), cookie: Some(274698) }

FOR SOME PATH P
If a build is running, stop it
If OP is REMOVE, remove all file copy jobs and create a remove job
ELSE
    if there is a previous job for this file, remove it and insert a new COPY job (op is likely to be WRITE, CLOSE_WRITE, RENAME or CHMOD)
 */



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

fn make_event(kind: FileSyncEventKind, paths: Vec<PathBuf>) -> Option<FileSyncEvent> {
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
