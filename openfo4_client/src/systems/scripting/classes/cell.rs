use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Cell", "EnableFastTravel", enable_fast_travel);
    engine.register_if_exists("Cell", "GetActorOwner", get_actor_owner);
    engine.register_if_exists("Cell", "GetFactionOwner", get_faction_owner);
    engine.register_if_exists("Cell", "IsAttached", is_attached);
    engine.register_if_exists("Cell", "IsInterior", is_interior);
    engine.register_if_exists("Cell", "IsLoaded", is_loaded);
    engine.register_if_exists("Cell", "Reset", reset);
    engine.register_if_exists("Cell", "SetActorOwner", set_actor_owner);
    engine.register_if_exists("Cell", "SetFogColor", set_fog_color);
    engine.register_if_exists("Cell", "SetFactionOwner", set_faction_owner);
    engine.register_if_exists("Cell", "SetFogPlanes", set_fog_planes);
    engine.register_if_exists("Cell", "SetFogPower", set_fog_power);
    engine.register_if_exists("Cell", "SetPublic", set_public);
}

/// `EnableFastTravel`
///
/// ```text
/// Function EnableFastTravel(bool abEnable) native
/// ```
fn enable_fast_travel(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_enable: bool,
) -> Result<(), Error> {
    todo!()
}

/// `GetActorOwner`
///
/// ```text
/// ActorBase Function GetActorOwner() native
/// ```
fn get_actor_owner(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetFactionOwner`
///
/// ```text
/// Faction Function GetFactionOwner() native
/// ```
fn get_faction_owner(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `IsAttached`
///
/// ```text
/// bool Function IsAttached() native
/// ```
fn is_attached(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsInterior`
///
/// ```text
/// bool Function IsInterior() native
/// ```
fn is_interior(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsLoaded`
///
/// ```text
/// bool Function IsLoaded() native
/// ```
fn is_loaded(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SetActorOwner`
///
/// ```text
/// Function SetActorOwner(ActorBase akActor) native
/// ```
fn set_actor_owner(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_actor: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetFactionOwner`
///
/// ```text
/// Function SetFactionOwner(Faction akFaction) native
/// ```
fn set_faction_owner(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_faction: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetFogColor`
///
/// ```text
/// Function SetFogColor(int aiNearRed, int aiNearGreen, int aiNearBlue, int aiFarRed, int aiFarGreen, int aiFarBlue) native
/// ```
fn set_fog_color(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _near_red: i32,
    _near_green: i32,
    _near_blue: i32,
    _far_red: i32,
    _far_green: i32,
    _far_blue: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetFogPlanes`
///
/// ```text
/// Function SetFogPlanes(float afNear, float afFar) native
/// ```
fn set_fog_planes(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_near: f32,
    _af_far: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetFogPower`
///
/// ```text
/// Function SetFogPower(float afPower) native
/// ```
fn set_fog_power(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _af_power: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetPublic`
///
/// ```text
/// Function SetPublic(bool abPublic) native
/// ```
fn set_public(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_public: bool,
) -> Result<(), Error> {
    todo!()
}
