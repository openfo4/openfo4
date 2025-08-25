use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Mul, MulAssign};
use std::sync::Arc;
use std::time::Duration;

use game_tracing::trace_span;
use rand::Rng;

use crate::animation::AnimationClipFlags;
use crate::{
    AnimationGraph, AnimationSet, DriverEvent, Event, Listener, ListenerEvent, NodeId, NodeInner,
    StartState, TransitionEffect, Value, VariableId, VariableSet,
};

#[derive(Clone, Debug)]
pub struct AnimationDriver {
    pub graph: Arc<AnimationGraph>,
    states: HashMap<NodeId, NodeState>,
    // state_machine_states: HashMap<NodeId, StateMachineState>,
    // pub animation_playback_states: HashMap<NodeId, AnimationPlaybackState>,
    is_init: bool,
    events: VecDeque<Event>,
    blend_factors: HashMap<NodeId, f32>,
    variables: VariableSet,
    update_blend_factors: bool,
    listeners: HashMap<ListenerEvent, HashMap<NodeId, Listener>>,
    variables_changed: HashSet<VariableId>,
}

impl AnimationDriver {
    pub fn new(graph: Arc<AnimationGraph>) -> Self {
        Self {
            variables: graph.variables.clone(),
            graph,
            // state_machine_states: HashMap::new(),
            // animation_playback_states: HashMap::new(),
            // active_nodes: HashSet::new(),
            is_init: false,
            events: VecDeque::new(),
            blend_factors: HashMap::new(),
            states: HashMap::new(),
            update_blend_factors: false,
            listeners: HashMap::new(),
            variables_changed: HashSet::new(),
        }
    }

