use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Loop,
    Once,
    PingPong,
}

#[derive(Clone, Debug)]
pub struct Timeline {
    starts: Vec<u64>,
    total: u64,
    mode: Mode,
    order: Vec<usize>,
}
impl Timeline {
    pub fn new(durations_ms: Vec<u32>, mode: Mode) -> Result<Self, String> {
        if durations_ms.is_empty() || durations_ms.contains(&0) {
            return Err("timeline needs frames with ms >= 1".into());
        }
        let mut order: Vec<usize> = (0..durations_ms.len()).collect();
        if mode == Mode::PingPong && durations_ms.len() > 2 {
            order.extend((1..durations_ms.len() - 1).rev());
        }
        let mut total = 0u64;
        let mut starts = Vec::with_capacity(order.len());
        for &index in &order {
            starts.push(total);
            total = total
                .checked_add(u64::from(durations_ms[index]))
                .ok_or("timeline duration overflow")?;
        }
        Ok(Self {
            starts,
            total,
            mode,
            order,
        })
    }
    pub fn frame_at(&self, elapsed_ms: u64) -> usize {
        let elapsed = if self.mode == Mode::Once {
            elapsed_ms.min(self.total - 1)
        } else {
            elapsed_ms % self.total
        };
        self.order[self.starts.partition_point(|&start| start <= elapsed) - 1]
    }
    pub fn finished_at(&self, elapsed_ms: u64) -> bool {
        self.mode == Mode::Once && elapsed_ms >= self.total
    }
    pub fn total_ms(&self) -> u64 {
        self.total
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundaries_and_large_elapsed_time() {
        let timeline = Timeline::new(vec![100; 3], Mode::Loop).unwrap();
        for (ms, frame) in [
            (0, 0),
            (99, 0),
            (100, 1),
            (299, 2),
            (300, 0),
            (1_000_000_000_150, 2),
        ] {
            assert_eq!(timeline.frame_at(ms), frame);
        }
        let timeline = Timeline::new(vec![100; 3], Mode::Once).unwrap();
        assert_eq!(timeline.frame_at(299), 2);
        assert_eq!(timeline.frame_at(u64::MAX), 2);
        assert!(!timeline.finished_at(299));
        assert!(timeline.finished_at(300));
        let timeline = Timeline::new(vec![10, 20, 30], Mode::PingPong).unwrap();
        assert_eq!(timeline.total_ms(), 80);
        for (ms, frame) in [(0, 0), (10, 1), (30, 2), (60, 1), (79, 1), (80, 0)] {
            assert_eq!(timeline.frame_at(ms), frame);
        }
        for mode in [Mode::Loop, Mode::Once, Mode::PingPong] {
            assert!(Timeline::new(vec![], mode).is_err());
            assert!(Timeline::new(vec![0], mode).is_err());
            assert_eq!(Timeline::new(vec![10], mode).unwrap().frame_at(u64::MAX), 0);
        }
        let two = Timeline::new(vec![10, 20], Mode::PingPong).unwrap();
        assert_eq!(two.total_ms(), 30);
        assert_eq!(two.frame_at(30), 0);
        assert_eq!(
            Timeline::new(vec![u32::MAX; 3], Mode::Loop)
                .unwrap()
                .total_ms(),
            u64::from(u32::MAX) * 3
        );
    }
}
