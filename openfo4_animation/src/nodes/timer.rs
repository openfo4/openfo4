use std::time::Duration;

use crate::Event;

use super::{Context, GraphNode};

#[derive(Clone, Debug)]
pub struct Timer {
    pub min_time: Duration,
    pub max_time: Duration,
    pub event: Event,
    pub repeat: bool,
    pub eval_duration: Duration,
    pub elapsed: Duration,
    pub is_running: bool,
}

impl Timer {
    pub fn new_uniform(time: Duration, event: Event) -> Self {
        Self {
            min_time: time,
            max_time: time,
            event,
            repeat: false,
            eval_duration: Duration::ZERO,
            elapsed: Duration::ZERO,
            is_running: false,
        }
    }
}

impl GraphNode for Timer {
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.eval_duration = rand::random_range(self.min_time..self.max_time);
        self.is_running = true;
    }

    fn deactivate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.is_running = false;
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        self.elapsed += ctx.delta();
        if self.elapsed > self.eval_duration {
            ctx.send_event(self.event.clone());
        }
    }
}
