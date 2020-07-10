use super::JobId;
use crate::{
    jobs::{CompletionStatus, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::{info, warn};
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
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = if self.destination.is_copying() {
            let dir = self.destination.destination_directory().unwrap();
            info!(
                "{} Building in shadow copy directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        } else {
            let dir = self.destination.source_directory();
            info!(
                "{} Building in the original directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        };

        // This will build both the main code and the test code, but won't
        // actually run the tests.
        let mut command = Command::new("cargo");
        command.arg("test");
        command.arg("--no-run");
        command.current_dir(cwd);

        let output = command.output().expect("Build command failed to start");

        self.exit_status = Some(output.status);
        self.stdout = output.stdout;
        self.stderr = output.stderr;

        let msg = format!(
            "{} Build {}. ExitStatus={:?}, stdout={} bytes, stderr={} bytes",
            parent_job_id,
            if output.status.success() {
                "succeeded"
            } else {
                "failed"
            },
            self.exit_status,
            self.stdout.len(),
            self.stderr.len()
        );

        if output.status.success() {
            info!("{}", msg);
            CompletionStatus::Ok
        } else {
            warn!("{}", msg);
            msg.into()
        }
    }
}
