//! A module of constructor functions for creating new jobs.

use crate::job::{Job, JobKind};
use std::path::PathBuf;

/// Create a new shadow copy job.
pub fn shadow_copy<P>(source: P, destination: P) -> Job
where
    P: Into<PathBuf>,
{
    let kind = JobKind::ShadowCopy(source.into(), destination.into());
    Job::new(kind)
}
