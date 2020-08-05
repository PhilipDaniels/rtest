use chrono::Utc;
use log::info;
use std::{io::Write, sync::mpsc::channel};
use gtk::*;
use gtk::prelude::*;
use gio::prelude::*;

use rtest_core::{
    configuration,
    engine::JobEngine,
    jobs::{FileSyncJob, ShadowCopyJob},
    source_directory_watcher,
    state::State,
};
use source_directory_watcher::FileSyncEvent;

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

    info!("Stopping {}", env!("CARGO_PKG_NAME"));

    let application = gtk::Application::new(Some("com.example.example"), Default::default())
        .expect("Initialization failed...");
    
    application.connect_activate(|app| {
        // Load the compiled resource bundle
        let resources_bytes = include_bytes!("../resources/resources.gresource");
        let resource_data = glib::Bytes::from(&resources_bytes[..]);
        let res = gio::Resource::from_data(&resource_data).unwrap();
        gio::resources_register(&res);

        // Load the window UI
        let builder = Builder::from_resource("/org/example/Example/main_window.glade");

        // Get a reference to the window
        let window: ApplicationWindow = builder.get_object("main_window").expect("Couldn't get window");
        window.set_application(Some(app));

        // Show the UI
        window.show_all();
    });

    let args = vec![];
    application.run(&args);
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