    pub fn update(
        &mut self,
        delta: Duration,
        anims: &AnimationSet,
        mut rng: impl Rng,
        events: &mut Vec<Event>,
    ) {
        let _span = trace_span!("AnimationDriver::update").entered();

        // let delta = delta.mul_f32(0.1);

        let mut set_blend_factor = VecDeque::new();

        let mut queue = VecDeque::new();

        if !self.is_init {
            self.is_init = true;

            if let Some(root) = self.graph.root {
                queue.push_back(DriverEvent::ActivateNode(root));
            }
        }

        for id in self.variables_changed.drain() {
            queue.push_back(DriverEvent::VariableChanged(id));
        }

        for id in self.states.keys() {
            queue.push_back(DriverEvent::UpdateNode(*id));
        }
        for event in self.events.drain(..) {
            queue.push_back(DriverEvent::HandleEvent(event));
        }

        while let Some(event) = queue.pop_front() {
            match event {
                DriverEvent::ActivateNode(id) => {
                    let node = &self.graph.nodes[id];
                    for cmd in &node.on_activate {
                        cmd.run(&mut self.variables, &mut queue);
                    }

                    let inner = match &self.graph.nodes[id].inner {
                        NodeInner::StateMachine(sm) => {
                            let state_index = match &sm.start_state {
                                StartState::None => None,
                                StartState::Index(state) => Some(*state),
                                StartState::Expression(expr) => {
                                    let id = expr.eval(&self.variables).to_i32().unwrap();
                                    sm.state_id_to_index.get(&id).copied()
                                }
                                StartState::Random => {
                                    let state = rand::random_range(0..sm.states.len());
                                    Some(state)
                                }
                            };

                            // TODO: Figure out whether state.enter_events is triggered.

                            if let Some(state_index) = state_index {
                                let state = &sm.states[state_index];
                                queue.push_back(DriverEvent::ActivateNode(state.node));
                            }

                            NodeStateInner::StateMachine(StateMachineState {
                                latest_state: state_index,
                                transitions: Vec::new(),
                                active_state: state_index,
                            })
                        }
                        NodeInner::List(list) => {
                            for id in list {
                                queue.push_back(DriverEvent::ActivateNode(*id));
                            }
                            NodeStateInner::None
                        }
                        NodeInner::Clip(_) => NodeStateInner::Clip(AnimationPlaybackState {
                            time: Duration::ZERO,
                        }),
                        NodeInner::Blend(node) => {
                            for c in &node.children {
                                queue.push_back(DriverEvent::ActivateNode(c.node));
                            }

                            NodeStateInner::None
                        }
                        NodeInner::VariableChangeTracker(_) => NodeStateInner::None,
                        NodeInner::Timer(timer) => NodeStateInner::Timer(TimerState {
                            remaining: timer.min_time,
                            running: true,
                        }),
                        NodeInner::Nop => NodeStateInner::None,
                    };

                    self.states.insert(
                        id,
                        NodeState {
                            blend_factor: 1.0,
                            computed_blend_factor: BoneWeights::Shared(1.0),
                            inner,
                        },
                    );

                    for listener in &node.listeners {
                        self.listeners
                            .entry(listener.event)
                            .or_default()
                            .insert(id, listener.clone());
                    }
                }
                DriverEvent::DeactivateNode(id) => {
                    if !self.states.contains_key(&id) {
                        continue;
                    }

                    let node = &self.graph.nodes[id];
                    for cmd in &node.on_deactivate {
                        cmd.run(&mut self.variables, &mut queue);
                    }

                    for listener in &node.listeners {
                        self.listeners.get_mut(&listener.event).unwrap().remove(&id);
                    }

                    match &self.graph.nodes[id].inner {
                        NodeInner::StateMachine(sm) => {
                            let sm_state = self.states.get(&id).unwrap();
                            let NodeStateInner::StateMachine(sm_state) = &sm_state.inner else {
                                unreachable!();
                            };

                            if let Some(state) = sm_state.latest_state {
                                let state = &sm.states[state];

                                // TODO: Figure out whether state.exit_events is triggered by
                                // state machine destruction.

                                queue.push_back(DriverEvent::DeactivateNode(state.node));
                            }
                        }
                        NodeInner::List(list) => {
                            for id in list {
                                queue.push_back(DriverEvent::DeactivateNode(*id));
                            }
                        }
                        NodeInner::Blend(node) => {
                            for c in &node.children {
                                queue.push_back(DriverEvent::DeactivateNode(c.node));
                            }
                        }
                        _ => (),
                    }

                    self.states.remove(&id);
                }
                DriverEvent::UpdateNode(id) => {
                    if !self.states.contains_key(&id) {
                        continue;
                    }

                    let node = &self.graph.nodes[id];
                    for cmd in &node.on_update {
                        cmd.run(&mut self.variables, &mut queue);
                    }

                    let state = self.states.get_mut(&id).unwrap();
                    match &self.graph.nodes[id].inner {
                        NodeInner::StateMachine(sm) => {
                            let NodeStateInner::StateMachine(sm_state) = &mut state.inner else {
                                unreachable!();
                            };

                            let mut remaining_time = delta;

                            let Some(mut active_state) = sm_state.active_state else {
                                continue;
                            };

                            while let Some(transition) = sm_state.transitions.first_mut() {
                                debug_assert_ne!(active_state, transition.to_state);

                                let old_state = &sm.states[active_state];
                                let new_state = &sm.states[transition.to_state];

                                if !transition.is_init {
                                    transition.is_init = true;

                                    for event in &new_state.enter_events {
                                        queue.push_back(DriverEvent::HandleEvent(event.clone()));
                                    }

                                    queue.push_back(DriverEvent::ActivateNode(new_state.node));
                                }

                                transition.time += remaining_time;

                                let is_done = match transition.effect {
                                    TransitionEffect::None => true,
                                    TransitionEffect::Blend(dur) => {
                                        debug_assert!(!dur.is_zero());

                                        // Clamp the time at the maximum duration to prevent
                                        // blend factors to go over 1.0.
                                        transition.time = transition.time.min(dur);

                                        let new_weight =
                                            transition.time.as_secs_f32() / dur.as_secs_f32();
                                        let old_weight = 1.0 - new_weight;

                                        debug_assert!(old_weight <= 1.0);
                                        debug_assert!(new_weight <= 1.0);
                                        debug_assert!(old_weight + new_weight <= 1.0);

                                        set_blend_factor.push_back((old_state.node, old_weight));
                                        set_blend_factor.push_back((new_state.node, new_weight));

                                        if transition.time >= dur {
                                            remaining_time -= transition.time - dur;
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                };

                                if !is_done {
                                    break;
                                }

                                set_blend_factor.push_back((new_state.node, 1.0));

                                queue.push_back(DriverEvent::DeactivateNode(old_state.node));

                                for event in &old_state.exit_events {
                                    queue.push_back(DriverEvent::HandleEvent(event.clone()));
                                }

                                active_state = transition.to_state;
                                sm_state.active_state = Some(transition.to_state);
                                sm_state.transitions.remove(0);

                                if let Some(variable) = sm.state_variable {
                                    self.variables.variables[variable] =
                                        Value::I32(sm_state.active_state.unwrap() as i32);
                                }
                            }
                        }
                        _ => (),
                    }
                }
                DriverEvent::HandleEvent(event) => {
                    events.push(event.clone());

                    if let Some(listeners) = self.listeners.get(&ListenerEvent::Event(event.id)) {
                        for listener in listeners.values() {
                            for cmd in &listener.reaction {
                                cmd.run(&mut self.variables, &mut queue);
                            }
                        }
                    }

                    for (id, state) in self.states.iter_mut() {
                        match &self.graph.nodes[*id].inner {
                            NodeInner::StateMachine(node) => {
                                let NodeStateInner::StateMachine(sm_state) = &mut state.inner
                                else {
                                    unreachable!();
                                };

                                let Some(transition) = node.transitions.iter().find(|transition| {
                                    transition.event_id == event.id
                                        && transition
                                            .from
                                            .is_none_or(|from| Some(from) == sm_state.latest_state)
                                }) else {
                                    continue;
                                };

                                let from = sm_state.latest_state;
                                let to = transition.to;

                                // Don't do a transition if the state doesn't change.
                                if from.unwrap() == to {
                                    // This can only happen if `from` was `None`, since we
                                    // already filter out transitions that are noops.
                                    debug_assert!(transition.from.is_none());
                                    continue;
                                }

                                sm_state.transitions.push(ActiveTransitionEffect {
                                    is_init: false,
                                    time: Duration::ZERO,
                                    effect: transition.effect.clone(),
                                    to_state: to,
                                });

                                sm_state.latest_state = Some(to);
                            }
                            _ => (),
                        }
                    }
                }
                DriverEvent::VariableChanged(variable) => {
                    let Some(listeners) = self.listeners.get(&ListenerEvent::Variable(variable))
                    else {
                        continue;
                    };

                    for listener in listeners.values() {
                        for cmd in &listener.reaction {
                            cmd.run(&mut self.variables, &mut queue);
                        }
                    }
                }
            }
        }

        for (node, state) in &mut self.states {
            match &self.graph.nodes[*node].inner {
                NodeInner::List(_) => (),
                NodeInner::StateMachine(_) => (),
                NodeInner::Clip(clip) => {
                    let NodeStateInner::Clip(clip_state) = &mut state.inner else {
                        unreachable!();
                    };

                    let Some(anim) = anims.0.get(&clip.path) else {
                        continue;
                    };

                    if anim.duration.is_zero() {
                        continue;
                    }

                    let start_time = clip_state.time;

                    let clip_speed = clip.speed.eval(&self.variables).to_f32().unwrap();
                    let delta = delta.mul_f32(clip_speed);

                    let mut anim_duration = anim.duration;
                    if anim
                        .flags
                        .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
                        && clip.looping
                    {
                        anim_duration -= anim.frame_duration();
                    }

                    if clip.looping {
                        // FIXME: Duration has no `Rem` impl?
                        clip_state.time = Duration::from_secs_f64(
                            (clip_state.time + delta).as_secs_f64() % anim_duration.as_secs_f64(),
                        );
                    } else {
                        // Clamp at the maximum time.
                        clip_state.time = Duration::min(clip_state.time + delta, anim_duration)
                    }

                    let end_time = clip_state.time;

                    for trigger in &clip.triggers {
                        let trigger_time = if trigger.from_end {
                            // We will clamp the time at 0, if the would go negative.
                            anim.duration.saturating_sub(trigger.local_time)
                        } else {
                            trigger.local_time
                        };

                        // Only activate the trigger if the time was reached in the current frame.
                        if trigger_time > start_time && trigger_time <= end_time {
                            // FIXME: Will cause 1 frame delay.
                            self.events.push_back(trigger.event.clone());
                        }
                    }
                }
                NodeInner::Blend(_) => (),
                NodeInner::VariableChangeTracker(_) => (),
                NodeInner::Timer(timer) => {
                    let NodeStateInner::Timer(state) = &mut state.inner else {
                        unreachable!();
                    };

                    state.remaining = state.remaining.saturating_sub(delta);

                    if state.remaining.is_zero() && state.running {
                        self.events.push_back(timer.event.clone());

                        if timer.repeat {
                            state.remaining = rng.random_range(timer.min_time..=timer.max_time);
                        } else {
                            state.running = false;
                        }
                    }
                }
                NodeInner::Nop => (),
            }
        }

        self.update_blend_factors |= !set_blend_factor.is_empty();

        while let Some((id, factor)) = set_blend_factor.pop_front() {
            if let Some(state) = self.states.get_mut(&id) {
                state.blend_factor = factor;
            }

            self.blend_factors.insert(id, factor);
        }

        // if self.update_blend_factors {
        self.update_blend_factors = false;
        self.update_blend_factors();
        // }

        events.extend(self.events.clone());
    }

    pub fn active_nodes(&self) -> Vec<NodeId> {
        self.states.keys().copied().collect()
    }

    /// Resets the current state.
    pub fn reset(&mut self) {
        self.variables = self.graph.variables.clone();

        self.events.clear();
        self.blend_factors.clear();
        self.states.clear();

        self.is_init = false;
    }

    fn update_blend_factors(&mut self) {
        let Some(root) = self.graph.root else {
            return;
        };

        let mut queue = VecDeque::new();
        queue.push_back((root, BoneWeights::Shared(1.0)));

        while let Some((id, mut blend_factor)) = queue.pop_front() {
            let Some(state) = self.states.get_mut(&id) else {
                continue;
            };

            blend_factor *= state.blend_factor;
            state.computed_blend_factor = blend_factor.clone();

            match &self.graph.nodes[id].inner {
                NodeInner::StateMachine(node) => {
                    for state in &node.states {
                        if self.states.contains_key(&state.node) {
                            queue.push_back((state.node, blend_factor.clone()));
                        }
                    }
                }
                NodeInner::List(list) => {
                    for node in list {
                        queue.push_back((*node, blend_factor.clone()));
                    }
                }
                NodeInner::Clip(clip) => {}
                NodeInner::Blend(node) => {
                    if node.is_parametric {
                        let blend_parameter =
                            node.blend_factor.eval(&self.variables).to_f32().unwrap();

                        let weights = node
                            .children
                            .iter()
                            .map(|node| node.weight.eval(&self.variables).to_f32().unwrap())
                            .collect::<Vec<_>>();

                        let Some((mut min, mut min_weight)) = weights
                            .iter()
                            .copied()
                            .enumerate()
                            .min_by(|(_, a), (_, b)| a.total_cmp(&b))
                        else {
                            continue;
                        };

                        let Some((mut max, mut max_weight)) = weights
                            .iter()
                            .copied()
                            .enumerate()
                            .max_by(|(_, a), (_, b)| a.total_cmp(&b))
                        else {
                            continue;
                        };

                        // Select the two children so that min < blend_parameter < max.
                        // If blend_parameter < min we will choose the children min and min+1.
                        // If blend_parameter > max we will choose the children max-1 and max.
                        // If only one children exists we always clamp blend_parameter to that
                        // exact weight.

                        for (index, &weight) in weights.iter().enumerate() {
                            if weight <= blend_parameter && weight > min_weight {
                                min = index;
                                min_weight = weight;
                            }

                            if weight >= blend_parameter && weight < max_weight {
                                max = index;
                                max_weight = weight;
                            }
                        }

                        let blend_parameter = blend_parameter.clamp(min_weight, max_weight);

                        for (index, child) in node.children.iter().enumerate() {
                            let weight = weights[index];

                            let s = if min == index {
                                debug_assert_eq!(min_weight, weight);

                                f32::min(
                                    1.0,
                                    1.0 - (blend_parameter - min_weight)
                                        / (max_weight - min_weight),
                                )
                            } else if max == index {
                                debug_assert_eq!(max_weight, weight);

                                f32::max(
                                    0.0,
                                    (blend_parameter - min_weight) / (max_weight - min_weight),
                                )
                            } else {
                                0.0
                            };

                            let mut blend_factor = blend_factor.clone() * s;
                            if let Some(weights) = &child.bone_weights {
                                let weights = weights.eval(&self.variables);
                                let weights = match weights {
                                    Value::BoneWeights(weights) => weights,
                                    _ => unreachable!(),
                                };

                                blend_factor *= &BoneWeights::Unique(weights);
                            }

                            queue.push_back((child.node, blend_factor));
                        }
                    } else {
                        let weights = node
                            .children
                            .iter()
                            .map(|child| child.weight.eval(&self.variables).to_f32().unwrap())
                            .collect::<Vec<_>>();

                        let sum = weights.iter().sum::<f32>();

                        for (child, weight) in node.children.iter().zip(weights) {
                            let mut blend_factor = if sum.abs() == 0.0 {
                                BoneWeights::Shared(0.0)
                            } else {
                                blend_factor.clone() * (weight / sum)
                            };

                            if let Some(weights) = &child.bone_weights {
                                let weights = weights.eval(&self.variables);
                                let weights = match weights {
                                    Value::BoneWeights(weights) => weights,
                                    _ => unreachable!(),
                                };

                                blend_factor *= &BoneWeights::Unique(weights);
                            }

                            queue.push_back((child.node, blend_factor));
                        }
                    }
                }
                NodeInner::VariableChangeTracker(_) => (),
                NodeInner::Timer(_) => (),
                NodeInner::Nop => (),
            }
        }
    }

    pub fn send_event(&mut self, event: &str) {
        let Some(event_id) = self
            .graph
            .event_names
            .iter()
            .position(|e| e.to_lowercase() == event.to_lowercase())
        else {
            return;
        };

        let event = Event {
            id: event_id as i64,
            payload: Value::None,
        };

        self.events.push_back(event);
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.get_by_name(name)
    }

    pub fn get_variable_mut(&mut self, name: &str) -> Option<&mut Value> {
        let id = *self.variables.variables_by_name.get(name)?;

        self.update_blend_factors = true;
        self.variables_changed.insert(id);

        Some(&mut self.variables.variables[id])
    }

    pub fn animations(
        &self,
    ) -> impl Iterator<Item = (usize, &AnimationPlaybackState, &BoneWeights)> {
        self.states
            .iter()
            .filter_map(|(id, node)| match &node.inner {
                NodeStateInner::Clip(state) => Some((*id, state, &node.computed_blend_factor)),
                _ => None,
            })
    }
}

#[derive(Clone, Debug)]
struct NodeState {
    blend_factor: f32,
    computed_blend_factor: BoneWeights,
    inner: NodeStateInner,
}

#[derive(Clone, Debug)]
enum NodeStateInner {
    None,
    StateMachine(StateMachineState),
    Clip(AnimationPlaybackState),
    Timer(TimerState),
}

#[derive(Clone, Debug)]
pub struct StateMachineState {
    /// The currently active state.
    pub active_state: Option<usize>,
    /// The last state that was enabled.
    pub latest_state: Option<usize>,
    pub transitions: Vec<ActiveTransitionEffect>,
}

#[derive(Clone, Debug)]
pub struct ActiveTransitionEffect {
    pub is_init: bool,
    pub effect: TransitionEffect,
    pub time: Duration,
    pub to_state: usize,
}

#[derive(Clone, Debug)]
pub struct AnimationPlaybackState {
    pub time: Duration,
}

#[derive(Clone, Debug)]
struct TimerState {
    remaining: Duration,
    running: bool,
}

#[derive(Clone, Debug)]
pub enum BoneWeights {
    Shared(f32),
    Unique(Vec<f32>),
}

impl BoneWeights {
    pub fn get(&self, bone: u32) -> f32 {
        match self {
            Self::Shared(v) => *v,
            Self::Unique(bones) => bones.get(bone as usize).copied().unwrap_or(1.0),
        }
    }
}

impl Mul for BoneWeights {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Shared(lhs), Self::Shared(rhs)) => Self::Shared(lhs * rhs),
            (Self::Unique(mut lhs), Self::Unique(rhs)) => {
                let count = lhs.len().max(rhs.len());
                lhs.resize(count, 1.0);
                for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                    *lhs *= rhs;
                }

                Self::Unique(lhs)
            }
            (Self::Unique(mut lhs), Self::Shared(rhs)) => {
                for lhs in &mut lhs {
                    *lhs *= rhs;
                }

                Self::Unique(lhs)
            }
            (Self::Shared(lhs), Self::Unique(rhs)) => {
                let mut rhs = rhs.clone();
                for rhs in &mut rhs {
                    *rhs *= lhs;
                }

                Self::Unique(rhs)
            }
        }
    }
}

