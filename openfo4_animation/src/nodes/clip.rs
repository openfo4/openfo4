use std::time::Duration;

use crate::animation::AnimationClipFlags;
use crate::nodes::{Context, GraphNode};
use crate::{ClipTrigger, ValueExpression};

#[derive(Clone, Debug)]
pub struct Clip {
    pub path: String,
    pub triggers: Vec<ClipTrigger>,
    pub looping: bool,
    pub speed: ValueExpression,
    pub time: Duration,
}

impl GraphNode for Clip {
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        self.time = Duration::ZERO;
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        // TODO: Remove this extra clone.
        let Some(anim) = ctx.animations().0.get(&self.path).cloned() else {
            return;
        };

        let speed = self.speed.eval(ctx.variables()).to_f32().unwrap();
        let delta = ctx.delta().mul_f32(speed);

        // if self.path.to_lowercase().contains("reload") {
        // dbg!(&self, speed, &anim.duration);
        // }

        let mut anim_duration = anim.duration;
        if anim
            .flags
            .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
            && self.looping
        {
            anim_duration -= anim.frame_duration();
        }

        let start_time = self.time;

        if self.looping {
            // FIXME: Duration has no `Rem` impl?
            self.time = Duration::from_secs_f64(
                (self.time + delta).as_secs_f64() % anim_duration.as_secs_f64(),
            );
        } else {
            // Clamp at the maximum time.
            self.time = Duration::min(self.time + delta, anim_duration)
        }

        let end_time = self.time;

        for trigger in &self.triggers {
            let trigger_time = if trigger.from_end {
                // We will clamp the time at 0, if the would go negative.
                anim.duration.saturating_sub(trigger.local_time)
            } else {
                trigger.local_time
            };

            // Only activate the trigger if the time was reached in the current frame.
            if trigger_time > start_time && trigger_time <= end_time {
                ctx.send_event(trigger.event.clone());
            }
        }
    }
}
