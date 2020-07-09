use crate::{
    failed,
    jobs::{JobKind, PendingJob, CompletionStatus},
    shadow_copy_destination::ShadowCopyDestination,
    succeeded,
};
use log::info;
use std::{
    fmt::Display,
    process::{Command, ExitStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

#[derive(Debug, Clone)]
pub struct BuildJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    exit_status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Display for BuildJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build in {:?} mode", self.build_mode)
    }
}

impl BuildJob {
    pub fn succeeded(&self) -> bool {
        self.exit_status.map_or(false, |s| s.success())
    }

    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::Build(BuildJob {
            destination: destination_directory,
            build_mode,
            exit_status: None,
            stdout: Vec::default(),
            stderr: Vec::default(),
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self) -> CompletionStatus {
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

        let output = command
            .output()
            .expect("`cargo build` command failed to start");

        self.exit_status = Some(output.status);
        self.stdout = output.stdout;
        self.stderr = output.stderr;

        if output.status.success() {
            succeeded!("`cargo build`. ExitStatus={:?}", self.exit_status);
            succeeded!("BUILD.stdout = {} bytes", self.stdout.len());
            succeeded!("BUILD.stderr = {} bytes", self.stderr.len());
            CompletionStatus::Ok
        } else {
            let msg = format!("`cargo build`. ExitStatus={:?}", self.exit_status);
            failed!("`cargo build`. ExitStatus={:?}", self.exit_status);
            msg.into()
        }
    }
}
