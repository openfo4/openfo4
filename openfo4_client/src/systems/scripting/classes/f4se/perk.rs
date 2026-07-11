use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

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
fn is_playable(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsHidden`
///
/// ```text
/// bool Function IsHidden() native
/// ```
fn is_hidden(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetLevel`
///
/// ```text
/// int Function GetLevel() native
/// ```
fn get_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetNumRanks`
///
/// ```text
/// int Function GetNumRanks() native
/// ```
fn get_num_ranks(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetNextPerk`
///
/// ```text
/// Perk Function GetNextPerk() native
/// ```
fn get_next_perk(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetSWFPath`
///
/// ```text
/// string Function GetSWFPath() native
/// ```
fn get_swf_path(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `IsEligible`
///
/// ```text
/// bool Function IsEligible(Actor akActor) native
/// ```
fn is_eligible(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
) -> Result<bool, Exception> {
    todo!()
}
