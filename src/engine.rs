use crate::{
    jobs::{BuildJob, BuildMode, Job, JobKind, ShadowCopyJob, PendingJob, CompletedJob, ExecutingJob, CompletionStatus},
    shadow_copy_destination::ShadowCopyDestination,
    thread_clutch::ThreadClutch,
};
use log::info;
use std::collections::VecDeque;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Condvar, Mutex, MutexGuard, atomic::{Ordering, AtomicBool},
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

type JobList = Arc<Mutex<VecDeque<PendingJob>>>;

#[derive(Debug, Clone)]
pub struct JobEngine {
    dest_dir: ShadowCopyDestination,

    /// The list of pending (yet to be executed) jobs.
    pending_jobs: Arc<Mutex<VecDeque<PendingJob>>>,

    executing_job: Option<ExecutingJob>,

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

        // These channels are used to connect up the various threads.
        let (job_exec_sender, job_exec_internal_receiver) = channel::<PendingJob>();
        let (job_exec_internal_sender, job_exec_receiver) = channel::<CompletedJob>();

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

    fn add_job_inner(&self, job: PendingJob, mut pending_jobs_guard: MutexGuard<VecDeque<PendingJob>>) {
        info!(
            "Added {}, there are now {} jobs in the pending queue",
            job,
            pending_jobs_guard.len() + 1
        );

        pending_jobs_guard.push_back(job);

        // Tell everybody listening (really it's just us with one thread) that there
        // is now a job in the pending queue.
        self.job_added_signal.notify_all();
    }

    /// Add a job to the end of the queue.
    pub fn add_job(&self, job: PendingJob) {
        // This lock won't block the caller much, because all other locks
        // on the `pending_jobs` are very short lived.
        let pending_jobs_guard = self.pending_jobs.lock().unwrap();
        self.add_job_inner(job, pending_jobs_guard);
    }

    fn run_job_executor_thread(
        &self,
        job_exec_internal_receiver: Receiver<PendingJob>,
        job_exec_internal_sender: Sender<CompletedJob>,
    ) {
        for job in job_exec_internal_receiver {
            let job = job.execute();
            job_exec_internal_sender
                .send(job)
                .expect("Cannot return job from JOB_EXECUTOR");
        }
    }

    fn run_job_starter_thread(&self, job_exec_sender: Sender<PendingJob>) {
        let dummy_mutex = Mutex::new(());

        loop {
            self.job_starter_clutch.wait_for_release();

            if let Some(job) = self.get_next_job() {
                job_exec_sender
                    .send(job)
                    .expect("Could not send job to JOB_EXECUTOR thread");
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

    fn run_job_completed_thread(&self, job_exec_receiver: Receiver<CompletedJob>) {
        for job in job_exec_receiver {
            self.set_flags(&job);

            let pending_jobs_lock = self.pending_jobs.lock().unwrap();
            let mut completed_jobs_lock = self.completed_jobs.lock().unwrap();

            let msg = format!(
                "{} completed, there are now {} pending and {} completed jobs",
                job,
                pending_jobs_lock.len(),
                completed_jobs_lock.len() + 1
            );

            completed_jobs_lock.push_back(job);
            drop(completed_jobs_lock);

            info!("{}", msg);

            if pending_jobs_lock.is_empty() {
                if self.build_required.load(Ordering::SeqCst) {
                    self.add_build_job(pending_jobs_lock);
                }
            }
        }
    }

    /// Convenince method to add a new build job.
    /// TODO: In the future this might be more sophisticated, for example checking to see
    /// if there is an existing build job already in the pipeline and moving it to the end (if it's
    /// not already running, that is).
    fn add_build_job(&self, pending_jobs_guard: MutexGuard<VecDeque<PendingJob>>) {
        let job = BuildJob::new(self.dest_dir.clone(), BuildMode::Debug);
        self.add_job_inner(job, pending_jobs_guard);
    }

    fn set_build_required_flag(&self, value: bool) {
        self.build_required.store(value, Ordering::SeqCst);
    }

    /// Sets the various state flags based on the job and its completion status.
    fn set_flags(&self, job: &CompletedJob) {
        match (job.kind(), job.completion_status()) {
            (JobKind::ShadowCopy(_), crate::jobs::CompletionStatus::Ok) => self.set_build_required_flag(true),
            (JobKind::ShadowCopy(_), crate::jobs::CompletionStatus::Error(_)) => self.set_build_required_flag(false),

            (JobKind::FileSync(_), crate::jobs::CompletionStatus::Ok) => self.set_build_required_flag(true),
            (JobKind::FileSync(_), crate::jobs::CompletionStatus::Error(_)) => {}

            (JobKind::Build(_), crate::jobs::CompletionStatus::Ok) => self.set_build_required_flag(false),
            // TODO: This is a problem. Will rebuild infinitely.
            (JobKind::Build(_), crate::jobs::CompletionStatus::Error(_)) => self.set_build_required_flag(true),

            (_, CompletionStatus::Unknown) => {}
        }
    }
}
