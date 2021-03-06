use crate::{
    configuration::{BuildMode, Configuration},
    jobs::{
        BuildAllTestsJob, CompletedJob, CompletionStatus, Job, JobKind, ListAllTestsJob, PendingJob,
        RunTestsJob,
    },
    thread_clutch::ThreadClutch, state::State,
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

Algorithm for adding file sync
FOR SOME PATH P
If a build is running, stop it
If OP is REMOVE, remove all file copy jobs and create a remove job
ELSE
    if there is a previous job for this file, remove it and insert a new COPY job (op is likely to be WRITE, CLOSE_WRITE, RENAME or CHMOD)

*/

#[derive(Clone)]
pub struct JobEngine {
    configuration: Configuration,
    state: State,

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

    build_tests_required: BoolFlag,
    list_tests_required: BoolFlag,
    run_tests_required: BoolFlag,
}

impl JobEngine {
    /// Creates a new job engine that is running and ready to process jobs.
    pub fn new(configuration: Configuration, state: State) -> Self {
        let this = Self {
            configuration,
            state,
            pending_jobs: Default::default(),
            executing_job: Default::default(),
            completed_jobs: Default::default(),
            job_starter_clutch: Default::default(),
            job_added_signal: Default::default(),
            build_tests_required: Default::default(),
            list_tests_required: Default::default(),
            run_tests_required: Default::default(),
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
                let mut executing_job_guard = self.executing_job.lock().unwrap();
                *executing_job_guard = Some(job.clone());
                // This is potentially time consuming, everything else in this
                // method should be fast (hence the locks will be released quickly).
                let completed_job = job.execute();

                let kind = completed_job.kind();
                match kind {
                    JobKind::ShadowCopy(_) => {}
                    JobKind::FileSync(_) => {}
                    JobKind::BuildAllTests(_) => {}
                    JobKind::BuildWorkspace(_) => {}
                    JobKind::ListAllTests(kind) => {
                        let tests = kind.parse_tests().unwrap();
                        self.state.update_test_list(&tests);
                    }
                    JobKind::RunTests(_) => {}
                }

                self.set_engine_state_flags(&completed_job);
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

                *executing_job_guard = None;
                drop(executing_job_guard);

                info!("{}", msg);

                let build_mode = match self.configuration.build_mode() {
                    crate::configuration::CompilationMode::None => BuildMode::Debug,
                    crate::configuration::CompilationMode::Debug => BuildMode::Debug,
                    crate::configuration::CompilationMode::Release => BuildMode::Release,
                    crate::configuration::CompilationMode::Both => BuildMode::Debug,
                };

                if pending_jobs_lock.is_empty() {
                    if self.build_tests_required.is_true() {
                        let job =
                            BuildAllTestsJob::new(self.configuration.destination.clone(), build_mode);
                        self.add_job_inner(job, pending_jobs_lock);
                    } else if self.list_tests_required.is_true() {
                        let job =
                            ListAllTestsJob::new(self.configuration.destination.clone(), build_mode);
                        self.add_job_inner(job, pending_jobs_lock);
                    } else if self.run_tests_required.is_true() {
                        let job =
                            RunTestsJob::new(self.configuration.destination.clone(), build_mode);
                        self.add_job_inner(job, pending_jobs_lock);
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
    // fn add_build_job(&self, pending_jobs_guard: MutexGuard<VecDeque<PendingJob>>) {
    //     let job = BuildJob::new(self.dest_dir.clone(), BuildMode::Debug);
    //     self.add_job_inner(job, pending_jobs_guard);
    // }

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

    /// Sets the various state flags based on the job and its completion status.
    fn set_engine_state_flags(&self, job: &CompletedJob) {
        match (job.kind(), job.completion_status()) {
            (JobKind::ShadowCopy(_), CompletionStatus::Ok) => {
                self.build_tests_required.set_true();
            }
            (JobKind::ShadowCopy(_), CompletionStatus::Error(_)) => {
                self.build_tests_required.set_false();
            }

            (JobKind::FileSync(_), CompletionStatus::Ok) => {
                self.build_tests_required.set_true();
            }
            (JobKind::FileSync(_), CompletionStatus::Error(_)) => {}

            (JobKind::BuildAllTests(_), CompletionStatus::Ok) => {
                self.build_tests_required.set_false();
                self.list_tests_required.set_true();
            }
            (JobKind::BuildAllTests(_), CompletionStatus::Error(_)) => {
                // To prevent recursion, we need to wait till we get another file copy.
                self.build_tests_required.set_false();
            }

            // Having a built crate available is just a convenience. It doesn't affect
            // the main flow of build tests -> list tests -> run tests.
            (JobKind::BuildWorkspace(_), CompletionStatus::Ok) => {}
            (JobKind::BuildWorkspace(_), CompletionStatus::Error(_)) => {}

            (JobKind::ListAllTests(_), CompletionStatus::Ok) => {
                self.list_tests_required.set_false();
                self.run_tests_required.set_true();
            }
            (JobKind::ListAllTests(_), CompletionStatus::Error(_)) => {
                // To prevent recursion, we need to wait till we get another file copy.
                self.list_tests_required.set_false();
            }

            (JobKind::RunTests(_), CompletionStatus::Ok) => {
                self.run_tests_required.set_false();
            }
            (JobKind::RunTests(_), CompletionStatus::Error(_)) => {
                // To prevent recursion, we need to wait till we get another file copy.
                self.run_tests_required.set_false();
            }

            (_, CompletionStatus::Unknown) => {}
        }
    }
}

/// Atomic reference counted bool flag.
/// It is safe to use and call this from multiple threads.
#[derive(Debug, Default, Clone)]
struct BoolFlag {
    flag: Arc<AtomicBool>,
}

impl BoolFlag {
    fn is_true(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    fn is_false(&self) -> bool {
        !self.is_true()
    }

    fn get(&self, value: bool) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    fn set(&self, value: bool) {
        self.flag.store(value, Ordering::SeqCst);
    }

    fn set_true(&self) {
        self.set(true);
    }

    fn set_false(&self) {
        self.set(false);
    }
}
