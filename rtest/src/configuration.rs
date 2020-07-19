use crate::{CARGO_PKG_AUTHORS, CARGO_PKG_DESCRIPTION, CARGO_PKG_NAME, CARGO_PKG_VERSION};
use clap::{App, Arg};
use log::info;
use std::path::PathBuf;

/// Represents the destination directory for the shadow-copy operation.
/// If `UseSourceDirectory`, then no shadow copying is performed and
/// all operations are performed in the original (source) directory.
#[derive(Debug, Clone)]
pub enum DestinationDirectory {
    SameAsSource,
    NamedDirectory(PathBuf),
}

impl DestinationDirectory {
    /// Returns `true` if shadow-copy operations are actually being peformed.
    /// Alternatively, if we are doing everything in the source without shadow
    /// copying, then `false` is returned.
    pub fn is_copying(&self) -> bool {
        match self {
            DestinationDirectory::SameAsSource => false,
            DestinationDirectory::NamedDirectory(_) => true,
        }
    }
}

/// Represents the global configuration of `rtest` during one run.
#[derive(Debug)]
pub struct Configuration {
    pub source_directory: PathBuf,
    pub destination_directory: DestinationDirectory,
}

pub fn new() -> Configuration {
    let args = get_cli_arguments();
    info!("CLI {:?}", args);

    let destination_directory = if args.do_shadow_copy {
        if args.destination.is_none() {
            let temp_dir = tempfile::tempdir().expect("Cannot create tempdir");
            DestinationDirectory::NamedDirectory(temp_dir.path().into())
        } else {
            DestinationDirectory::NamedDirectory(args.destination.unwrap())
        }
    } else {
        DestinationDirectory::SameAsSource
    };

    Configuration {
        source_directory: args.source,
        destination_directory,
    }
}

#[derive(Debug)]
struct CommandLineArguments {
    do_shadow_copy: bool,
    source: PathBuf,
    destination: Option<PathBuf>,
}

fn get_cli_arguments() -> CommandLineArguments {
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

    CommandLineArguments {
        do_shadow_copy,
        source,
        destination,
    }
}
