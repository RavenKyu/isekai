use super::job::Job;
use crate::raster::{self, GridGuess, RgbaImage};
pub(super) fn start(image: RgbaImage) -> Result<Job<GridGuess>, String> {
    let total = raster::guess_grid_work(&image, &raster::DEFAULT_GUESS_CANDIDATES, true);
    Job::start(
        "sprite-grid-guess",
        "Guess",
        total,
        move |progress, cancel| {
            raster::guess_grid_with(
                &image,
                &raster::DEFAULT_GUESS_CANDIDATES,
                true,
                progress,
                cancel,
            )
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, atomic::Ordering, mpsc},
        time::Duration,
    };

    #[test]
    fn worker_reports_the_same_result_and_full_progress() {
        let mut image = RgbaImage::new(32, 32);
        image.fill_rect(0, 0, 32, 32, [1, 2, 3, 255]);
        let expected = raster::guess_grid(&image, &raster::DEFAULT_GUESS_CANDIDATES, true);
        let job = start(image).unwrap();
        assert_eq!(
            job.result.recv_timeout(Duration::from_secs(2)).unwrap(),
            expected
        );
        assert_eq!(job.fraction(), 1.0);
    }
    #[test]
    fn a_running_worker_observes_cancellation_within_a_second() {
        let job = start(RgbaImage::new(512, 512)).unwrap();
        job.cancel.store(true, Ordering::Relaxed);
        assert!(matches!(
            job.result.recv_timeout(Duration::from_secs(1)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ));
    }
    #[test]
    fn dropping_a_running_job_signals_cancellation_without_waiting() {
        let job = start(RgbaImage::new(512, 512)).unwrap();
        let flag = Arc::clone(&job.cancel);
        let started = std::time::Instant::now();
        drop(job);
        assert!(flag.load(Ordering::Relaxed));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
