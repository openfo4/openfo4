use std::collections::HashMap;
use std::time::Duration;

use crate::events::EventId;
use crate::nodes::{Context, GraphNode};
use crate::{BoneWeights, Event, NodeId, StartState, Value, ValueExpression, VariableId};

#[derive(Clone, Debug)]
pub struct StateMachine {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub state_id_to_index: HashMap<i32, usize>,
    pub start_state: StartState,
    pub state_variable: Option<VariableId>,
    /// Whether transitions to a state that is currently active are applied.
    ///
    /// If `false` such transitions are ignored.
    pub reentrant: bool,
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
            StartState::Random => {
                if self.states.is_empty() {
                    None
                } else {
                    Some(rand::random_range(0..self.states.len()))
                }
            }
        };

        self.active_effect = None;

        let Some(start_state) = start_state else {
            self.active_state = None;
            return;
        };

        let state = &self.states[start_state];
        if let Some(node) = state.node {
            ctx.activate_node(node);

            // It is possible that an old bone weight still exists
            // when the state machine was disabled, so we must reset it.
            ctx.set_playback_speed(node, 1.0);
            ctx.set_bone_weights(node, BoneWeights::Shared(1.0));
        }

        self.active_state = Some(start_state);
    }

    fn deactivate<T>(&mut self, mut ctx: T)
    where
        T: Context,
    {
        // Deacticate all states; there may be multiple states active
        // during a transition.
        // If a state is already inactive the caller will ignore it.
        // Note that we are not "leaving" the state and do not need to
        // send any leave events.
        for state in &self.states {
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
            if let Some(state) = self.active_state {
                if let Some(node) = self.states[state].node {
                    ctx.set_bone_weights(node, BoneWeights::Shared(1.0));
                }
            }

            return;
        };

        effect.time_left = effect.time_left.saturating_sub(ctx.delta());

        let progress = if effect.total_time.is_zero() {
            1.0
        } else {
            1.0 - (effect.time_left.as_secs_f32() / effect.total_time.as_secs_f32())
        };

        if let Some(dst_state) = &self.states[effect.dst_state].node {
            ctx.set_bone_weights(*dst_state, BoneWeights::Shared(progress));
        }

        for src_state in &effect.src_states {
            let mut weight = src_state.weight * (1.0 - progress);

            // It is possible that a source->destination blending includes the
            // same node. We must combine the weights in this case.
            // Since destination weight was already written this will overwrite
            // it with the combined weight.
            if src_state.state_index == effect.dst_state {
                weight += progress;
            }

            let state = &self.states[src_state.state_index];

            if let Some(node) = state.node {
                ctx.set_bone_weights(node, BoneWeights::Shared(weight));
            }
        }

        if effect.time_left.is_zero() {
            for state in &effect.src_states {
                if state.state_index == effect.dst_state {
                    continue;
                }

                let state = &self.states[state.state_index];
                if let Some(node) = state.node {
                    ctx.deactivate_node(node);
                }

                for event in &state.exit_events {
                    ctx.send_event(event.clone());
                }
            }

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
                && transition
                    .condition
                    .eval(ctx.variables())
                    .to_bool()
                    .unwrap()
        }) else {
            return;
        };

        let state = &self.states[transition.to];

        // Don't trigger a transition if we already are in the correct state.
        if self.active_state == Some(transition.to) {
            if !self.reentrant {
                return;
            }

            // No need to start a transition effect if we remain in the same state.
            if let Some(node) = state.node {
                ctx.deactivate_node(node);
                ctx.activate_node(node);
            }

            for event in &state.enter_events {
                ctx.send_event(event.clone());
            }

            return;
        }

        if let Some(id) = self.state_variable {
            ctx.set_variable(id, Value::I32(state.id));
        }

        // Freeze the currently active node.
        if let Some(index) = self.active_state {
            if let Some(node) = self.states[index].node {
                ctx.set_playback_speed(node, 0.0);
            }
        }

        if let Some(node) = self.states[transition.to].node {
            ctx.set_playback_speed(node, 1.0);
        }

        let effect = match &mut self.active_effect {
            Some(effect) => {
                let progress = if effect.total_time.is_zero() {
                    1.0
                } else {
                    1.0 - effect.time_left.as_secs_f32() / effect.total_time.as_secs_f32()
                };

                let mut coalesced = false;

                for state in &mut effect.src_states {
                    state.weight *= 1.0 - progress;

                    if state.state_index == effect.dst_state {
                        debug_assert!(!coalesced);
                        state.weight += progress;
                        coalesced = true;
                    }
                }

                if !coalesced {
                    effect.src_states.push(BlendingState {
                        state_index: effect.dst_state,
                        weight: progress,
                    });
                }

                effect
            }
            None => self.active_effect.insert(ActiveTransitionEffect {
                total_time: Duration::ZERO,
                time_left: Duration::ZERO,
                src_states: vec![BlendingState {
                    state_index: self.active_state.unwrap(),
                    weight: 1.0,
                }],
                dst_state: 0,
            }),
        };

        let blend_duration = Duration::from_secs_f32(
            transition
                .blend_duration
                .eval(ctx.variables())
                .to_f32()
                .unwrap(),
        );

        effect.total_time = blend_duration;
        effect.time_left = blend_duration;
        effect.dst_state = transition.to;

        // The node is only deactivated once the transition weight reaches 0.
        // If quickly transitioning in and then out the state can already be
        // active and we must not activate it again.
        if !effect
            .src_states
            .iter()
            .any(|state| state.state_index == transition.to)
        {
            if let Some(node) = state.node {
                ctx.activate_node(node);
                ctx.set_playback_speed(node, 1.0);
            }

            for event in &state.enter_events {
                ctx.send_event(event.clone());
            }
        }

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
    pub condition: ValueExpression,
    /// The source state.
    ///
    /// Can be `None` in which case the transition is always valid.
    pub from: Option<usize>,
    /// The destination state.
    pub to: usize,
    /// The event which will trigger this transition if recevied.
    pub event_id: EventId,
    pub priority: i64,
    /// Blending transition duration in seconds.
    pub blend_duration: ValueExpression,
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
    state_index: usize,
    /// The weight of the state before starting the blending.
    ///
    /// The weights of all states must be normalized to 1.
    weight: f32,
}
