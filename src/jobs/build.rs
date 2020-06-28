use crate::{
    jobs::{Job, JobKind},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::{debug, info};
use std::{fmt::Display, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

#[derive(Debug, Clone)]
pub struct BuildJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
}

impl Display for BuildJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build")
    }
}

impl BuildJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> Job {
        let kind = JobKind::Build(BuildJob {
            destination: destination_directory,
            build_mode,
        });

        Job::new(kind)
    }

    pub fn execute(&mut self) {
        let cwd = if self.destination.is_copying() {
            let dir = self.destination.destination_directory().unwrap();
            info!("Building in shadow copy directory {}", dir.display());
            dir
        } else {
            let dir = self.destination.source_directory();
            info!("Building in the original directory {}", dir.display());
            dir
        };

        let mut command = Command::new("cargo");
        command.arg("build");
        command.current_dir(cwd);
        command.env("RUST_BACKTRACE", "1");
        command.env("RUSTC_WRAPPER", "");

        //let handle = command.spawn().expect("cargo build failed");
        let output = command.output()
            .expect("`cargo build` command failed to start");

        debug!("Got `cargo build` output: {:?}", output);
    }
}
