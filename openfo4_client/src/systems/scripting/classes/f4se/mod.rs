mod actor_base;
mod armor;
mod cell;
mod default_object;
mod encounter_zone;
mod f4se;
mod favorites_manger;
mod form;
mod game;
mod head_part;
mod input;
mod location;
mod math;
mod misc_object;
mod object_mod;
mod object_reference;
mod perk;
mod script_object;
mod ui;
mod utility;
mod water_type;
mod weapon;

use openfo4_script::engine::Engine;

pub fn register(engine: &mut Engine) {
    f4se::register(engine);
    math::register(engine);
    ui::register(engine);
    form::register(engine);
    game::register(engine);
    weapon::register(engine);
    favorites_manger::register(engine);
    location::register(engine);
    input::register(engine);
    script_object::register(engine);
    cell::register(engine);
    water_type::register(engine);
    object_mod::register(engine);
    default_object::register(engine);
    utility::register(engine);
    misc_object::register(engine);
    armor::register(engine);
    actor_base::register(engine);
    perk::register(engine);
    encounter_zone::register(engine);
    object_reference::register(engine);
    head_part::register(engine);
}
