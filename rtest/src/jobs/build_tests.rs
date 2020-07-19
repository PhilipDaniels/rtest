use crate::{
    jobs::{BuildMode, CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::{info, warn};
use std::{
    fmt::Display,
    process::{Command, ExitStatus},
};

/// Builds the tests **only**. This will fail if there is a compilation error in the main (non-test)
/// code. The difference from `cargo build` is that it doesn't build the final crate target (such
/// as an EXE for a bin crate). Some time is therefore saved on linking.
///
/// See also the `BuildCrateJob`.
#[derive(Debug, Clone)]
pub struct BuildTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    exit_status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Display for BuildTestsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build tests in {:?} mode", self.build_mode)
    }
}

impl BuildTestsJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::BuildTests(BuildTestsJob {
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
                "{} Building test in shadow copy directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        } else {
            let dir = self.destination.source_directory();
            info!(
                "{} Building tests in the original directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        };

        // This will build both the main code and the test code, but won't
        // actually run the tests.
        let mut command = Command::new("cargo");
        command.current_dir(cwd);

        command.arg("test");
        command.arg("--no-run");
        command.arg("--color");
        command.arg("never");
        if self.build_mode == BuildMode::Release {
            command.arg("--release");
        }

        let output = command.output().expect("Build tests command failed");

        self.exit_status = Some(output.status);
        self.stdout = output.stdout;
        self.stderr = output.stderr;

        let msg = format!(
            "{} Build tests {}. ExitStatus={:?}, stdout={} bytes, stderr={} bytes",
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
