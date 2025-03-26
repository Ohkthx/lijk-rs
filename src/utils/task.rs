use std::time::{Duration, Instant};

/// A utility for managing time intervals in a loop.
pub struct Task {
    next: Instant,      // The next time the interval should be checked.
    duration: Duration, // The duration of the interval.
}

impl Task {
    /// Creates a new interval with the specified duration and an initial delay.
    pub fn start(duration: Duration, delay_ms: u64) -> Self {
        Self {
            next: Instant::now() + Duration::from_millis(delay_ms) + duration,
            duration,
        }
    }

    /// Executes the function if the current time is past the next scheduled time.
    #[allow(dead_code)]
    pub fn if_ready<F>(&mut self, mut f: F)
    where
        F: FnMut(),
    {
        if Instant::now() >= self.next {
            f();
            self.next = Instant::now() + self.duration;
        }
    }

    /// Checks if the current time is past the next scheduled time.
    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.next
    }

    /// Resets the interval to the current time plus the duration.
    pub fn reset(&mut self) {
        self.next = Instant::now() + self.duration;
    }
}
