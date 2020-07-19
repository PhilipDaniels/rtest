use crate::{
    configuration::BuildMode,
    jobs::{gather_process_output, CompletionStatus, JobId, JobKind, PendingJob, ProcessOutput},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::info;
use std::{fmt::Display, process::Command};

/// Builds the tests **only**. This will fail if there is a compilation error in the main (non-test)
/// code. The difference from `cargo build` is that it doesn't build the final crate target (such
/// as an EXE for a bin crate). Some time is therefore saved on linking.
///
/// See also the `BuildCrateJob`.
#[derive(Debug, Clone)]
pub struct BuildTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: Option<ProcessOutput>,
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
            output: None,
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = if self.destination.is_copying() {
            let dir = self.destination.destination_directory().unwrap();
            info!(
                "{} Building tests in shadow copy directory {}",
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
        // cargo test --no-run --color never [--release]
        let mut command = Command::new("cargo");
        command.current_dir(cwd);

        command.arg("test");
        command.arg("--no-run");
        command.arg("--color");
        command.arg("never");
        if self.build_mode == BuildMode::Release {
            command.arg("--release");
        }

        self.output = match gather_process_output(command, "Build tests", parent_job_id) {
            Ok(output) => Some(output),
            Err(msg) => return msg.into(),
        };

        CompletionStatus::Ok
    }
}
