use crate::{
    jobs::{BuildJob, BuildMode, CompletedJob, CompletionStatus, Job, JobKind, PendingJob},
    shadow_copy_destination::ShadowCopyDestination,
    thread_clutch::ThreadClutch,
};
use log::info;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex, MutexGuard,
};
use std::thread;

/*
* While a job is executing, the GUI needs to update to show the latest status.
* When a job is completed, we will still want to display details in the GUI.
  For example, a list of completed tests. So the GUI is based on Vec<Tests>,
  and each test is linked to a particular job. One job may be linked to
  several tests.
* We need to support cancellation of jobs.
* When a job finishes execution it may create N more jobs.

Algorithm for adding file sync
FOR SOME PATH P
If a build is running, stop it
If OP is REMOVE, remove all file copy jobs and create a remove job
ELSE
    if there is a previous job for this file, remove it and insert a new COPY job (op is likely to be WRITE, CLOSE_WRITE, RENAME or CHMOD)

Flag for 'is a build required'?
*/

#[derive(Debug, Clone)]
pub struct JobEngine {
    dest_dir: ShadowCopyDestination,

    /// The list of pending (yet to be executed) jobs.
    pending_jobs: Arc<Mutex<VecDeque<PendingJob>>>,

    executing_job: Arc<Mutex<Option<PendingJob>>>,

    /// The list of completed jobs.
    completed_jobs: Arc<Mutex<VecDeque<CompletedJob>>>,

    /// A clutch that allows us to pause and restart the JOB_STARTER thread.
    /// This basically allows us to pause the entire job queue, because if we
    /// don't start to execute new jobs, nothing happens. Yet we can still
    /// add new jobs to the queue, because that is controlled by a different thread.
    job_starter_clutch: ThreadClutch,

    /// The `job_added_signal` is notified when a new job is added to the pending queue.
    /// This will cause the JOB_STARTER thread to wake up (it goes to sleep when
    /// there are no pending jobs).
    job_added_signal: Arc<Condvar>,

    build_required: Arc<AtomicBool>,
}

impl JobEngine {
    /// Creates a new job engine that is running and ready to process jobs.
    pub fn new(dest_dir: ShadowCopyDestination) -> Self {
        let this = Self {
            dest_dir,
            pending_jobs: Default::default(),
            executing_job: Default::default(),
            completed_jobs: Default::default(),
            job_starter_clutch: Default::default(),
            job_added_signal: Default::default(),
            build_required: Default::default(),
        };

        // Start the JOB_EXECUTOR thread. This thread picks jobs off the front
        // of the queue and executes them one at a time.
        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        builder
            .spawn({
                let mut this = this.clone();
                move || this.execute_jobs()
            })
            .expect("Cannot create JOB_EXECUTOR thread");

        this
    }

    /// Pauses the job engine.
    /// This does not clear out the list of pending jobs, nor does it stop the
    /// currently executing job, if any. However, after that job has completed
    /// no new jobs will begin to execute.
    pub fn pause(&self) {
        info!("JobEngine paused");
        self.job_starter_clutch.pause_threads();
    }

    /// Restarts the job engine after a pause.
    pub fn restart(&self) {
        info!("JobEngine restarting");
        self.job_starter_clutch.release_threads();
    }

    /// Add a job to the end of the queue.
    pub fn add_job(&self, job: PendingJob) {
        // This lock won't block the caller much, because all other locks
        // on the `pending_jobs` are very short lived.
        let pending_jobs_guard = self.pending_jobs.lock().unwrap();
        self.add_job_inner(job, pending_jobs_guard);
    }

    fn execute_jobs(&mut self) {
        let dummy_mutex = Mutex::new(());

        loop {
            // If we are paused, wait until we are released.
            self.job_starter_clutch.wait_for_release();

            // Do we have a job to execute?
            if let Some(job) = self.get_next_job() {
                //                self.executing_job = Some(job.clone());
                // This is potentially time consuming, everything else in this
                // method should be fast (hence the locks will be released quickly).
                let completed_job = job.execute();
                //                self.executing_job = None;

                self.set_flags(&completed_job);
                let pending_jobs_lock = self.pending_jobs.lock().unwrap();
                let mut completed_jobs_lock = self.completed_jobs.lock().unwrap();

                let msg = format!(
                    "{} completed, there are now {} pending and {} completed jobs",
                    completed_job,
                    pending_jobs_lock.len(),
                    completed_jobs_lock.len() + 1
                );

                completed_jobs_lock.push_back(completed_job);
                drop(completed_jobs_lock);

                info!("{}", msg);

                if pending_jobs_lock.is_empty() {
                    if self.build_required.load(Ordering::SeqCst) {
                        self.add_build_job(pending_jobs_lock);
                    }
                }
            } else {
                // The idea here is that this will BLOCK and you are not allowed to touch the
                // data guarded by the MUTEX until the signal happens.
                let guard = dummy_mutex.lock().unwrap();
                let _ = self.job_added_signal.wait(guard).unwrap();
            }
        }
    }

    fn get_next_job(&self) -> Option<PendingJob> {
        let mut pending_jobs_guard = self.pending_jobs.lock().unwrap();
        pending_jobs_guard.pop_front()
    }

    /// Convenince method to add a new build job.
    /// TODO: In the future this might be more sophisticated, for example checking to see
    /// if there is an existing build job already in the pipeline and moving it to the end (if it's
    /// not already running, that is).
    fn add_build_job(&self, pending_jobs_guard: MutexGuard<VecDeque<PendingJob>>) {
        let job = BuildJob::new(self.dest_dir.clone(), BuildMode::Debug);
        self.add_job_inner(job, pending_jobs_guard);
    }

    fn add_job_inner(
        &self,
        job: PendingJob,
        mut pending_jobs_guard: MutexGuard<VecDeque<PendingJob>>,
    ) {
        info!(
            "{} added, there are now {} jobs in the pending queue",
            job,
            pending_jobs_guard.len() + 1
        );

        pending_jobs_guard.push_back(job);

        // Tell everybody listening (really it's just us with one thread) that there
        // is now a job in the pending queue.
        self.job_added_signal.notify_all();
    }

    fn set_build_required_flag(&self, value: bool) {
        self.build_required.store(value, Ordering::SeqCst);
    }

    /// Sets the various state flags based on the job and its completion status.
    fn set_flags(&self, job: &CompletedJob) {
        match (job.kind(), job.completion_status()) {
            (JobKind::ShadowCopy(_), crate::jobs::CompletionStatus::Ok) => {
                self.set_build_required_flag(true)
            }
            (JobKind::ShadowCopy(_), crate::jobs::CompletionStatus::Error(_)) => {
                self.set_build_required_flag(false)
            }

            (JobKind::FileSync(_), crate::jobs::CompletionStatus::Ok) => {
                self.set_build_required_flag(true)
            }
            (JobKind::FileSync(_), crate::jobs::CompletionStatus::Error(_)) => {}

            (JobKind::Build(_), crate::jobs::CompletionStatus::Ok) => {
                self.set_build_required_flag(false)
            }

            (JobKind::Build(_), crate::jobs::CompletionStatus::Error(_)) => {
                self.set_build_required_flag(false)
            }

            (_, CompletionStatus::Unknown) => {}
        }
    }
}
