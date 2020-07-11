use crate::{CARGO_PKG_AUTHORS, CARGO_PKG_DESCRIPTION, CARGO_PKG_NAME, CARGO_PKG_VERSION};
use clap::{App, Arg};
use log::info;
use std::path::PathBuf;
use tempfile::TempDir;

/// Specifies the mode that the shadow-copy destination directory works in.
#[derive(Debug)]
pub enum DestinationType {
    /// The source directory is used. No shadow-copying is done.
    SourceDirectory(PathBuf),
    /// A specific, user-named directory is used.
    NamedDirectory(PathBuf),
    /// A temporary directory created and cleaned up by the system is used.
    TempDirectory(TempDir),
}

#[derive(Debug)]
pub struct Configuration {
    /// The directory that contains the sources we will be testing.
    pub source_directory: PathBuf,
    pub destination_directory: DestinationType,
}

pub fn new() -> Configuration {
    let args = get_cli_arguments();
    info!("CLI {:?}", args);

    let destination_directory = if args.do_shadow_copy {
        if args.destination.is_none() {
            let dir = tempfile::tempdir().expect("Cannot create tempdir");
            DestinationType::TempDirectory(dir)
        } else {
            DestinationType::NamedDirectory(args.destination.unwrap())
        }
    } else {
        DestinationType::SourceDirectory(args.source.clone())
    };

    Configuration {
        source_directory: args.source,
        destination_directory: destination_directory,
    }
}

#[derive(Debug)]
struct Arguments {
    do_shadow_copy: bool,
    source: PathBuf,
    destination: Option<PathBuf>,
}

/// Use clap to parse the arguments.
fn get_cli_arguments() -> Arguments {
    let matches = App::new(CARGO_PKG_NAME)
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(CARGO_PKG_DESCRIPTION)
        .arg(
            Arg::new("shadow-copy")
                .about("Do not shadow copy, use the original source directory for compilations")
                .short('n')
                .long("no-copy"),
        )
        .arg("[source] 'The source directory (defaults to cwd)'")
        .arg("[dest] 'The destination directory for shadow copies' (defaults to a temp folder)")
        .get_matches();

    let do_shadow_copy = !matches.is_present("shadow-copy");

    let source = matches.value_of("source").map_or_else(
        || std::env::current_dir().expect("Cannot determine cwd"),
        |v| v.into(),
    );

    let destination = matches.value_of("dest").map(|v| v.into());

    Arguments {
        do_shadow_copy,
        source,
        destination,
    }
}
