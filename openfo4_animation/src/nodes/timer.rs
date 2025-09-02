use std::time::Duration;

use crate::{Event, ValueExpression};

use super::{Context, GraphNode};

#[derive(Clone, Debug)]
pub struct Timer {
    pub condition: ValueExpression,
    pub min_time: ValueExpression,
    pub max_time: ValueExpression,
    pub event: Event,
    pub repeat: bool,
    pub eval_duration: Duration,
    pub elapsed: Duration,
    pub is_running: bool,
}

impl GraphNode for Timer {
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        let min_time =
            Duration::from_secs_f32(self.min_time.eval(ctx.variables()).to_f32().unwrap());
        let max_time =
            Duration::from_secs_f32(self.max_time.eval(ctx.variables()).to_f32().unwrap());

        self.eval_duration = rand::random_range(min_time..=max_time);
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
        if !self.condition.eval(ctx.variables()).to_bool().unwrap() {
            return;
        }

        self.elapsed += ctx.delta();
        if self.elapsed > self.eval_duration {
            ctx.send_event(self.event.clone());
        }
    }
}
