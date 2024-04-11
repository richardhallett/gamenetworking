use std::time::{Duration, Instant};

pub struct TickTimer {
    /// The interval at which ticks are generated
    pub tick_interval: Duration,
    /// A timer to track the time since the last tick
    pub timer: Instant,
    /// The current tick number
    pub current_tick: i32,
    /// The time available to generate ticks
    time_available: Duration,
}

impl TickTimer {
    pub fn new(tick_interval: Duration) -> Self {
        TickTimer {
            tick_interval,
            timer: Instant::now(),
            current_tick: 0,
            time_available: Duration::from_secs(0),
        }
    }

    pub fn tick(&mut self) -> Vec<i32> {
        // Frame time is the elapsed time since the last frame
        let frame_time = self.timer.elapsed();

        // Reset the timer
        self.timer = Instant::now();

        // We accumlate the time given to us by frame_time
        // This then allows us to track the ticks over multiple frames
        self.time_available += frame_time;

        // We then generate the ticks
        let mut ticks = Vec::new();
        while self.time_available >= self.tick_interval {
            // Every tick we can produces means we can reduce the accumulator
            // by the tick interval
            self.time_available -= self.tick_interval;

            ticks.push(self.current_tick);

            // We wrap the tick count to avoid overflow
            self.current_tick = self.current_tick.wrapping_add(1);
        }

        ticks
    }
}
