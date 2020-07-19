use crate::{
    configuration::BuildMode,
    jobs::{gather_process_output, CompletionStatus, JobId, JobKind, PendingJob, ProcessOutput},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::info;
use std::{fmt::Display, process::Command};

/// Builds the crate (or workspace). This makes the final product(s) available
/// quickly, as a convenience to the user.
///
/// See also the `BuildTestsJob`.
#[derive(Debug, Clone)]
pub struct BuildCrateJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: Option<ProcessOutput>,
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
            output: None,
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = if self.destination.is_copying() {
            let dir = self.destination.destination_directory().unwrap();
            info!(
                "{} Building crate or workspace in shadow copy directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        } else {
            let dir = self.destination.source_directory();
            info!(
                "{} Building crate or workspace in the original directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        };

        // cargo build --color never [--release]
        let mut command = Command::new("cargo");
        command.current_dir(cwd);

        command.arg("build");
        command.arg("--color");
        command.arg("never");
        if self.build_mode == BuildMode::Release {
            command.arg("--release");
        }

        self.output = match gather_process_output(command, "Build crate/workspace", parent_job_id) {
            Ok(output) => Some(output),
            Err(msg) => return msg.into(),
        };

        CompletionStatus::Ok
    }
}
