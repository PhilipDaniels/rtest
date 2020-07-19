use crate::{
    configuration::{BuildMode, CompilationMode},
    jobs::{CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::{info, warn};
use std::{
    fmt::Display,
    process::{Command, ExitStatus},
};

/// Builds the crate. This makes the final product available, as a convenience.
///
/// See also the `BuildTestsJob`.
#[derive(Debug, Clone)]
pub struct BuildCrateJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    exit_status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Display for BuildCrateJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build crate in {:?} mode", self.build_mode)
    }
}

impl BuildCrateJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::BuildCrate(BuildCrateJob {
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
                "{} Building crate in shadow copy directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        } else {
            let dir = self.destination.source_directory();
            info!(
                "{} Building crate in the original directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        };

        // This will build both the main code and the test code, but won't
        // actually run the tests.
        let mut command = Command::new("cargo");
        command.current_dir(cwd);

        command.arg("build");
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
            "{} Build crate {}. ExitStatus={:?}, stdout={} bytes, stderr={} bytes",
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
