use crate::{
    configuration::BuildMode,
    jobs::{gather_process_stdout, CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use duct::cmd;
use log::info;
use std::fmt::Display;

/// Builds the tests but don't run them. This will fail if there is a compilation error in the main
/// (non-test)/// code. The difference from `cargo build` is that it doesn't build the final crate
/// target (such as an EXE for a bin crate). Some time is therefore saved on linking.
///
/// See also the `BuildCrateJob`.
#[derive(Debug, Clone)]
pub struct BuildAllTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: String,
}

impl Display for BuildAllTestsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build tests in {:?} mode", self.build_mode)
    }
}

impl BuildAllTestsJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::BuildAllTests(BuildAllTestsJob {
            destination: destination_directory,
            build_mode,
            output: Default::default(),
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = self.destination.cwd();
        info!("{} Building tests in {}", parent_job_id, cwd.display());

        let mut args = Vec::new();
        args.push("test");
        args.push("--no-run");
        args.push("--color");
        args.push("never");
        if self.build_mode == BuildMode::Release {
            args.push("--release");
        }

        let cmd = cmd("cargo", args).stderr_to_stdout().dir(cwd);

        self.output = match gather_process_stdout(cmd, "Cargo build tests", parent_job_id) {
            Ok(output) => output,
            Err(err) => return err.to_string().into(),
        };

        CompletionStatus::Ok
    }
}
