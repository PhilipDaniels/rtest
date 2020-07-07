use crate::{
    jobs::{BuildJob, BuildMode, Job, JobKind},
    shadow_copy_destination::ShadowCopyDestination,
    thread_clutch::ThreadClutch,
};
use log::info;
use std::collections::VecDeque;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Condvar, Mutex, MutexGuard,
};
use std::thread;

type JobList = Arc<Mutex<VecDeque<Job>>>;

#[derive(Debug, Clone)]
pub struct JobEngine {
    dest_dir: ShadowCopyDestination,
    /// The list of pending (yet to be executed) jobs.
    pending_jobs: JobList,

    /// The list of completed jobs.
    completed_jobs: JobList,

    /// A clutch that allows us to pause and restart the JOB_STARTER thread.
    /// This basically allows us to pause the entire job queue, because if we
    /// don't start to execute new jobs, nothing happens. Yet we can still
    /// add new jobs to the queue, because that is controlled by a different thread.
    job_starter_clutch: ThreadClutch,

    /// The `job_added_signal` is notified when a new job is added to the pending queue.
    /// This will cause the JOB_STARTER thread to wake up (it goes to sleep when
    /// there are no pending jobs).
    job_added_signal: Arc<Condvar>,

    engine_state: Arc<Mutex<EngineState>>,
}

impl JobEngine {
    /// Creates a new job engine that is running and ready to process jobs.
    pub fn new(dest_dir: ShadowCopyDestination) -> Self {
        let this = Self {
            dest_dir,
            pending_jobs: Default::default(),
            completed_jobs: Default::default(),
            job_starter_clutch: Default::default(),
            job_added_signal: Default::default(),
            engine_state: Arc::new(Mutex::new(EngineState::WaitingOk)),
        };

        // These channels are used to connect up the various threads.
        let (job_exec_sender, job_exec_internal_receiver) = channel::<Job>();
        let (job_exec_internal_sender, job_exec_receiver) = channel::<Job>();

        // Create the JOB_EXECUTOR thread. This thread just calls
        // `Job.execute()`, one job at a time. It receives jobs on a channel, and sends
        // the results back on another channel (where they are picked up by
        // the JOB_COMPLETED thread).
        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());
        builder
            .spawn({
                let this = this.clone();
                move || {
                    this.run_job_executor_thread(
                        job_exec_internal_receiver,
                        job_exec_internal_sender,
                    )
                }
            })
            .expect("Cannot create JOB_EXECUTOR thread");

        // Create the JOB_STARTER thread. This thread is responsible for checking the
        // `pending_jobs` queue to see if there are any jobs that need executing, and if
        // there are it clones them and sends them to the JOB_EXECUTOR thread.
        // If there are no pending jobs then it goes to sleep, until it is woken up by
        // `add_job` notifying the signal.
        // Q: Why not use a channel with a for loop like the other threads, since that would
        // allow us to dispense with the Condvar? A: We want the list of pending jobs to
        // be observable / iterable, so we must maintain our own queue, we can't use the
        // channels queue because iterating it consumes the items.
        let builder = thread::Builder::new().name("JOB_STARTER".into());
        builder
            .spawn({
                let this = this.clone();
                move || this.run_job_starter_thread(job_exec_sender)
            })
            .expect("Cannot create JOB_STARTER thread");

        // Create the JOB_COMPLETED thread. It is the job of this thread to listen
        // for completed job messages which are sent by the JOB_EXECUTOR thread.
        let builder = thread::Builder::new().name("JOB_COMPLETED".into());
        builder
            .spawn({
                let this = this.clone();
                move || this.run_job_completed_thread(job_exec_receiver)
            })
            .expect("Cannot create JOB_COMPLETED thread");

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
    pub fn add_job(&self, job: Job) {
        // This lock won't block the caller much, because all other locks
        // on the `pending_jobs` are very short lived.
        let mut pending_jobs_lock = self.pending_jobs.lock().unwrap();

        info!(
            "Added {}, there are now {} jobs in the pending queue",
            job,
            pending_jobs_lock.len() + 1
        );

        pending_jobs_lock.push_back(job);

        // Tell everybody listening (really it's just us with one thread) that there
        // is now a job in the pending queue.
        self.job_added_signal.notify_all();
    }

    fn run_job_executor_thread(
        &self,
        job_exec_internal_receiver: Receiver<Job>,
        job_exec_internal_sender: Sender<Job>,
    ) {
        for mut job in job_exec_internal_receiver {
            // TODO: Tidy up the JobState management.
            job.execute();
            job_exec_internal_sender
                .send(job)
                .expect("Cannot return job from JOB_EXECUTOR");
        }
    }

