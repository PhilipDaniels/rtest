use crate::{
    shadow_copy_destination::ShadowCopyDestination
};
use clap::{App, Arg};
use log::info;
use std::{ops::Deref, path::PathBuf, str::FromStr, sync::Arc};

/// Represents the global configuration of `rtest` during one run.
#[derive(Debug, Clone)]
pub struct Configuration {
    inner: Arc<InnerConfiguration>,
}

#[derive(Debug, Clone)]
pub struct InnerConfiguration {
    pub source_directory: PathBuf,
    pub destination: ShadowCopyDestination,
    pub build_mode: CompilationMode,
    pub test_mode: CompilationMode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Specifies what compilation mode should be applied.
/// At the moment we only support Debug (dev) and Release profiles.
/// Since Cargo does not yet support named profiles that
/// is not really a problem.
pub enum CompilationMode {
    None,
    Debug,
    Release,
    Both,
}

/// The `BuildMode` is used to parameterise invocations
/// of cargo subprocesses - do we add "--release"?.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release
}

impl Deref for Configuration {
    type Target = InnerConfiguration;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
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
        inner: Arc::new(InnerConfiguration {
            source_directory: args.source,
            destination,
            build_mode: args.build_mode,
            test_mode: args.test_mode,
        }),
    }
}

#[derive(Debug)]
struct CommandLineArguments {
    do_shadow_copy: bool,
    source: PathBuf,
    destination: Option<PathBuf>,
    build_mode: CompilationMode,
    test_mode: CompilationMode,
}

impl FromStr for CompilationMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(CompilationMode::None),
            "debug" => Ok(CompilationMode::Debug),
            "release" => Ok(CompilationMode::Release),
            "both" => Ok(CompilationMode::Both),
            _ => Err("no matching CompilationMode"),
        }
    }
}

fn get_cli_arguments() -> CommandLineArguments {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about( env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("shadow-copy")
                .about("Do not shadow copy, use the original source directory for compilations")
                .short('n')
                .long("no-copy"),
        )
        .arg(
            Arg::with_name("BUILD-MODE")
                .about("Specifies compilation mode for builds")
                .short('b')
                .long("build-mode")
                .possible_values(&["none", "debug", "release", "both"]),
        )
        .arg(
            Arg::with_name("TEST-MODE")
                .about("Specifies compilation mode for tests")
                .short('t')
                .long("test-mode")
                .possible_values(&["none", "debug", "release", "both"]),
        )
        .arg("[source] 'The source directory (defaults to cwd)'")
        .arg("[dest] 'The destination directory for shadow copies (defaults to a temp folder)'")
        .get_matches();

    let do_shadow_copy = !matches.is_present("shadow-copy");

    let source = matches.value_of("source").map_or_else(
        || std::env::current_dir().expect("Cannot determine cwd"),
        |v| v.into(),
    );

    let destination = matches.value_of("dest").map(|v| v.into());

    let build_mode = CompilationMode::from_str(matches.value_of("BUILD-MODE").unwrap_or("none"))
        .expect("Invalid BUILD-MODE");
    let test_mode = CompilationMode::from_str(matches.value_of("TEST-MODE").unwrap_or("debug"))
        .expect("Invalid TEST-MODE");

    CommandLineArguments {
        do_shadow_copy,
        source,
        destination,
        build_mode,
        test_mode,
    }
}
