use crate::Engine;

mod form;
mod game;
mod math;
mod message;
mod object_reference;
mod script_object;

pub type Int = i32;
pub type Float = f32;

pub fn register_builtins(engine: &mut Engine) {
    engine.register_native_function("Math", "sin", math::sin);
    engine.register_native_function("Math", "cos", math::cos);
    engine.register_native_function("Math", "tan", math::tan);
    engine.register_native_function("Math", "acos", math::acos);
    engine.register_native_function("Math", "asin", math::asin);
    engine.register_native_function("Math", "atan", math::atan);

    engine.register_native_function("ObjectReference", "GetValue", object_reference::get_value);
    engine.register_native_function("ObjectReference", "SetValue", object_reference::set_value);
    engine.register_native_function(
        "ObjectReference",
        "GetBaseValue",
        object_reference::get_base_value,
    );

    engine.register_native_function("Game", "GetPlayer", game::get_player);

    engine.register_native_function("Message", "Show", message::show);
}

pub struct State {}
