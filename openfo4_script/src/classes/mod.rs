use crate::Engine;

mod form;
mod game;
mod math;
mod message;
mod object_reference;
mod script_object;

pub type Int = i32;
pub type Float = f32;
pub type Object = isize;
pub type Bool = bool;

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
    engine.register_native_function(
        "ObjectReference",
        "SetActorOwner",
        object_reference::set_actor_owner,
    );
    engine.register_native_function(
        "ObjectReference",
        "GetBaseObject",
        object_reference::get_base_object,
    );

    engine.register_native_function("Game", "GetPlayer", game::get_player);

    engine.register_native_function("Message", "Show", message::show);

    engine.register_native_function(
        "ScriptObject",
        "CancelTimerGameTime",
        script_object::cancel_timer_game_time,
    );

    engine.register_native_function("Form", "GetFormID", form::get_form_id);
    engine.register_native_function("Form", "GetGoldValue", form::get_gold_value);
    engine.register_native_function("Form", "HasKeyword", form::has_keyword);
    engine.register_native_function(
        "Form",
        "HasKeywordInFormList",
        form::has_keyword_in_form_list,
    );
    engine.register_native_function("Form", "PlayerKnows", form::player_knows);
}

pub struct State {}
