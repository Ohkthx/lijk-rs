use std::time::{Duration, Instant};

/// Controls loop ticks in increments to maintain TPS.
pub struct Timestep {
    pub last_ts: Instant,    // Last timestamp processed.
    pub tick: u64,           // Current tick count.
    tick_duration: Duration, // Duration of each tick.

    // For FPS tracking.
    fps: f32,              // Current frames-per-second (FPS).
    frame_count: f32,      // Number of frames since last FPS update.
    accumulator: Duration, // Accumulator for time since last FPS update.
}

impl Timestep {
    /// Create a Timestep with a desired ticks-per-second (`tick_rate`).
    pub fn new(tick_rate: f32) -> Self {
        Self {
            last_ts: Instant::now(),
            tick: 0,
            tick_duration: Duration::from_secs_f32(1.0 / tick_rate),

            fps: 0.0,
            frame_count: 0.0,
            accumulator: Duration::default(),
        }
    }

    /// Returns the most recently computed frames-per-second (FPS).
    #[allow(dead_code)]
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Blocks until the next tick is due, and updates the tick count. Returns the amount of ticks behind.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn wait(&mut self) -> u32 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_ts);

        // Check if we're behind schedule.
        let behind = if elapsed < self.tick_duration {
            let remainder = self.tick_duration - elapsed;
            std::thread::sleep(remainder);
            0
        } else {
            let passed = (elapsed.as_secs_f32() / self.tick_duration.as_secs_f32()).floor() as u32;
            passed.saturating_sub(1)
        };

        // Keep our tick and timestamp up to date.
        let update_ts = Instant::now();
        let real_elapsed = update_ts.duration_since(self.last_ts);
        self.last_ts = update_ts;
        self.tick += 1;

        // Update our FPS counters.
        self.frame_count += 1.0;
        self.accumulator += real_elapsed;

        // Every time we accumulate >= 1 second, we compute an FPS snapshot.
        if self.accumulator.as_secs_f32() >= 1.0 {
            self.fps = self.frame_count / self.accumulator.as_secs_f32();
            self.frame_count = 0.0;
            self.accumulator = Duration::default();
        }

        behind
    }
}
