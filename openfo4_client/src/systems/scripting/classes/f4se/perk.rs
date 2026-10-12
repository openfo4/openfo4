use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Perk", "IsPlayable", is_playable);
    engine.register_if_exists("Perk", "IsHidden", is_hidden);
    engine.register_if_exists("Perk", "GetLevel", get_level);
    engine.register_if_exists("Perk", "GetNumRanks", get_num_ranks);
    engine.register_if_exists("Perk", "GetNextPerk", get_next_perk);
    engine.register_if_exists("Perk", "GetSWFPath", get_swf_path);
    engine.register_if_exists("Perk", "IsEligible", is_eligible);
}

/// `IsPlayable`
///
/// ```text
/// bool Function IsPlayable() native
/// ```
fn is_playable(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsHidden`
///
/// ```text
/// bool Function IsHidden() native
/// ```
fn is_hidden(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetLevel`
///
/// ```text
/// int Function GetLevel() native
/// ```
fn get_level(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetNumRanks`
///
/// ```text
/// int Function GetNumRanks() native
/// ```
fn get_num_ranks(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetNextPerk`
///
/// ```text
/// Perk Function GetNextPerk() native
/// ```
fn get_next_perk(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetSWFPath`
///
/// ```text
/// string Function GetSWFPath() native
/// ```
fn get_swf_path(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `IsEligible`
///
/// ```text
/// bool Function IsEligible(Actor akActor) native
/// ```
fn is_eligible(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_actor: Object,
) -> Result<bool, Error> {
    todo!()
}
