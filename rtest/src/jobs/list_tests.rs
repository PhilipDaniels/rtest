use crate::{
    jobs::{BuildMode, CompletionStatus, JobId, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
};
use cargo_test_parser::{parse_test_list, Tests, ParseError};
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
    stdout: String,
    stderr: String,
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
            exit_status: None,
            stdout: Default::default(),
            stderr: Default::default(),
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
            .expect("List tests command failed");

        self.exit_status = Some(output.status);
        self.stdout = String::from_utf8(output.stdout)
            .unwrap_or("Error, cannot convert cargo stdout to a string".into());
        self.stderr = String::from_utf8(output.stderr)
            .unwrap_or("Error, cannot convert cargo stderr to a string".into());

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

    /// Parses the cargo test output from stdout and returns the
    /// set of tests. Since this is based on textual parsing, this
    /// can fail. What are all the output variations of cargo?
    pub fn parse_tests(&self) -> Result<Vec<Tests>, ParseError> {
        parse_test_list(&self.stdout)
        //Ok(vec![])
    }
}
