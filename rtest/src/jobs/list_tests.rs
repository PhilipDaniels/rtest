use crate::{
    configuration::BuildMode,
    jobs::{CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use cargo_test_parser::{parse_test_list, ParseError, Tests};
use duct::cmd;
use log::info;
use std::fmt::Display;

/// Lists the tests. Does not run any of them.
#[derive(Debug, Clone)]
pub struct ListTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: String,
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
            output: Default::default(),
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

        let mut args = Vec::new();
        args.push("test");
        args.push("--color");
        args.push("never");
        if self.build_mode == BuildMode::Release {
            args.push("--release");
        }
        args.push("--");
        args.push("--list");

        let cmd = cmd("cargo", args);

        self.output = match cmd.stderr_to_stdout().read() {
            Ok(output) => output,
            Err(err) => return err.to_string().into(),
        };

        CompletionStatus::Ok
    }

    /// Parses the cargo test output from stdout and returns the
    /// set of tests. Since this is based on textual parsing, this
    /// can fail. What are all the output variations of cargo?
    pub fn parse_tests(&self) -> Result<Vec<Tests>, ParseError> {
        //info!("PARSING TEST LIST: {}", &self.output);
        parse_test_list(&self.output)
    }
}
