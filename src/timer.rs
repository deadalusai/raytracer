use std::time::{Duration, Instant};

pub struct Timer {
    start: Instant,
    timer_duration: Duration,
}

impl Timer {
    pub fn new(timer_duration: Duration) -> Self {
        Self { start: Instant::now(), timer_duration }
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    pub fn tick(&mut self) -> bool {
        let expired = self.start.elapsed() >= self.timer_duration;
        if expired {
            self.reset();
        }
        expired
    }
}
