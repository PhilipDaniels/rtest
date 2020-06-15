use std::io::Write;

use chrono::{DateTime, Utc};
use env_logger::Builder;
use log::info;

use druid::piet::Color;
use druid::widget::{Label, Button, Align, Container, Padding, Split, Flex, CrossAxisAlignment, SizedBox};
use druid::{AppLauncher, LocalizedString, Widget, WidgetExt, WindowDesc};


pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
    configure_logging();
    info!("Starting {}", PROGRAM_NAME);
    create_main_window();
    info!("Stopping {}", PROGRAM_NAME);
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

fn build_main_window() -> impl Widget<()> {
    // Padding occurs outside the border.

    // ---- Construct the tabstrip at the top of the screen ----
    let tabstrip = Button::new("THE TABSTRIP GOES HERE")
        .center()
        .border(Color::WHITE, 1.0)
        .padding(4.0);
    let tabstrip = SizedBox::new(tabstrip).height(50.0);

    // ---- To start with, there is only 1 'panel' for us to select. ----
    // ---- The one corresponding to when the 'Tests' tab is selected. ----
    // A toolbar which will contain the controls for the 'Tests', e.g. 'Run Tests' button.
    let test_toolbar = Button::new("TEST TOOLBAR")
        .border(Color::WHITE, 1.0)
        .expand_width()
        .padding(4.0);
    let test_toolbar = SizedBox::new(test_toolbar).height(50.0);

    // This splitter contains the treeview on the LHS and the results on the RHS.
    let test_tree_splitter = Split::columns(
        Label::new("TEST TREE"),
        Label::new("TEST RESULTS"))
        .split_point(0.35)
        .draggable(true)
        .min_size(120.0)
        .border(Color::WHITE, 1.0)
        .expand()
        .padding(4.0);

    // This constructs the actual panel containing those two controls.
    let test_tree_panel = Flex::column()
        .with_child(test_toolbar)
        .with_flex_child(test_tree_splitter, 1.0)
        .background(Color::rgb8(128,128,128))
        .expand();

    // Finally put them all together.
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(tabstrip)
        .with_flex_child(test_tree_panel, 1.0)
}

fn create_main_window() {
    let title_placeholder = format!("{} - TDD for Rust", PROGRAM_NAME);

    let main_window_desc = WindowDesc::new(build_main_window)
        .window_size((512.0, 512.0))
        .resizable(true)
        .title(
            LocalizedString::new("rtest-main-window-title")
                .with_placeholder(&title_placeholder),
        );

    let state = ();

    AppLauncher::with_window(main_window_desc)
        .launch(state)
        .expect("Cannot create main window");
}
