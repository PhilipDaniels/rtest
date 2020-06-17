use chrono::{DateTime, Utc};
use druid::{AppLauncher, LocalizedString, WindowDesc};
use env_logger::Builder;
use log::info;
use std::io::Write;

mod configuration;
mod engine;
mod ui;

use engine::{Job, JobEngine};
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

    let mut engine = JobEngine::new();
    engine.add_job(Job::ShadowCopy);
    engine.add_job(Job::ShadowCopy);
    std::thread::sleep_ms(5000);
    engine.add_job(Job::ShadowCopy);

    create_main_window();
    info!("Stopping {}", CARGO_PKG_NAME);
}

/// Just configures logging in such a way that we can see everything.
/// We are using [env_logger](https://crates.io/crates/env_logger)
/// so everything is configured via environment variables.
fn configure_logging() {
    let mut builder = Builder::from_default_env();
    builder.format(|buf, record| {
        let utc: DateTime<Utc> = Utc::now();

        write!(
            buf,
            "{:?} {} [{}] ",
            //utc.format("%Y-%m-%dT%H:%M:%S.%fZ"),
            utc, // same, probably faster?
            record.level(),
            record.target()
        )?;

        match (record.file(), record.line()) {
            (Some(file), Some(line)) => write!(buf, "[{}/{}] ", file, line),
            (Some(file), None) => write!(buf, "[{}] ", file),
            (None, Some(_line)) => write!(buf, " "),
            (None, None) => write!(buf, " "),
        }?;

        writeln!(buf, "{}", record.args())
    });

    builder.init();
}

fn create_main_window() {
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
