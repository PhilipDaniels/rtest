use crate::{
    configuration::BuildMode,
    jobs::{gather_process_output, CompletionStatus, JobId, JobKind, PendingJob, ProcessOutput},
    shadow_copy_destination::ShadowCopyDestination,
};
use cargo_test_parser::{parse_test_list, ParseError, Tests};
use log::info;
use std::{fmt::Display, process::Command};

/// Lists the tests. Does not run any of them.
#[derive(Debug, Clone)]
pub struct ListTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: Option<ProcessOutput>,
    tests: Vec<()>,
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
            output: None,
            tests: Default::default(),
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

        // cargo test -q [--release] -- --list
        let mut command = Command::new("cargo");
        command.current_dir(cwd);

        command.arg("test");
        command.arg("-q");
        if self.build_mode == BuildMode::Release {
            command.arg("--release");
        }
        command.arg("--");
        command.arg("--list");

        self.output = match gather_process_output(command, "Lists tests", parent_job_id) {
            Ok(output) => Some(output),
            Err(msg) => return msg.into(),
        };

        CompletionStatus::Ok
    }

    /// Parses the cargo test output from stdout and returns the
    /// set of tests. Since this is based on textual parsing, this
    /// can fail. What are all the output variations of cargo?
    pub fn parse_tests(&self) -> Result<Vec<Tests>, ParseError> {
        match &self.output {
            Some(output) => parse_test_list(&output.stdout),
            None => Ok(vec![]),
        }
    }
}
