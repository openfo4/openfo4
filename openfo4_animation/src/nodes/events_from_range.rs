use crate::nodes::{Context, GraphNode};
use crate::{Event, ValueExpression};

#[derive(Clone, Debug)]
pub struct EventsFromRange {
    /// Only send events when the condition is true.
    pub condition: ValueExpression,
    pub input: ValueExpression,
    pub ranges: Vec<Range>,
}

impl GraphNode for EventsFromRange {
    fn activate<T>(&mut self, ctx: T)
    where
        T: Context,
    {
        for range in &mut self.ranges {
            range.is_active = false;
        }

        self.update(ctx);
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if !self.condition.eval(ctx.variables()).to_bool().unwrap() {
            return;
        }

        let input = self.input.eval(ctx.variables());

        for range in &mut self.ranges {
            let lower = range.lower.eval(ctx.variables());
            let upper = range.upper.eval(ctx.variables());
            let in_range = input >= lower && input <= upper;

            if in_range && (!range.is_active || range.continous) {
                ctx.send_event(range.event.clone());
            }

            range.is_active = in_range;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Range {
    /// The lower bound of the range.
    pub lower: ValueExpression,
    /// The upper bound of the range.
    pub upper: ValueExpression,
    /// Whether to constantly send events while in the range.
    pub continous: bool,
    pub event: Event,
    /// Whether the range has already been entered.
    pub is_active: bool,
}
