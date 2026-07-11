use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("ActorBase", "GetClass", get_class);
    engine.register_if_exists("ActorBase", "GetDeadCount", get_dead_count);
    engine.register_if_exists("ActorBase", "GetGiftFilter", get_gift_filter);
    engine.register_if_exists("ActorBase", "GetRace", get_race);
    engine.register_if_exists("ActorBase", "GetLevel", get_level);
    engine.register_if_exists("ActorBase", "GetLevelExact", get_level_exact);
    engine.register_if_exists("ActorBase", "GetSex", get_sex);
    engine.register_if_exists("ActorBase", "GetUniqueActor", get_unique_actor);
    engine.register_if_exists("ActorBase", "IsEssential", is_essential);
    engine.register_if_exists("ActorBase", "IsInvulnerable", is_invulnerable);
    engine.register_if_exists("ActorBase", "IsProtected", is_protected);
    engine.register_if_exists("ActorBase", "IsUnique", is_unique);
    engine.register_if_exists("ActorBase", "SetEssential", set_essential);
    engine.register_if_exists("ActorBase", "SetInvulnerable", set_invulnerable);
    engine.register_if_exists("ActorBase", "SetProtected", set_protected);
    engine.register_if_exists("ActorBase", "SetOutfit", set_outfit);
}

/// `GetClass`
///
/// ```text
/// Class Function GetClass() native
/// ```
fn get_class(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetDeadCount`
///
/// ```text
/// int Function GetDeadCount() native
/// ```
fn get_dead_count(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetGiftFilter`
///
/// ```text
/// FormList Function GetGiftFilter() native
/// ```
fn get_gift_filter(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetRace`
///
/// ```text
/// Race Function GetRace() native
/// ```
fn get_race(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
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

/// `GetLevelExact`
///
/// ```text
/// int Function GetLevelExact() native
/// ```
fn get_level_exact(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetSex`
///
/// ```text
/// int Function GetSex() native
/// ```
fn get_sex(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetUniqueActor`
///
/// ```text
/// Actor Function GetUniqueActor() native
/// ```
fn get_unique_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `IsEssential`
///
/// ```text
/// bool Function IsEssential() native
/// ```
fn is_essential(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInvulnerable`
///
/// ```text
/// bool Function IsInvulnerable() native
/// ```
fn is_invulnerable(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsProtected`
///
/// ```text
/// bool Function IsProtected() native
/// ```
fn is_protected(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsUnique`
///
/// ```text
/// bool Function IsUnique() native
/// ```
fn is_unique(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `SetEssential`
///
/// ```text
/// Function SetEssential(bool abEssential) native
/// ```
fn set_essential(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_essential: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetInvulnerable`
///
/// ```text
/// Function SetInvulnerable(bool abInvulnerable) native
/// ```
fn set_invulnerable(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_invulnerable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetProtected`
///
/// ```text
/// Function SetProtected(bool abProtected) native
/// ```
fn set_protected(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_protected: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetOutfit`
///
/// ```text
/// Function SetOutfit(Outfit akOutfit, bool abSleepOutfit) native
/// ```
fn set_outfit(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_outfit: Object,
    ab_sleep_outfit: bool,
) -> Result<(), Exception> {
    todo!()
}
