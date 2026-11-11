use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

/// Owns only CPU data; dropping signals cancellation without joining the worker.
pub(super) struct Job<T> {
    pub(super) cancel: Arc<AtomicBool>,
    done: Arc<AtomicU64>,
    total: u64,
    pub(super) result: Receiver<T>,
    label: &'static str,
    logged: u32,
}
impl<T: Send + 'static> Job<T> {
    pub fn start(
        name: &str,
        label: &'static str,
        total: u64,
        work: impl FnOnce(&dyn Fn(u64, u64), &AtomicBool) -> Option<T> + Send + 'static,
    ) -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicU64::new(0));
        let (sender, result) = mpsc::channel();
        let worker_cancel = Arc::clone(&cancel);
        let worker_done = Arc::clone(&done);
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                if let Some(value) = work(
                    &|done, _| worker_done.store(done, Ordering::Relaxed),
                    &worker_cancel,
                ) {
                    let _ = sender.send(value);
                }
            })
            .map_err(|e| format!("Cannot start {label}: {e}"))?;
        Ok(Self {
            cancel,
            done,
            total,
            result,
            label,
            logged: 0,
        })
    }
    /// The next 10% step to log while running, once each; completion logs 100%.
    pub fn next_step(&mut self) -> Option<u32> {
        let percent = (self.fraction() * 100.0) as u32 / 10 * 10;
        if percent > self.logged && percent < 100 {
            self.logged = percent;
            Some(percent)
        } else {
            None
        }
    }
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            (self.done.load(Ordering::Relaxed) as f64 / self.total as f64).clamp(0.0, 1.0) as f32
        }
    }
    pub fn poll(&self) -> Result<Option<T>, String> {
        match self.result.try_recv() {
            Ok(value) => Ok(Some(value)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err(format!("{} worker stopped without a result", self.label))
            }
        }
    }
}
impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
