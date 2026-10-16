use std::time::{Duration, Instant};

use async_io::Timer;
use game_tracing::trace_span;

#[derive(Clone, Debug)]
pub struct Interval {
    last_update: Instant,
    /// The uniform timestep duration of a control frame.
    timestep: Duration,
}

impl Interval {
    pub fn new(timestep: Duration) -> Self {
        Self {
            last_update: Instant::now(),
            timestep,
        }
    }

    /// Updates the timestep value without resetting the update timestamp.
    pub fn set_timestep(&mut self, timestep: Duration) {
        self.timestep = timestep;
    }

    /// Returns the timestep value used to create this `Interval`.
    pub fn timestep(&self) -> Duration {
        self.timestep
    }

    pub fn is_ready(&mut self, now: Instant) -> bool {
        let elapsed = now - self.last_update;

        if elapsed >= self.timestep {
            self.last_update += self.timestep;
            true
        } else {
            false
        }
    }

    /// Sleep until the next tick.
    pub async fn wait(&mut self, now: Instant) {
        let _span = trace_span!("Interval::wait").entered();

        // The timer may already be delayed which means we need
        // to yield immediately.
        let elapsed = now - self.last_update;
        if elapsed >= self.timestep {
            self.last_update += self.timestep;
            return;
        }

        // FIXME: This will likely break terribly on windows for <15ms sleep
        // times. We should probably timeBeginPeriod/timeBeginEndPeriod and spin
        // with PAUSEs for small periods.
        // Linux timers are accurate enough (~50us) that we don't really have to
        // bother with it.
        let duration = self.timestep - elapsed;
        tracing::trace!("Interval::wait({:?})", duration);
        Timer::after(duration).await;
        self.last_update += self.timestep;
    }
}
