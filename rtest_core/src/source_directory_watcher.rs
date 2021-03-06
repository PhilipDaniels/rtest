use log::info;
use std::{
    collections::{hash_map::Entry, HashMap},
    path::{PathBuf, MAIN_SEPARATOR},
    sync::mpsc::Sender,
    thread,
};
use watchexec::cli::ArgsBuilder;
use watchexec::{pathop::PathOp, Args, Handler};
use crate::utils::plural_s;

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
    thread_builder
        .spawn(move || {
            watchexec::run::watch(&handler).unwrap();
        })
        .expect("Cannot create background thread to run the directory watcher");
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
        .debounce(500_u64)
        .build()
        .expect("Construction of Args failed")
}

/// This struct is used to impl the `Handler` trait from `watchexec`.
/// File system events are raised as events on the `sender`.
struct FileEventHandler {
    args: Args,
    sender: Sender<FileSyncEvent>,
}

impl FileEventHandler {
    fn new(args: Args, sender: Sender<FileSyncEvent>) -> Self {
        Self { args, sender }
    }
}

/// High-level events that reflect the changes that are happening within the
/// source directory. A job (FileSyncJob) takes care of making the corresponding
/// changes in the destination directory.
#[derive(Debug, Clone)]
pub enum FileSyncEvent {
    /// A file has been created or updated. In either case, we simply want to
    /// copy the file from the source to the destination.
    FileUpdate(PathBuf),
    /// A file or directory has been deleted. We can't tell which.
    Remove(PathBuf),
}

impl Handler for FileEventHandler {
    /// This method is the one that is called by `watchexec` when a file system event occurs.
    /// Events will have been somewhat debounced already, but we still get a large number
    /// of events for a single file. And because different editors use different strategies of saving
    /// and creating files (including use of backup files and renames) there is really no
    /// telling what sequence of events we might get.
    ///
    /// However, we really only care about two things:
    /// 1. Files or directories that have been deleted. We need to remove these from the shadow
    /// copy directory.
    /// 2. Files that have been created or updated. We need to copy these over to the shadow copy
    /// directory.
    ///
    /// Note that we don't care about directory creation events, since copying a file to the destination
    /// will create all needed parent directories.
    fn on_update(&self, ops: &[watchexec::pathop::PathOp]) -> watchexec::error::Result<bool> {
        // Utility function to actually send the appropriate event.
        fn send_event(me: &FileEventHandler, op: &watchexec::pathop::PathOp) {
            let op_type = op.op.unwrap();

            if PathOp::is_remove(op_type) {
                let event = FileSyncEvent::Remove(op.path.clone());
                me.sender.send(event).unwrap();
                return;
            }

            if std::path::Path::is_file(&op.path) {
                if PathOp::is_create(op_type)
                    || PathOp::is_rename(op_type)
                    || PathOp::is_write(op_type)
                {
                    let event = FileSyncEvent::FileUpdate(op.path.clone());
                    me.sender.send(event).unwrap();
                }
            }
        }

        // Common case we can avoid allocating a HashMap.
        if ops.len() == 1 {
            if ops[0].op.is_some() {
                send_event(self, &ops[0]);
            }

            return Ok(true);
        }

        // If multiple events, take the last event for each distinct path.
        // Within that constraint, we are careful to issue events in the order
        // that we receive them (hence the tuple).
        let mut map = HashMap::<PathBuf, (usize, &PathOp)>::new();
        for op in ops {
            if op.op.is_none() {
                continue;
            }

            let len = map.len();
            match map.entry(op.path.clone()) {
                Entry::Occupied(mut occupied) => {
                    occupied.get_mut().1 = op;
                }
                Entry::Vacant(vacant) => {
                    vacant.insert((len, op));
                }
            }
        }

        if ops.len() != map.len() {
            let plural_s = plural_s(map.len());

            info!(
                "Received {} file operations, simplified to {} event{}",
                ops.len(),
                map.len(),
                plural_s
            );
        }

        let mut events: Vec<_> = map.iter().map(|(pb, (ord, op))| (*ord, pb, *op)).collect();

        // Sort by the first field of the tuple, the ord, which was originally map.len() above.
        // This gives us the events in the order they were sent to us.
        events.sort_by_key(|tpl| tpl.0);

        for (_ord, _pb, op) in events {
            send_event(self, op)
        }

        Ok(true)
    }

    /// This is called if we ask `watchexec` to do a 'manual run'.
    /// We aren't, so it never gets called.
    fn on_manual(&self) -> watchexec::error::Result<bool> {
        Ok(true)
    }

    /// `watchexec` calls this once to get the args.
    fn args(&self) -> Args {
        self.args.clone()
    }
}
