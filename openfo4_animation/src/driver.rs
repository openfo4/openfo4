use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use game_tracing::trace_span;
use rand::Rng;

use crate::animation::{AnimationClipFlags, Annotation};
use crate::{
    AnimationGraph, AnimationSet, Event, ListenerEvent, Node, NodeId, NodeInner, StartState,
    Statement, TransitionEffect, Value, VariableSet,
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
    sync_groups: HashMap<NodeId, SyncGroup>,
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
            sync_groups: HashMap::new(),
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

        let mut activate_nodes = Vec::new();
        let mut deactivate_nodes = Vec::new();
        let mut set_blend_factor = VecDeque::new();

        if !self.is_init {
            self.is_init = true;

            if let Some(root) = self.graph.root {
                activate_nodes.push(root);
            }
        }

        while let Some(event) = self.events.pop_front() {
            for (id, state) in &mut self.states {
                match &self.graph.nodes[*id].inner {
                    NodeInner::StateMachine(node) => {
                        let NodeStateInner::StateMachine(sm_state) = &mut state.inner else {
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
                    NodeInner::List(_) => (),
                    NodeInner::Clip(_) => (),
                    NodeInner::SetVariables(_) => (),
                    NodeInner::Blend(_) => (),
                    NodeInner::VariableChangeTracker(_) => (),
                    NodeInner::PerFrameCommands(_) => (),
                    NodeInner::Timer(_) => (),
                    NodeInner::Listener(listener) => {
                        if let ListenerEvent::Event(e) = listener.event {
                            if e == event.id {
                                listener.reaction.run(&mut self.variables, &mut self.events);
                            }
                        }
                    }
                }
            }
        }

        for sync_group in self.sync_groups.values_mut() {
            let mut duration_sum = Duration::ZERO;

            let leader_looping = match &self.graph.nodes[sync_group.leader.id].inner {
                NodeInner::Clip(clip) => clip.looping,
                _ => unreachable!(),
            };

            for node in sync_group
                .followers
                .iter()
                .chain(std::iter::once(&sync_group.leader))
            {
                match &self.graph.nodes[node.id].inner {
                    NodeInner::Clip(clip) => {
                        let Some(anim) = anims.0.get(&clip.path) else {
                            continue;
                        };

                        let mut anim_duration = anim.duration;
                        if anim
                            .flags
                            .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
                            && leader_looping
                        {
                            anim_duration -= anim.frame_duration();
                        }

                        duration_sum += anim_duration.mul_f32(node.scale);
                    }
                    _ => (),
                }
            }

            match (
                &self.graph.nodes[sync_group.leader.id].inner,
                &self.states[&sync_group.leader.id].inner,
            ) {
                (NodeInner::Clip(clip), NodeStateInner::Clip(state)) => {
                    let Some(anim) = anims.0.get(&clip.path) else {
                        continue;
                    };

                    let mut anim_duration = anim.duration;
                    if anim
                        .flags
                        .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
                        && clip.looping
                    {
                        anim_duration -= anim.frame_duration();
                    }

                    let leader_timescale = anim_duration.as_secs_f32() / duration_sum.as_secs_f32();

                    let delta = delta.mul_f32(clip.speed * leader_timescale);

                    if clip.looping {
                        // FIXME: Duration has no `Rem` impl?
                        sync_group.leader.time = Duration::from_secs_f64(
                            (state.time + delta).as_secs_f64() % anim_duration.as_secs_f64(),
                        );
                    } else {
                        // Clamp at the maximum time.
                        sync_group.leader.time = Duration::min(state.time + delta, anim_duration);
                    }
                }
                _ => (),
            }
        }

        for (node, state) in &mut self.states {
            match &self.graph.nodes[*node].inner {
                NodeInner::List(_) => (),
                NodeInner::StateMachine(sm) => {
                    let NodeStateInner::StateMachine(sm_state) = &mut state.inner else {
                        unreachable!();
                    };

                    let mut remaining_time = delta;

                    let Some(active_state) = sm_state.active_state else {
                        continue;
                    };

                    while let Some(transition) = sm_state.transitions.first_mut() {
                        debug_assert_ne!(active_state, transition.to_state);

                        let old_state = &sm.states[active_state];
                        let new_state = &sm.states[transition.to_state];

                        if !transition.is_init {
                            transition.is_init = true;

                            // FIXME: These events are not processed in this frame, causing a
                            // 1 frame delay. We should process them in this frame.
                            self.events.extend(new_state.enter_events.clone());

                            activate_nodes.push(new_state.node);
                        }

                        transition.time += remaining_time;

                        let is_done = match transition.effect {
                            TransitionEffect::None => true,
                            TransitionEffect::Blend(dur) => {
                                debug_assert!(!dur.is_zero());

                                // Clamp the time at the maximum duration to prevent
                                // blend factors to go over 1.0.
                                transition.time = transition.time.min(dur);

                                let new_weight = transition.time.as_secs_f32() / dur.as_secs_f32();
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

                        deactivate_nodes.push(old_state.node);
                        self.events.extend(old_state.exit_events.clone());

                        sm_state.active_state = Some(transition.to_state);
                        sm_state.transitions.remove(0);

                        if let Some(variable) = sm.state_variable {
                            self.variables.variables[variable] =
                                Value::I32(sm_state.active_state.unwrap() as i32);
                        }
                    }
                }
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

                    let delta = delta.mul_f32(clip.speed);

                    if let Some(group) = state.sync_group {
                        let group = &self.sync_groups[&group];
                        if group.leader.id == *node {
                            clip_state.time = group.leader.time;
                        }
                    }

                    if let Some(group) = state.sync_group {
                        let group = &self.sync_groups[&group];

                        // Compute leader position.

                        let mut leader_annotations = Vec::new();
                        let mut leader_anim_duration = Duration::ZERO;
                        let mut leader_looping = false;
                        match &self.graph.nodes[group.leader.id].inner {
                            NodeInner::Clip(clip) => {
                                leader_looping = clip.looping;

                                match anims.0.get(&clip.path) {
                                    Some(anim) => {
                                        leader_annotations = anim
                                            .annotations
                                            .iter()
                                            .filter_map(|annotation| match annotation {
                                                Annotation::Sync(duration, name) => {
                                                    Some((duration, name))
                                                }
                                            })
                                            .collect();

                                        leader_anim_duration = anim.duration;
                                        if anim.flags.contains(
                                            AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING,
                                        ) && clip.looping
                                        {
                                            leader_anim_duration -= anim.frame_duration();
                                        }
                                    }
                                    _ => (),
                                };
                            }
                            _ => (),
                        }

                        let mut leader_start_sync_point = None;
                        let mut leader_end_sync_point = None;
                        let mut leader_start_time = Duration::ZERO;
                        let mut leader_end_time = leader_anim_duration;

                        for (&time, name) in leader_annotations {
                            if time <= group.leader.time && time > leader_start_time {
                                leader_start_sync_point = Some(name);
                                leader_start_time = time;
                            }

                            if time > group.leader.time && time < leader_end_time {
                                leader_end_sync_point = Some(name);
                                leader_end_time = time;
                            }
                        }

                        debug_assert!(leader_end_time >= leader_start_time);
                        debug_assert!(group.leader.time >= leader_start_time);
                        debug_assert!(group.leader.time <= leader_end_time);

                        let sync_location = (group.leader.time.as_secs_f32()
                            - leader_start_time.as_secs_f32())
                            / (leader_end_time.as_secs_f32() - leader_start_time.as_secs_f32());

                        // Compute our position.

                        let mut anim_duration = anim.duration;
                        if leader_looping
                            && anim
                                .flags
                                .contains(AnimationClipFlags::TRIM_LAST_FRAME_IF_LOOPING)
                        {
                            anim_duration -= anim.frame_duration();
                        }

                        let start_time = match leader_start_sync_point {
                            Some(leader_start_sync_point) => anim
                                .annotations
                                .iter()
                                .find_map(|annotation| match annotation {
                                    Annotation::Sync(time, name) => {
                                        (name == leader_start_sync_point).then_some(*time)
                                    }
                                })
                                .unwrap_or_default(),
                            None => Duration::ZERO,
                        };

                        let end_time = match leader_end_sync_point {
                            Some(leader_end_sync_point) => anim
                                .annotations
                                .iter()
                                .find_map(|annotation| match annotation {
                                    Annotation::Sync(time, name) => {
                                        (name == leader_end_sync_point).then_some(*time)
                                    }
                                })
                                .unwrap_or(anim_duration),
                            None => anim_duration,
                        };

                        dbg!(
                            leader_start_time,
                            leader_end_time,
                            start_time,
                            end_time,
                            sync_location
                        );

                        let time = start_time + (end_time - start_time).mul_f32(sync_location);
                        clip_state.time = time;
                    } else {
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
                                (clip_state.time + delta).as_secs_f64()
                                    % anim_duration.as_secs_f64(),
                            );
                        } else {
                            // Clamp at the maximum time.
                            clip_state.time = Duration::min(clip_state.time + delta, anim_duration)
                        }
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
                NodeInner::SetVariables(_) => (),
                NodeInner::Blend(_) => (),
                NodeInner::VariableChangeTracker(_) => (),
                NodeInner::PerFrameCommands(_) => (),
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
                NodeInner::Listener(_) => (),
            }
        }

        while let Some(id) = deactivate_nodes.pop() {
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

                        deactivate_nodes.push(state.node);
                    }
                }
                NodeInner::List(list) => {
                    deactivate_nodes.extend(list);
                }
                NodeInner::Clip(_) => {}
                NodeInner::SetVariables(node) => {
                    for var in &node.on_deactivate {
                        if var.condition.eval(&self.variables) != Value::Bool(true) {
                            continue;
                        }

                        let value = var.value.eval(&self.variables);

                        self.variables.variables[var.variable] = value;
                    }
                }
                NodeInner::Blend(node) => {
                    for c in &node.children {
                        deactivate_nodes.push(c.node);
                    }
                }
                NodeInner::VariableChangeTracker(_) => (),
                NodeInner::PerFrameCommands(_) => (),
                NodeInner::Timer(_) => (),
                NodeInner::Listener(_) => (),
            }

            self.sync_groups.remove(&id);
            self.states.remove(&id);
        }

        while let Some(id) = activate_nodes.pop() {
            let inner = match &self.graph.nodes[id].inner {
                NodeInner::StateMachine(sm) => {
                    let state_index = match &sm.start_state {
                        StartState::None => None,
                        StartState::Index(state) => Some(*state),
                        StartState::Expression(expr) => {
                            Some(expr.eval(&self.variables).to_usize().unwrap_or(0))
                        }
                        StartState::Random => todo!(),
                    };

                    // TODO: Figure out whether state.enter_events is triggered.

                    if let Some(state_index) = state_index {
                        let state = &sm.states[state_index];
                        activate_nodes.push(state.node);
                    }

                    NodeStateInner::StateMachine(StateMachineState {
                        latest_state: state_index,
                        transitions: Vec::new(),
                        active_state: state_index,
                    })
                }
                NodeInner::List(list) => {
                    activate_nodes.extend(list);
                    NodeStateInner::None
                }
                NodeInner::Clip(_) => NodeStateInner::Clip(AnimationPlaybackState {
                    time: Duration::ZERO,
                }),
                NodeInner::SetVariables(node) => {
                    for var in &node.on_activate {
                        if var.condition.eval(&self.variables) != Value::Bool(true) {
                            continue;
                        }

                        let value = var.value.eval(&self.variables);
                        self.variables.variables[var.variable] = value;
                    }

                    NodeStateInner::None
                }
                NodeInner::Blend(node) => {
                    for c in &node.children {
                        activate_nodes.push(c.node);
                    }

                    NodeStateInner::None
                }
                NodeInner::VariableChangeTracker(_) => NodeStateInner::None,
                NodeInner::PerFrameCommands(_) => NodeStateInner::None,
                NodeInner::Timer(timer) => NodeStateInner::Timer(TimerState {
                    remaining: rng.random_range(timer.min_time..=timer.max_time),
                    running: true,
                }),
                NodeInner::Listener(_) => NodeStateInner::None,
            };

            self.states.insert(
                id,
                NodeState {
                    blend_factor: 1.0,
                    global_blend_factor: 1.0,
                    inner,
                    sync_group: None,
                },
            );
        }

        self.update_blend_factors |= !set_blend_factor.is_empty();

        while let Some((id, factor)) = set_blend_factor.pop_front() {
            if let Some(state) = self.states.get_mut(&id) {
                state.blend_factor = factor;
            }

            self.blend_factors.insert(id, factor);
        }

        if self.update_blend_factors {
            self.update_blend_factors = false;
            self.update_blend_factors();
        }

        self.handle_variable_changes();
        self.run_per_frame_cmds();

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

    fn handle_variable_changes(&mut self) {
        for node in self.states.keys() {
            let node = &self.graph.nodes[*node];
            match &node.inner {
                NodeInner::VariableChangeTracker(node) => {
                    for tracker in &node.trackers {
                        if let Value::Bool(value) = tracker.expr.eval(&self.variables) {
                            if value {
                                self.events.push_back(tracker.event.clone());
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn run_per_frame_cmds(&mut self) {
        for node in self.states.keys() {
            let node = &self.graph.nodes[*node];
            match &node.inner {
                NodeInner::PerFrameCommands(cmd) => {
                    for stmt in &cmd.stmts {
                        match stmt {
                            Statement::SetVariable(stmt) => {
                                if stmt.condition.eval(&self.variables).to_bool() != Some(true) {
                                    continue;
                                }

                                let value = stmt.value.eval(&self.variables);
                                self.variables.variables[stmt.variable] = value;
                            }
                            Statement::SendEvent(stmt) => {
                                if stmt.condition.eval(&self.variables).to_bool() != Some(true) {
                                    continue;
                                }

                                self.events.push_back(stmt.event.clone());
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn update_blend_factors(&mut self) {
        let Some(root) = self.graph.root else {
            return;
        };

        let mut queue = VecDeque::new();
        queue.push_back((root, 1.0, None));

        while let Some((id, blend_factor, sync_group)) = queue.pop_front() {
            let state = self.states.get_mut(&id).unwrap();

            let blend_factor = blend_factor * state.blend_factor;
            state.global_blend_factor = blend_factor;
            state.sync_group = sync_group;

            match &self.graph.nodes[id].inner {
                NodeInner::StateMachine(node) => {
                    // Ensure at least one state is active.
                    // debug_assert!(
                    //     !node
                    //         .states
                    //         .iter()
                    //         .all(|s| !self.states.contains_key(&s.node))
                    // );

                    for state in &node.states {
                        if self.states.contains_key(&state.node) {
                            queue.push_back((state.node, blend_factor, sync_group));
                        }
                    }
                }
                NodeInner::List(list) => {
                    for node in list {
                        queue.push_back((*node, blend_factor, sync_group));
                    }
                }
                NodeInner::SetVariables(_) => (),
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

                        let mut nodes = Vec::new();

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

                            nodes.push(SyncGroupNode {
                                id: child.node,
                                time: Duration::ZERO,
                                scale: s,
                            });

                            queue.push_back((child.node, blend_factor * s, Some(id)));
                        }

                        if let Some((index, leader)) = nodes
                            .iter()
                            .enumerate()
                            .max_by(|(_, a), (_, b)| a.scale.total_cmp(&b.scale))
                        {
                            let leader = *leader;
                            nodes.remove(index);

                            self.sync_groups.insert(
                                id,
                                SyncGroup {
                                    leader,
                                    followers: nodes,
                                },
                            );
                        }
                    } else {
                        for child in &node.children {
                            let weight = child.weight.eval(&self.variables).to_f32().unwrap();
                            let blend_factor = blend_factor * weight;

                            queue.push_back((child.node, blend_factor, sync_group));
                        }
                    }
                }
                NodeInner::VariableChangeTracker(_) => (),
                NodeInner::PerFrameCommands(_) => (),
                NodeInner::Timer(_) => (),
                NodeInner::Listener(_) => (),
            }
        }
    }

    pub fn send_event(&mut self, event: &'static str) {
        let Some(event_id) = self.graph.event_names.iter().position(|e| *e == event) else {
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
        self.update_blend_factors = true;
        self.variables.get_mut_by_name(name)
    }

    pub fn animations(&self) -> impl Iterator<Item = (usize, &AnimationPlaybackState, f32)> {
        self.states
            .iter()
            .filter_map(|(id, node)| match &node.inner {
                NodeStateInner::Clip(state) => Some((*id, state, node.global_blend_factor)),
                _ => None,
            })
    }
}

#[derive(Clone, Debug)]
struct NodeState {
    blend_factor: f32,
    global_blend_factor: f32,
    /// The sync group in which this node is contained.
    sync_group: Option<NodeId>,
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

/// A group of animations that are synchronized.
#[derive(Clone, Debug)]
struct SyncGroup {
    leader: SyncGroupNode,
    followers: Vec<SyncGroupNode>,
}

#[derive(Copy, Clone, Debug)]
struct SyncGroupNode {
    id: NodeId,
    time: Duration,
    scale: f32,
}
