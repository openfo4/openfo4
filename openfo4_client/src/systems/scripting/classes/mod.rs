mod f4se;
mod mcm;

mod activator;
mod active_magic_effect;
mod actor;
mod actor_base;
mod alias;
mod cell;
mod debug;
mod effect_shader;
mod encounter_zone;
mod faction;
mod form;
mod form_list;
mod furniture;
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
mod misc_object;
mod music_type;
mod object_reference;
mod package;
mod potion;
mod quest;
mod ref_collection_alias;
mod reference_alias;
mod scene;
mod script_object;
mod sound;
mod sound_category;
mod sound_category_snapshot;
mod spell;
mod terminal;
mod topic;
mod topic_info;
mod utility;
mod visual_effect;
mod weapon;
mod weather;

use openfo4_script::engine::Engine;
use openfo4_script::runtime::ObjectAddr;

use crate::systems::scripting::EngineExt;

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
    form::register(engine);
    actor::register(engine);
    utility::register(engine);
    visual_effect::register(engine);
    sound_category_snapshot::register(engine);
    encounter_zone::register(engine);
    weather::register(engine);
    misc_object::register(engine);
    topic::register(engine);
    package::register(engine);
    activator::register(engine);
    furniture::register(engine);
    sound_category::register(engine);
    effect_shader::register(engine);
    active_magic_effect::register(engine);
    terminal::register(engine);
    global_variable::register(engine);
    form_list::register(engine);

    engine.register_if_exists("Sound", "SetInstanceVolume", sound::set_instance_volume);
    engine.register_if_exists("Sound", "StopInstance", sound::stop_instance);
    engine.register_if_exists("Sound", "Play", sound::play);
    engine.register_if_exists("Sound", "PlayAndWait", sound::play_and_wait);

    engine.register_if_exists("Spell", "Cast", spell::cast);
    engine.register_if_exists("Spell", "IsHostile", spell::is_hostile);
    engine.register_if_exists("Spell", "RemoteCast", spell::remote_cast);


    engine.register_if_exists("Keyword", "SendStoryEvent", keyword::send_story_event);
    engine.register_if_exists("Keyword", "SendStoryEventAndWait", keyword::send_story_event_and_wait);

    engine.register_if_exists("MagicEffect", "GetAssociatedSkill", magic_effect::get_assocaited_skill);

    engine.register_if_exists("Potion", "IsHostile", potion::is_hostile);

    f4se::register(engine);
    mcm::register(engine);
}
