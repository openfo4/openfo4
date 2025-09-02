use std::collections::HashMap;
use std::time::Duration;

use crate::nodes::{Context, GraphNode, NodeEvent};
use crate::{Blend, BoneWeights, Event, NodeId, StartState, Value, VariableId};

#[derive(Clone, Debug)]
pub struct StateMachine {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub state_id_to_index: HashMap<i32, usize>,
    pub start_state: StartState,
    pub state_variable: Option<VariableId>,
    // Runtime:
    pub active_state: Option<usize>,
    pub active_effect: Option<ActiveTransitionEffect>,
}

impl GraphNode for StateMachine {
    fn activate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        let start_state = match &self.start_state {
            StartState::None => None,
            StartState::Index(index) => Some(*index),
            StartState::Expression(expr) => {
                let id = expr.eval(ctx.variables()).to_i32().unwrap();
                self.state_id_to_index.get(&id).copied()
            }
            StartState::Random => Some(rand::random_range(0..self.states.len())),
        };

        let Some(start_state) = start_state else {
            self.active_state = None;
            return;
        };

        let state = &self.states[start_state];
        if let Some(node) = state.node {
            ctx.activate_node(node);
        }

        self.active_state = Some(start_state);
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        if let Some(index) = self.active_state {
            let state = &self.states[index];
            if let Some(node) = state.node {
                ctx.deactivate_node(node);
            }
        }
    }

    fn update<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        let Some(effect) = &mut self.active_effect else {
            return;
        };

        effect.time_left = effect.time_left.saturating_sub(ctx.delta());

        let progress = 1.0 - (effect.time_left.as_secs_f32() / effect.total_time.as_secs_f32());

        if let Some(dst_state) = &self.states[effect.dst_state].node {
            ctx.set_bone_weights(*dst_state, BoneWeights::Shared(progress));
        }

        for src_state in &mut effect.src_states {
            src_state.weight *= 1.0 - progress;

            let state = &self.states[src_state.state];

            if let Some(node) = state.node {
                ctx.set_bone_weights(node, BoneWeights::Shared(src_state.weight));
            }
        }

        if effect.time_left.is_zero() {
            self.active_effect = None;
        }
    }

    fn handle_event<T>(&mut self, mut ctx: T, event: &Event)
    where
        T: Context,
    {
        let Some(transition) = self.transitions.iter().find(|transition| {
            transition.event_id == event.id
                && transition
                    .from
                    .is_none_or(|from| Some(from) == self.active_state)
        }) else {
            return;
        };

        // Don't trigger a transition if we already are in the correct state.
        if self.active_state == Some(transition.to) {
            return;
        }

        if let Some(index) = self.active_state {
            if let Some(node) = self.states[index].node {
                ctx.deactivate_node(node);
            }
        }

        let state = &self.states[transition.to];

        if let Some(node) = state.node {
            ctx.activate_node(node);
        }

        if let Some(id) = self.state_variable {
            ctx.set_variable(id, Value::I32(state.id));
        }

        let effect = match &mut self.active_effect {
            Some(effect) => {
                let progress =
                    1.0 - effect.time_left.as_secs_f32() / effect.total_time.as_secs_f32();

                for state in &mut effect.src_states {
                    state.weight *= 1.0 - progress;
                }

                effect.src_states.push(BlendingState {
                    state: effect.dst_state,
                    weight: progress,
                });

                effect
            }
            None => self.active_effect.insert(ActiveTransitionEffect {
                total_time: Duration::ZERO,
                time_left: Duration::ZERO,
                src_states: vec![BlendingState {
                    state: self.active_state.unwrap(),
                    weight: 1.0,
                }],
                dst_state: 0,
            }),
        };

        effect.total_time = transition.blend_duration;
        effect.time_left = transition.blend_duration;
        effect.dst_state = transition.to;

        self.active_state = Some(transition.to);
    }

    fn children(&self) -> Vec<NodeId> {
        self.states.iter().filter_map(|s| s.node).collect()
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub id: i32,
    pub node: Option<NodeId>,
    /// Events that are fired once this `State` becomes active.
    pub enter_events: Vec<Event>,
    /// Events that are fired once this `State` becomes inactive.
    pub exit_events: Vec<Event>,
}

#[derive(Clone, Debug)]
pub struct Transition {
    /// The source state.
    ///
    /// Can be `None` in which case the transition is always valid.
    pub from: Option<usize>,
    /// The destination state.
    pub to: usize,
    /// The event which will trigger this transition if recevied.
    pub event_id: i64,
    pub priority: i64,
    /// If not set to [`ZERO`] blending between the enabled and disabled state happens in the given
    /// duration.
    pub blend_duration: Duration,
}

#[derive(Clone, Debug, Default)]
pub struct ActiveTransitionEffect {
    /// Total time of the transition.
    total_time: Duration,
    time_left: Duration,
    src_states: Vec<BlendingState>,
    dst_state: usize,
}

#[derive(Copy, Clone, Debug)]
struct BlendingState {
    state: usize,
    /// The weight of the state before starting the blending.
    ///
    /// The weights of all states must be normalized to 1.
    weight: f32,
}
