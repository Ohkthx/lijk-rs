use std::time::{Duration, Instant};

/// Controls loop ticks in increments to maintain TPS.
pub struct Timestep {
    pub last_ts: Instant,    // Last timestamp processed.
    tick: u64,               // Current tick count.
    tick_duration: Duration, // Duration of each tick.
}

impl Timestep {
    /// Create a Timestep with a desired ticks-per-second (`tick_rate`).
    pub fn new(tick_rate: f32) -> Self {
        Self {
            last_ts: Instant::now(),
            tick: 0,
            tick_duration: Duration::from_secs_f32(1.0 / tick_rate),
        }
    }

    /// Returns the fixed delta time in seconds.
    #[inline]
    pub fn fixed_dt(&self) -> f32 {
        self.tick_duration.as_secs_f32()
    }

    /// Returns the current tick count.
    #[inline]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Blocks until the next tick is due, and updates the tick count.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn wait(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_ts);

        // Check if we're behind schedule.
        if elapsed < self.tick_duration {
            std::thread::sleep(self.tick_duration - elapsed);
        }

        // Keep our tick and timestamp up to date.
        self.last_ts = Instant::now();
        self.tick += 1;
    }
}
