use std::sync::{Arc, Condvar, Mutex};

/// The `ThreadClutch` provides a way to pause and release threads from other threads.
/// We talk about *controlled threads* - these are threads which have called `wait_for_release`
/// and are possibly blocked by that call. We also have *controlling threads* - these are
/// threads which call `pause_threads` and `release_threads` to control the execution of
/// the other threads. A typical pattern is to have 1 control threads and 1 or more
/// controlled threads, but there is nothing stopping you having multiple control threads
/// and multiple controlled threads.
///
/// `ThreadCluth` is `Send` and clonable, a clone should be passed into each controlled thread.
#[derive(Debug, Clone, Default)]
pub struct ThreadClutch {
    inner: Arc<ThreadClutchInner>,
}

impl ThreadClutch {
    /// Creates a new `ThreadClutch`, with the controlled thread(s)
    /// in the running state.
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a new `ThreadClutch`, with the controlled thread(s)
    /// in the paused state.
    pub fn new_paused() -> Self {
        Self {
            inner: ThreadClutchInner::new_paused();
        }
    }

    /// Pauses all threads that are controlled by this `ThreadClutch`.
    pub fn pause_threads(&self) {
        self.inner.pause_thread();
    }

    /// Releases all threads that are controlled by this `ThreadClutch`.
    pub fn release_threads(&self) {
        self.inner.release_thread();
    }

    /// Waits for the thread to be allowed to run. Call this from one or more
    /// *controlled threads*. In the *controlling thread*, call `release_threads`
    /// to unblock the waiting threads.
    pub fn wait_for_release(&self) {
        self.inner.wait_for_release();
    }

    /// Returns true if the controlled threads are paused, waiting
    /// on this clutch.
    pub fn is_paused(&self) -> bool {
        self.inner.is_paused()
    }

    /// Returns true if the controlled threads are running.
    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }
}

#[derive(Debug, Default)]
struct ThreadClutchInner {
    paused: Mutex<bool>,
    condvar: Condvar,
}

impl ThreadClutchInner {
    pub fn new() -> Self {
        Self {
            paused: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    pub fn new_paused() -> Self {
        Self {
            paused: Mutex::new(true),
            condvar: Condvar::new(),
        }
    }

    pub fn pause_thread(&self) {
        let mut paused = self.paused.lock().unwrap();
        *paused = true;
    }

    pub fn release_thread(&self) {
        let mut paused = self.paused.lock().unwrap();
        *paused = false;
        self.condvar.notify_all();
    }

    pub fn wait_for_release(&self) {
        let mut paused = self.paused.lock().unwrap();
        while *paused {
            paused = self.condvar.wait(paused).unwrap();
        }
    }

    pub fn is_paused(&self) -> bool {
        *self.paused.lock().unwrap()
    }

    pub fn is_running(&self) -> bool {
        !*self.paused.lock().unwrap()
    }
}