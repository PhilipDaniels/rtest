use crate::{
    jobs::{BuildMode, CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::{info, warn};
use std::{
    fmt::Display,
    process::{Command, ExitStatus},
};

/// Lists the tests. Does not run any of them.
#[derive(Debug, Clone)]
pub struct ListTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    exit_status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Display for ListTestsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "List tests in {:?} mode", self.build_mode)
    }
}

impl ListTestsJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::ListTests(ListTestsJob {
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
                "{} Listing tests in shadow copy directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        } else {
            let dir = self.destination.source_directory();
            info!(
                "{} Listing tests in the original directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        };

        // cargo test -q -- --list
        let mut command = Command::new("cargo");
        command.current_dir(cwd);

        command.arg("test");
        command.arg("-q");
        if self.build_mode == BuildMode::Release {
            command.arg("--release");
        }
        command.arg("--");
        command.arg("--list");

        let output = command
            .output()
            .expect("List tests command failed to start");

        self.exit_status = Some(output.status);
        self.stdout = output.stdout;
        self.stderr = output.stderr;

        let msg = format!(
            "{} List tests {}. ExitStatus={:?}, stdout={} bytes, stderr={} bytes",
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
