use std::sync::{Arc, Condvar, Mutex};

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
}

#[derive(Debug, Clone, Default)]
pub struct ThreadClutch {
    inner: Arc<ThreadClutchInner>,
}

impl ThreadClutch {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn pause_thread(&self) {
        self.inner.pause_thread();
    }

    pub fn release_thread(&self) {
        self.inner.release_thread();
    }

    pub fn wait_for_release(&self) {
        self.inner.wait_for_release();
    }

    pub fn is_paused(&self) -> bool {
        self.inner.is_paused()
    }
}
