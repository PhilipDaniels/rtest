use crate::{
    configuration::BuildMode,
    jobs::{gather_process_output, CompletionStatus, JobId, JobKind, PendingJob, ProcessOutput},
    shadow_copy_destination::ShadowCopyDestination,
};
use log::info;
use std::{fmt::Display, process::Command};

#[derive(Debug, Clone)]
pub struct RunTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: Option<ProcessOutput>,
}

impl Display for RunTestsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Run tests in {:?} mode", self.build_mode)
    }
}

impl RunTestsJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::RunTests(RunTestsJob {
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

        // cargo test --no-fail-fast -- --show-output --test-threads=1 --color never
        let mut command = Command::new("cargo");
        command.arg("test");
        command.current_dir(cwd);

        self.output = match gather_process_output(command, "Run tests", parent_job_id) {
            Ok(output) => Some(output),
            Err(msg) => return msg.into(),
        };

        CompletionStatus::Ok
    }
}

/*
Thread on unstable options to cargo
https://users.rust-lang.org/t/capture-test-output-in-an-object/38082/2
*/

/*
This command: cargo test -- --help
Shows the options that can be passed after the -- argument. These options are
passed to the test executables.

# List all tests: cargo test -q -- --list
   This outputs to stdout, compile warnings go to stderr


For normal channels, the best we can do is:
    RUST_BACKTRACE=1 RUST_LOG=info cargo test --no-fail-fast -- --show-output --test-threads=1 --color never
# CONFIG
    - RUST_BACKTRACE environment variable
    - RUST_LOG environment variable.
    - Number of test threads (for speed, default to 1)
# Logging
    - Capturing log output depends on how the test are written and which logging framework
      is being used. For example, see https://docs.rs/env_logger/0.7.1/env_logger/#capturing-logs-in-tests


running 2 tests
test tests::test1_passing ... ok
test tests::test2_failing ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
*/

/*
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
