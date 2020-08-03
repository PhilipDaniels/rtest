use crate::{
    configuration::BuildMode,
    jobs::{gather_process_stdout, CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use duct::cmd;
use log::info;
use std::fmt::Display;

/// Builds the crate (or workspace). This makes the final product(s) available
/// quickly, as a convenience to the user.
///
/// See also the `BuildTestsJob`.
#[derive(Debug, Clone)]
pub struct BuildWorkspaceJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: String,
}

impl Display for BuildWorkspaceJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build crate in {:?} mode", self.build_mode)
    }
}

impl BuildWorkspaceJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::BuildWorkspace(BuildWorkspaceJob {
            destination: destination_directory,
            build_mode,
            output: Default::default(),
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = self.destination.cwd();
        info!(
            "{} Building crate or workspace in {}",
            parent_job_id,
            cwd.display()
        );

        let mut args = Vec::new();
        args.push("build");
        args.push("--color");
        args.push("never");
        if self.build_mode == BuildMode::Release {
            args.push("--release");
        }

        let cmd = cmd("cargo", args).stderr_to_stdout().dir(cwd);

        self.output = match gather_process_stdout(cmd, "Build crate or workspace", parent_job_id) {
            Ok(output) => output,
            Err(err) => return err.to_string().into(),
        };

        CompletionStatus::Ok
    }
}
