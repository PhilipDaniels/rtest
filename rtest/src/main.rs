use chrono::Utc;
use log::info;
use std::{io::Write, sync::mpsc::channel};

use rtest_core::{
    configuration,
    engine::JobEngine,
    jobs::{FileSyncJob, ShadowCopyJob},
    source_directory_watcher,
    state::State,
};
use source_directory_watcher::FileSyncEvent;

mod ui;

fn main() {
    configure_logging();
    info!("Starting {}", env!("CARGO_PKG_NAME"));
    let config = configuration::new();
    info!("{:?}", config);

    let state = State::new();
    let engine = JobEngine::new(config.clone(), state.clone());

    // If a shadow copy operation is required, kick one off.
    if config.destination.is_copying() {
        let job = ShadowCopyJob::new(config.destination.clone());
        engine.add_job(job);

        // Then watch for incremental file changes. Use another thread to
        // add jobs to the engine.
        let (sender, receiver) = channel::<FileSyncEvent>();
        source_directory_watcher::start_watching(&config.source_directory, sender);

        std::thread::spawn({
            let engine = engine.clone();
            let dest = config.destination.clone();

            move || {
                for event in receiver {
                    let job = FileSyncJob::new(dest.clone(), event);
                    engine.add_job(job);
                }
            }
        });
    }

    ui::show_main_window();

    info!("Stopping {}", env!("CARGO_PKG_NAME"));
}

/// Just configures logging in such a way that we can see everything.
/// We are using [env_logger](https://crates.io/crates/env_logger)
/// so everything is configured via environment variables.
fn configure_logging() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format(|buf, record| {
        let utc = Utc::now();

        match (record.file(), record.line()) {
            (Some(file), Some(line)) => writeln!(
                buf,
                "{:?} {} [{}/{}] {}",
                utc,
                record.level(),
                file,
                line,
                record.args()
            ),
            (Some(file), None) => writeln!(
                buf,
                "{:?} {} [{}] {}",
                utc,
                record.level(),
                file,
                record.args()
            ),
            (None, Some(_line)) => writeln!(buf, "{:?} {} {}", utc, record.level(), record.args()),
            (None, None) => writeln!(buf, "{:?} {} {}", utc, record.level(), record.args()),
        }
    });

    builder.init();
}