impl MulAssign<&Self> for BoneWeights {
    fn mul_assign(&mut self, rhs: &Self) {
        match (&mut *self, rhs) {
            (Self::Shared(lhs), Self::Shared(rhs)) => *lhs *= rhs,
            (Self::Unique(lhs), Self::Unique(rhs)) => {
                let count = lhs.len().max(rhs.len());
                lhs.resize(count, 1.0);
                for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                    *lhs *= rhs;
                }
            }
            (Self::Unique(lhs), Self::Shared(rhs)) => {
                for lhs in lhs {
                    *lhs *= rhs;
                }
            }
            (Self::Shared(lhs), Self::Unique(rhs)) => {
                let mut rhs = rhs.clone();
                for rhs in &mut rhs {
                    *rhs *= *lhs;
                }

                *self = Self::Unique(rhs);
            }
        }
    }
}

impl Mul<f32> for BoneWeights {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Self::Shared(lhs) => Self::Shared(lhs * rhs),
            Self::Unique(mut lhs) => {
                for lhs in &mut lhs {
                    *lhs *= rhs;
                }

                Self::Unique(lhs)
            }
        }
    }
}

impl MulAssign<f32> for BoneWeights {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Self::Shared(lhs) => *lhs *= rhs,
            Self::Unique(lhs) => {
                for lhs in lhs {
                    *lhs *= rhs;
                }
            }
        }
    }
}
