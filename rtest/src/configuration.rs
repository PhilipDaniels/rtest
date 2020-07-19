use crate::{
    shadow_copy_destination::ShadowCopyDestination, CARGO_PKG_AUTHORS, CARGO_PKG_DESCRIPTION,
    CARGO_PKG_NAME, CARGO_PKG_VERSION,
};
use clap::{App, Arg};
use log::info;
use std::path::PathBuf;

/// Represents the global configuration of `rtest` during one run.
#[derive(Debug, Clone)]
pub struct Configuration {
    pub source_directory: PathBuf,
    pub destination: ShadowCopyDestination,
}

pub fn new() -> Configuration {
    let args = get_cli_arguments();
    info!("CLI {:?}", args);

    let destination = if args.do_shadow_copy {
        if args.destination.is_none() {
            let temp_dir = tempfile::tempdir().expect("Cannot create tempdir");
            ShadowCopyDestination::new(
                args.source.to_path_buf(),
                Some(temp_dir.path().to_path_buf()),
            )
        } else {
            ShadowCopyDestination::new(args.source.to_path_buf(), Some(args.destination.unwrap()))
        }
    } else {
        ShadowCopyDestination::new(args.source.to_path_buf(), None)
    };

    Configuration {
        source_directory: args.source,
        destination,
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
