mod f4se;
mod mcm;

mod active_magic_effect;
mod actor;
mod actor_base;
mod alias;
mod cell;
mod debug;
mod faction;
mod form;
mod form_list;
mod game;
mod global_variable;
mod input_enable_layer;
mod keyword;
mod location;
mod location_alias;
mod magic_effect;
mod math;
mod message;
mod music_type;
mod object_reference;
mod potion;
mod quest;
mod ref_collection_alias;
mod reference_alias;
mod scene;
mod script_object;
mod sound;
mod spell;
mod utility;
mod weapon;

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use hecs::World;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{ObjectAddr, ObjectRef};
use parking_lot::Mutex;
use slab::Slab;

use crate::systems::quests::QuestManager;
use crate::systems::scripting::Objects;

pub type Int = i32;
pub type Float = f32;
pub type Object = ObjectAddr;
pub type Bool = bool;

// Don't reformat long lines in here. This function is essentially a big table
// and one line per entry makes it clearer.
// FIXME: Optimally we would like to only skip long lines check, but is this possible?
#[rustfmt::skip]
pub fn register_builtins(engine: &mut Engine) {
    math::register(engine);
    object_reference::register(engine);
    input_enable_layer::register(engine);
    music_type::register(engine);
    alias::register(engine);
    scene::register(engine);
    debug::register(engine);
    actor_base::register(engine);
    weapon::register(engine);
    ref_collection_alias::register(engine);
    cell::register(engine);
    location::register(engine);
    location_alias::register(engine);
    game::register(engine);
    reference_alias::register(engine);

    engine.register_native_function("Message", "ResetHelpMessage", message::reset_help_message);
    engine.register_native_function("Message", "Show", message::show);
    engine.register_native_function("Message", "ShowAsHelpMessage", message::show_as_help_message);
    engine.register_native_function("Message", "UnshowAsHelpMessage", message::unshow_as_help_message);

    script_object::register(engine);


    engine.register_native_function("Form", "GetFormID", form::get_form_id);
    engine.register_native_function("Form", "GetGoldValue", form::get_gold_value);
    engine.register_native_function("Form", "HasKeyword", form::has_keyword);
    engine.register_native_function("Form", "HasKeywordInFormList", form::has_keyword_in_form_list);
    engine.register_native_function("Form", "PlayerKnows", form::player_knows);

    // engine.register_native_function("GlobalVariable", "GetValue", global_variable::get_value);
    // engine.register_native_function("GlobalVariable", "SetValue", global_variable::set_value);

    actor::register(engine);
    utility::register(engine);

    engine.register_native_function("Sound", "SetInstanceVolume", sound::set_instance_volume);
    engine.register_native_function("Sound", "StopInstance", sound::stop_instance);
    engine.register_native_function("Sound", "Play", sound::play);
    engine.register_native_function("Sound", "PlayAndWait", sound::play_and_wait);

    engine.register_native_function("Spell", "Cast", spell::cast);
    engine.register_native_function("Spell", "IsHostile", spell::is_hostile);
    engine.register_native_function("Spell", "RemoteCast", spell::remote_cast);

    engine.register_native_function("GlobalVariable", "GetValue", global_variable::get_value);
    engine.register_native_function("GlobalVariable", "SetValue", global_variable::set_value);

    engine.register_native_function("Keyword", "SendStoryEvent", keyword::send_story_event);
    engine.register_native_function("Keyword", "SendStoryEventAndWait", keyword::send_story_event_and_wait);

    engine.register_native_function("ActiveMagicEffect", "Dispel", active_magic_effect::dispel);
    engine.register_native_function("ActiveMagicEffect", "GetBaseObject", active_magic_effect::get_base_object);
    engine.register_native_function("ActiveMagicEffect", "GetCasterActor", active_magic_effect::get_caster_actor);
    engine.register_native_function("ActiveMagicEffect", "GetTargetActor", active_magic_effect::get_target_actor);

    engine.register_native_function("MagicEffect", "GetAssociatedSkill", magic_effect::get_assocaited_skill);

    engine.register_native_function("Potion", "IsHostile", potion::is_hostile);

    engine.register_native_function("FormList", "AddForm", form_list::add_form);
    engine.register_native_function("FormList", "Find", form_list::find);
    engine.register_native_function("FormList", "GetAt", form_list::get_at);
    engine.register_native_function("FormList", "GetSize", form_list::get_size);
    engine.register_native_function("FormList", "HasForm", form_list::has_form);
    engine.register_native_function("FormList", "RemoveAddedForm", form_list::remove_added_form);
    engine.register_native_function("FormList", "Revert", form_list::revert);

    engine.register_native_function("Faction", "CanPayCrimeGold", faction::can_pay_crime_gold);

    engine.register_native_function("Quest", "CompleteAllObjectives", quest::complete_all_objectives);
    engine.register_native_function("Quest", "CompleteQuest", quest::complete_quest);
    engine.register_native_function("Quest", "FailAllObjectives", quest::fail_all_objectives);
    engine.register_native_function("Quest", "GetAlias", quest::get_alias);
    engine.register_native_function("Quest", "GetCurrentStageID", quest::get_current_stage_id);
    engine.register_native_function("Quest", "HasObjective", quest::has_objective);
    engine.register_native_function("Quest", "IsActive", quest::is_active);
    engine.register_native_function("Quest", "IsCompleted", quest::is_completed);
    engine.register_native_function("Quest", "IsObjectiveCompleted", quest::is_objective_completed);
    engine.register_native_function("Quest", "IsObjectiveDisplayed", quest::is_objective_displayed);
    engine.register_native_function("Quest", "IsObjectiveFailed", quest::is_objective_failed);
    engine.register_native_function("Quest", "IsRunning", quest::is_running);
    engine.register_native_function("Quest", "IsStageDone", quest::is_stage_done);
    engine.register_native_function("Quest", "IsStarting", quest::is_starting);
    engine.register_native_function("Quest", "IsStopping", quest::is_stopping);
    engine.register_native_function("Quest", "IsStopped", quest::is_stopped);
    engine.register_native_function("Quest", "Reset", quest::reset);
    engine.register_native_function("Quest", "ResetSpeechChallenges", quest::reset_speech_challenges);
    engine.register_native_function("Quest", "SetActive", quest::set_active);
    engine.register_native_function("Quest", "SetCurrentStageID", quest::set_current_stage_id);
    engine.register_native_function("Quest", "SetObjectiveCompleted", quest::set_objective_completed);
    engine.register_native_function("Quest", "SetObjectiveDisplayed", quest::set_objective_displayed);
    engine.register_native_function("Quest", "SetObjectiveFailed", quest::set_objective_failed);
    engine.register_native_function("Quest", "Start", quest::start);
    engine.register_native_function("Quest", "stop", quest::stop);
    engine.register_native_function("Quest", "UpdateCurrentInstanceGlobal", quest::update_current_instance_global);

    f4se::register(engine);
    mcm::register(engine);
}

