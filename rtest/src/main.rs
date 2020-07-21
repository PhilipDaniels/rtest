use chrono::Utc;
use druid::{AppLauncher, LocalizedString, WindowDesc};
use env_logger::Builder;
use log::info;
use std::{io::Write, sync::mpsc::channel};

mod configuration;
mod engine;
#[path = "jobs/jobs.rs"]
mod jobs;
mod shadow_copy_destination;
mod source_directory_watcher;
mod state;
mod thread_clutch;
mod ui;
mod utils;

use engine::JobEngine;
use jobs::{FileSyncJob, ShadowCopyJob};
use source_directory_watcher::FileSyncEvent;
use state::State;
use ui::build_main_window;

pub const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const CARGO_PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    configure_logging();
    info!("Starting {}", CARGO_PKG_NAME);
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

    // This call blocks this thread.
    create_main_window();

    info!("Stopping {}", CARGO_PKG_NAME);
}

/// Just configures logging in such a way that we can see everything.
/// We are using [env_logger](https://crates.io/crates/env_logger)
/// so everything is configured via environment variables.
fn configure_logging() {
    let mut builder = Builder::from_default_env();
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

fn create_main_window() {
    info!("Creating main window");

    let title_string = LocalizedString::new("rtest-main-window-title")
        .with_placeholder(format!("{} - TDD for Rust", CARGO_PKG_NAME));

    let main_window_desc = WindowDesc::new(build_main_window)
        .window_size((512.0, 512.0))
        .resizable(true)
        .title(title_string);

    let state = ();

    AppLauncher::with_window(main_window_desc)
        .launch(state)
        .expect("Cannot create main window");
}
