use crate::{
    failed,
    jobs::{CompletionStatus, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
    succeeded,
};
use log::info;
use std::{
    fmt::Display,
    process::{Command, ExitStatus},
};
use super::BuildMode;

#[derive(Debug, Clone)]
pub struct TestJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    exit_status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Display for TestJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Run tests in {:?} mode", self.build_mode)
    }
}

impl TestJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::Test(TestJob {
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
            info!("Testing in shadow copy directory {}", dir.display());
            dir
        } else {
            let dir = self.destination.source_directory();
            info!("Testing in the original directory {}", dir.display());
            dir
        };

        // This will build both the main code and the test code, but won't
        // actually run the tests.
        let mut command = Command::new("cargo");
        command.arg("test");
        command.current_dir(cwd);
        command.env("RUST_BACKTRACE", "1");
        command.env("RUSTC_WRAPPER", "sccache");

        let output = command
            .output()
            .expect("`cargo test` command failed to start");

        self.exit_status = Some(output.status);
        self.stdout = output.stdout;
        self.stderr = output.stderr;

        // TODO: Function to tidy up these messages. Use in build also.
        // TODO: Tidy up all job-related messages.
        if output.status.success() {
            succeeded!("`cargo test`. ExitStatus={:?}, stdout = {} bytes, stderr = {} bytes", self.exit_status, self.stdout.len(), self.stderr.len());
            CompletionStatus::Ok
        } else {
            let msg = format!("`cargo test`. ExitStatus={:?}", self.exit_status);
            failed!("`cargo build`. ExitStatus={:?}", self.exit_status);
            msg.into()
        }
    }
}
