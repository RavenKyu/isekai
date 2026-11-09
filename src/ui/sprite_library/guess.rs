use crate::raster::{self, GridGuess, RgbaImage};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

/// Owns only CPU data. Dropping a job signals cancellation and never joins the
/// detached worker; no GL object or editor reference crosses the thread boundary.
pub(super) struct GuessJob {
    cancel: Arc<AtomicBool>,
    done: Arc<AtomicU64>,
    total: u64,
    result: Receiver<GridGuess>,
}
impl GuessJob {
    pub fn start(image: RgbaImage) -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicU64::new(0));
        let total = raster::guess_grid_work(&image, &raster::DEFAULT_GUESS_CANDIDATES, true);
        let (sender, result) = mpsc::channel();
        let worker_cancel = Arc::clone(&cancel);
        let worker_done = Arc::clone(&done);
        std::thread::Builder::new()
            .name("sprite-grid-guess".into())
            .spawn(move || {
                if let Some(guess) = raster::guess_grid_with(
                    &image,
                    &raster::DEFAULT_GUESS_CANDIDATES,
                    true,
                    &|done, _| worker_done.store(done, Ordering::Relaxed),
                    &worker_cancel,
                ) {
                    let _ = sender.send(guess);
                }
            })
            .map_err(|e| format!("Cannot start Guess: {e}"))?;
        Ok(Self {
            cancel,
            done,
            total,
            result,
        })
    }
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            (self.done.load(Ordering::Relaxed) as f64 / self.total as f64).clamp(0.0, 1.0) as f32
        }
    }
    pub fn poll(&self) -> Result<Option<GridGuess>, String> {
        match self.result.try_recv() {
            Ok(guess) => Ok(Some(guess)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("Guess worker stopped without a result".into()),
        }
    }
}
impl Drop for GuessJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn worker_reports_the_same_result_and_full_progress() {
        let mut image = RgbaImage::new(32, 32);
        image.fill_rect(0, 0, 32, 32, [1, 2, 3, 255]);
        let expected = raster::guess_grid(&image, &raster::DEFAULT_GUESS_CANDIDATES, true);
        let job = GuessJob::start(image).unwrap();
        assert_eq!(
            job.result.recv_timeout(Duration::from_secs(2)).unwrap(),
            expected
        );
        assert_eq!(job.fraction(), 1.0);
    }
    #[test]
    fn a_running_worker_observes_cancellation_within_a_second() {
        let job = GuessJob::start(RgbaImage::new(512, 512)).unwrap();
        job.cancel.store(true, Ordering::Relaxed);
        assert!(matches!(
            job.result.recv_timeout(Duration::from_secs(1)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ));
    }
    #[test]
    fn dropping_a_running_job_signals_cancellation_without_waiting() {
        let job = GuessJob::start(RgbaImage::new(512, 512)).unwrap();
        let flag = Arc::clone(&job.cancel);
        let started = std::time::Instant::now();
        drop(job);
        assert!(flag.load(Ordering::Relaxed));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
