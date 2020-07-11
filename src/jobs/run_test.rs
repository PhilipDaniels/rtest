use super::{BuildMode, JobId};
use crate::{
    jobs::{CompletionStatus, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::{info, warn};
use std::{
    fmt::Display,
    process::{Command, ExitStatus},
};

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
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = if self.destination.is_copying() {
            let dir = self.destination.destination_directory().unwrap();
            info!(
                "{} Testing in shadow copy directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        } else {
            let dir = self.destination.source_directory();
            info!(
                "{} Testing in the original directory {}",
                parent_job_id,
                dir.display()
            );
            dir
        };

        let mut command = Command::new("cargo");
        command.arg("test");
        command.current_dir(cwd);

        let output = command
            .output()
            .expect("`cargo test` command failed to start");

        self.exit_status = Some(output.status);
        self.stdout = output.stdout;
        self.stderr = output.stderr;

        let num_passed = 0;
        let num_failed = 0;

        let msg = format!(
            "{} 'cargo test' {}. ExitStatus={:?}, Passed={}, Failed={}, stdout={} bytes, stderr={} bytes",
            parent_job_id,
            if output.status.success() { "succeeded" } else { "failed" },
            self.exit_status,
            num_passed,
            num_failed,
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

/*
For normal channels, the best we can do is:

running 2 tests
test tests::test1_passing ... ok
test tests::test2_failing ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out



For nightly channels, we can do:
    rustup run nightly cargo test -- -Z unstable-options --format=json
    cargo +nightly test -- -Z unstable-options --format=json
which results in

{ "type": "suite", "event": "started", "test_count": 2 }
{ "type": "test", "event": "started", "name": "tests::test1_passing" }
{ "type": "test", "event": "started", "name": "tests::test2_failing" }
{ "type": "test", "name": "tests::test1_passing", "event": "ok" }
{ "type": "test", "name": "tests::test2_failing", "event": "ok" }
{ "type": "suite", "event": "ok", "passed": 2, "failed": 0, "allowed_fail": 0, "ignored": 0, "measured": 0, "filtered_out": 0 }

See libtest in rustlang

*/