    fn run_job_starter_thread(&self, job_exec_sender: Sender<Job>) {
        let dummy_mutex = Mutex::new(());

        loop {
            self.job_starter_clutch.wait_for_release();

            if let Some(job) = self.get_next_job() {
                job_exec_sender
                    .send(job)
                    .expect("Could not send job to JOB_EXECUTOR thread");
            } else {
                let mut engine_state_lock = self.engine_state.lock().unwrap();
                let more_jobs = self.required_jobs(&engine_state_lock);

                if more_jobs.is_empty() {
                    // No jobs exist, go to sleep waiting for a signal on the condition variable.
                    // This will be signaled by `add_job`.

                    // The idea here is that this will BLOCK and you are not allowed to touch the
                    // data guarded by the MUTEX until the signal happens.
                    *engine_state_lock = EngineState::WaitingOk;
                    let guard = dummy_mutex.lock().unwrap();
                    let _ = self.job_added_signal.wait(guard).unwrap();
                } else {
                    // We got into a state which means more jobs are required.
                    // Add them to the queue.
                    for job in more_jobs {
                        self.add_job(job);
                    }

                    *engine_state_lock = EngineState::Working;
                }
            }
        }
    }

    fn get_next_job(&self) -> Option<Job> {
        let mut pending_jobs_lock = self.pending_jobs.lock().unwrap();
        for job in pending_jobs_lock.iter_mut() {
            if job.is_pending() {
                // Mark the job while it remains in the queue, so that we
                // skip over it the next time.
                job.begin_execution();
                return Some(job.clone());
            }
        }

        None
    }

    fn required_jobs(&self, engine_state_lock: &MutexGuard<EngineState>) -> Vec<Job> {
        match **engine_state_lock {
            EngineState::BuildRequired => {
                vec![BuildJob::new(self.dest_dir.clone(), BuildMode::Debug)]
            }
            EngineState::TestRunRequired => vec![],
            EngineState::LastBuildFailed => vec![],
            EngineState::WaitingOk => vec![],
            EngineState::ShadowCopyFailed => vec![],
            EngineState::Working => vec![],
        }
    }

    fn run_job_completed_thread(&self, job_exec_receiver: Receiver<Job>) {
        for job in job_exec_receiver {
            // TODO
            // let mut engine_state_lock = engine_state.lock().unwrap();
            // engine_state_lock.job_completed(&job);
            // drop(engine_state_lock);

            let mut pending_jobs_lock = self.pending_jobs.lock().unwrap();

            // Find this job by id in the list of pending jobs. It may not be there, if we
            // 'tweaked' the job queue while this one was executing. But if we do
            // find it, then remove it and add it to the list of completed jobs.
            // If it's not found, just ignore it.
            if let Some(index) = pending_jobs_lock.iter().position(|j| j.id() == job.id()) {
                pending_jobs_lock.remove(index);
                let pj_len = pending_jobs_lock.len();
                // Release lock ASAP.
                drop(pending_jobs_lock);

                let mut completed_jobs_lock = self.completed_jobs.lock().unwrap();
                let msg = format!(
                    "Completed {}, there are now {} pending and {} completed jobs",
                    job,
                    pj_len,
                    completed_jobs_lock.len() + 1
                );
                completed_jobs_lock.push_back(job);
                drop(completed_jobs_lock);

                info!("{}", msg);
            }
        }
    }
}

/// Represents the state of the engine, based on what jobs have completed
/// and/or are pending.
#[derive(Debug, Copy, Clone)]
enum EngineState {
    /// A build is required.
    BuildRequired,

    /// A test-run is required.
    TestRunRequired,

    /// The last build failed. We are stuck here until we
    /// get new files to copy, then we can run a build again
    /// in the hope that it fixes it.
    LastBuildFailed,

    /// Waiting. All jobs have been run and we finished building
    /// and running tests.
    WaitingOk,

    ShadowCopyFailed,
    Working,
}

impl EngineState {
    fn job_completed(&mut self, job: &Job) {
        match job.kind() {
            JobKind::ShadowCopy(shadow_copy_job) => {
                if shadow_copy_job.succeeded() {
                    *self = Self::BuildRequired;
                } else {
                    *self = Self::ShadowCopyFailed;
                }
            }

            JobKind::FileSync(file_sync_job) => {
                if file_sync_job.succeeded() {
                    *self = Self::BuildRequired;
                }
            }

            JobKind::Build(build_job) => {
                if build_job.succeeded() {
                    *self = Self::TestRunRequired;
                } else {
                    *self = Self::LastBuildFailed;
                }
            }
        }
    }

    /// Called when a file copy has been successfully completed.
    /// Changes the state to `BuildRequired`.
    fn file_copy_completed(&mut self) {
        *self = Self::BuildRequired;
    }
}

/*
We need the following

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
