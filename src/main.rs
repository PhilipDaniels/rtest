use std::io::Write;

use chrono::{DateTime, Utc};
use druid::widget::Label;
use env_logger::Builder;
use log::info;
use druid::piet::Color;
use druid::widget::{Align, Container, Padding, Split, Flex, CrossAxisAlignment, SizedBox};
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
    let label = Label::new("Hello world")
        .center()
        .border(Color::WHITE, 1.0)
        .padding(4.0);  // Padding occurs outside the border.

    let label2 = Label::new("Hello world 2222")
        .center()
        .border(Color::WHITE, 1.0)
        .padding(4.0);  // Padding occurs outside the border.

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        //.must_fill_main_axis(true)
        .with_child(SizedBox::new(label).height(60.0))
        .with_flex_child(label2, 1.0)

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
