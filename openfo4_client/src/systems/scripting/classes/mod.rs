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
mod image_space_modifier;
mod input_enable_layer;
mod keyword;
mod leveled_actor;
mod leveled_item;
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
mod topic_info;
mod utility;
mod weapon;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use futures::future::BoxFuture;
use hashbrown::{HashMap, HashSet};
use hecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Exception, ObjectAddr, ObjectPool, ObjectRef};
use slab::Slab;

use crate::components::Player;
use crate::systems::global::GlobalProperties;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::effects::{DynamicEffect, Effect, EffectId, Effects};
use crate::systems::scripting::loader::{ScriptObject, ScriptObjectId};
use crate::systems::scripting::objects::{ScriptObjects, ScriptsRef};
use crate::systems::scripting::registration::RegistrationSet;

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
    quest::register(engine);
    leveled_item::register(engine);
    leveled_actor::register(engine);
    topic_info::register(engine);
    faction::register(engine);
    image_space_modifier::register(engine);
    message::register(engine);

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


    f4se::register(engine);
    mcm::register(engine);
}

pub struct State {
    pub core_objects: CoreObjects,
    pub real_timers: TimerSet,
    pub game_timers: TimerSet,
    pub world: *mut World,
    pub quests: *mut QuestManager,
    pub entities: *const ScriptEntities,
    pub globals: *mut GlobalProperties,
    pub registration: RegistrationSet,
    pub objects: ScriptObjects,
    pub object_data: HashMap<ObjectAddr, ObjectData>,
    pub game_entities: HashMap<ObjectAddr, ScriptObjectId>,
    pub aliases: AliasSet,
    pub effects: Effects,
    pub wait_on_effect: Option<EffectId>,
}

#[derive(Clone, Debug, Default)]
pub struct AliasSet {
    aliases: HashMap<ObjectAddr, AliasState>,
    /// Maps "real" objects to aliases.
    objects: HashMap<ObjectAddr, HashSet<ObjectAddr>>,
}

impl AliasSet {
    pub fn get(&self, addr: ObjectAddr) -> Option<AliasState> {
        self.aliases.get(&addr).copied()
    }

    pub fn set(&mut self, alias: ObjectAddr, object: Option<ObjectAddr>) {
        if let Some(state) = self.aliases.get_mut(&alias) {
            if state.object == object {
                return;
            }

            if let Some(prev_object) = state.object {
                self.objects.get_mut(&prev_object).unwrap().remove(&alias);
            }

            state.object = object;

            if let Some(object) = object {
                self.objects.entry(object).or_default().insert(alias);
            }
        }
    }

    /// Returns all alias objects that point to the given object.
    pub fn get_alises_for_object(
        &self,
        object: ObjectAddr,
    ) -> impl Iterator<Item = ObjectAddr> + '_ {
        self.objects.get(&object).into_iter().flatten().copied()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AliasState {
    /// The quest that owns this alias.
    pub quest: RecordId,
    /// The real object that this alias points to.
    pub object: Option<ObjectAddr>,
}

impl State {
    fn world(&self) -> &World {
        unsafe { &mut *self.world }
    }

    fn quests(&mut self) -> &mut QuestManager {
        unsafe { &mut *self.quests }
    }

    fn globals(&self) -> &GlobalProperties {
        unsafe { &*self.globals }
    }

    fn globals_mut(&mut self) -> &mut GlobalProperties {
        unsafe { &mut *self.globals }
    }

    fn entities(&self) -> &ScriptEntities {
        unsafe { &*self.entities }
    }

    pub fn get_script_object(&self, entity: Entity) -> ScriptObject {
        if let Ok(object) = self.world().get::<&ScriptObject>(entity) {
            return ScriptObject::clone(&object);
        }

        todo!()
    }

    /// Returns the game world entity associated with a script object, if any exists.
    pub fn entity(&self, addr: ObjectAddr) -> Option<Entity> {
        if addr == self.core_objects.player.addr() {
            let (entity, _) = self.world().query::<&Player>().iter().next().unwrap();
            return Some(entity);
        }

        let id = *self.game_entities.get(&addr)?;
        self.entities().0.get_left(&id).copied()
    }

    pub fn run_effect_and_wait<T>(&mut self, effect: T) -> Result<Option<T::Result>, Exception>
    where
        T: Effect,
    {
        // Previous effect complete, wake up.
        if let Some(effect) = self.wait_on_effect.take() {
            let value = self
                .effects
                .take_value(effect)
                .map(|v| v.downcast().ok())
                .flatten()
                .map(|v| *v);

            self.effects.discard(effect);

            return Ok(value);
        }

        let id = self.effects.push(effect);
        self.wait_on_effect = Some(id);
        Err(Exception::Suspend)
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State").finish_non_exhaustive()
    }
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimerId(pub Int);

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

#[derive(Clone, Debug)]
pub struct BaseObjects {}

#[derive(Clone, Debug)]
pub enum ObjectData {
    InputEnableLayer { layer: usize },
}