pub struct State {
    pub core_objects: CoreObjects,
    pub real_timers: TimerSet,
    pub game_timers: TimerSet,
    pub world: *mut World,
    pub quests: *mut QuestManager,
    pub objects: Objects,
}

impl State {
    fn world(&mut self) -> &mut World {
        unsafe { &mut *self.world }
    }

    fn quests(&mut self) -> &mut QuestManager {
        unsafe { &mut *self.quests }
    }
}

/// Set of engine-builtin objects that are always loaded.
#[derive(Clone, Debug)]
pub struct CoreObjects {
    pub player: ObjectRef,
    pub health: ObjectRef,
    // SPECIAL
    pub strength: ObjectRef,
    pub perception: ObjectRef,
    pub endurance: ObjectRef,
    pub charisma: ObjectRef,
    pub intelligence: ObjectRef,
    pub agility: ObjectRef,
    pub luck: ObjectRef,
    // AI
    pub aggression: ObjectRef,
    pub confidence: ObjectRef,
    pub suspicious: ObjectRef,
}

pub struct ObjectData {
    timers: HashMap<Int, Vec<Timer>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimerId(Int);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Timer {
    RealTime(Duration),
    GameTime(Duration),
}

#[derive(Clone, Debug)]
pub struct TimerSet {
    now: Instant,
    entries: Slab<TimerEntry>,
    next: BTreeMap<Instant, usize>,
    objects: HashMap<Object, HashMap<TimerId, usize>>,
}

impl TimerSet {
    pub fn insert(&mut self, object: Object, id: TimerId, duration: Duration) {
        self.remove(object, id);

        let mut at = self.now + duration;
        // Find next slot that is not empty.
        // FIXME: This pushes tasks slightly back, and is O(n),
        // can we do it in O(log(n)) instead?
        while self.next.contains_key(&at) {
            at += Duration::from_nanos(1);
        }

        let index = self.entries.insert(TimerEntry { object, id, at });
        self.next.insert(at, index);
        self.objects.entry(object).or_default().insert(id, index);
    }

    pub fn remove(&mut self, object_id: Object, id: TimerId) {
        let Some(object) = self.objects.get_mut(&object_id) else {
            return;
        };

        let Some(index) = object.remove(&id) else {
            return;
        };

        let entry = self.entries.remove(index);
        self.next.remove(&entry.at);

        if object.is_empty() {
            self.objects.remove(&object_id);
        }
    }

    pub fn tick(&mut self, time: Duration) -> impl Iterator<Item = TimerEntry> {
        self.now += time;

        let mut timers = Vec::new();
        for (_, index) in self.next.extract_if(..self.now, |_, _| true) {
            let entry = &self.entries[index];
            timers.push(*entry);
        }

        for e in &timers {
            self.remove(e.object, e.id);
        }

        timers.into_iter()
    }
}

impl Default for TimerSet {
    fn default() -> Self {
        Self {
            now: Instant::now(),
            entries: Slab::new(),
            next: BTreeMap::new(),
            objects: HashMap::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TimerEntry {
    pub object: ObjectAddr,
    pub id: TimerId,
    pub at: Instant,
}
