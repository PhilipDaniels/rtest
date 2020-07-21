use crate::{
    configuration::BuildMode,
    jobs::{gather_process_stdout, CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use cargo_test_parser::{parse_test_list, ParseError, Tests};
use duct::cmd;
use log::info;
use std::fmt::Display;

/// Lists all the tests. Does not run any of them.
#[derive(Debug, Clone)]
pub struct ListAllTestsJob {
    destination: ShadowCopyDestination,
    build_mode: BuildMode,
    output: String,
}

impl Display for ListAllTestsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "List tests in {:?} mode", self.build_mode)
    }
}

impl ListAllTestsJob {
    pub fn new(destination_directory: ShadowCopyDestination, build_mode: BuildMode) -> PendingJob {
        let kind = JobKind::ListAllTests(ListAllTestsJob {
            destination: destination_directory,
            build_mode,
            output: Default::default(),
        });

        kind.into()
    }

    #[must_use = "Don't ignore the completion status, caller needs to store it"]
    pub fn execute(&mut self, parent_job_id: JobId) -> CompletionStatus {
        let cwd = self.destination.cwd();
        info!("{} Listing tests in {}", parent_job_id, cwd.display());

        let mut args = Vec::new();
        args.push("test");
        args.push("--color");
        args.push("never");
        if self.build_mode == BuildMode::Release {
            args.push("--release");
        }
        args.push("--");
        args.push("--list");

        let cmd = cmd("cargo", args).stderr_to_stdout().dir(cwd);

        self.output = match gather_process_stdout(cmd, "Cargo test listing", parent_job_id) {
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
