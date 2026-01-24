mod cell;
mod favorites_manger;
mod form;
mod game;
mod input;
mod location;
mod math;
mod object_reference;
mod script_object;
mod ui;
mod water_type;
mod weapon;

use openfo4_script::engine::Engine;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("ObjectReference", "ApplyMaterialSwap", object_reference::apply_material_swap);
    engine.register_native_function("ObjectReference", "AttachWire", object_reference::attach_wire);
    engine.register_native_function("ObjectReference", "GetAllMods", object_reference::get_all_mods);
    engine.register_native_function("ObjectReference", "GetConnectPoints", object_reference::get_connect_points);
    engine.register_native_function("ObjectReference", "GetConnectedObjects", object_reference::get_connected_objects);
    engine.register_native_function("ObjectReference", "GetDisplayName", object_reference::get_display_name);
    engine.register_native_function("ObjectReference", "GetInventoryItems", object_reference::get_inventory_items);
    engine.register_native_function("ObjectReference", "GetInventoryWeight", object_reference::get_inventory_weight);
    engine.register_native_function("ObjectReference", "GetMaterialSwap", object_reference::get_material_swap);
    engine.register_native_function("ObjectReference", "SetMaterialSwap", object_reference::set_material_swap);
    engine.register_native_function("ObjectReference", "TransmitConnectedPower", object_reference::transmit_connected_power);

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
}
