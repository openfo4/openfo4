use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("Cell", "EnableFastTravel", enable_fast_travel);
    engine.register_native_function("Cell", "GetActorOwner", get_actor_owner);
    engine.register_native_function("Cell", "GetFactionOwner", get_faction_owner);
    engine.register_native_function("Cell", "IsAttached", is_attached);
    engine.register_native_function("Cell", "IsInterior", is_interior);
    engine.register_native_function("Cell", "IsLoaded", is_loaded);
    engine.register_native_function("Cell", "Reset", reset);
    engine.register_native_function("Cell", "SetActorOwner", set_actor_owner);
    engine.register_native_function("Cell", "SetFogColor", set_fog_color);
    engine.register_native_function("Cell", "SetFactionOwner", set_faction_owner);
    engine.register_native_function("Cell", "SetFogPlanes", set_fog_planes);
    engine.register_native_function("Cell", "SetFogPower", set_fog_power);
    engine.register_native_function("Cell", "SetPublic", set_public);
}

/// `EnableFastTravel`
///
/// ```text
/// Function EnableFastTravel(bool abEnable) native
/// ```
fn enable_fast_travel(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `GetActorOwner`
///
/// ```text
/// ActorBase Function GetActorOwner() native
/// ```
fn get_actor_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetFactionOwner`
///
/// ```text
/// Faction Function GetFactionOwner() native
/// ```
fn get_faction_owner(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `IsAttached`
///
/// ```text
/// bool Function IsAttached() native
/// ```
fn is_attached(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsInterior`
///
/// ```text
/// bool Function IsInterior() native
/// ```
fn is_interior(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsLoaded`
///
/// ```text
/// bool Function IsLoaded() native
/// ```
fn is_loaded(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `SetActorOwner`
///
/// ```text
/// Function SetActorOwner(ActorBase akActor) native
/// ```
fn set_actor_owner(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_actor: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetFactionOwner`
///
/// ```text
/// Function SetFactionOwner(Faction akFaction) native
/// ```
fn set_faction_owner(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_faction: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `SetFogColor`
///
/// ```text
/// Function SetFogColor(int aiNearRed, int aiNearGreen, int aiNearBlue, int aiFarRed, int aiFarGreen, int aiFarBlue) native
/// ```
fn set_fog_color(
    caller: Caller<'_, State>,
    self_ptr: Object,
    near_red: i32,
    near_green: i32,
    near_blue: i32,
    far_red: i32,
    far_green: i32,
    far_blue: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetFogPlanes`
///
/// ```text
/// Function SetFogPlanes(float afNear, float afFar) native
/// ```
fn set_fog_planes(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_near: f32,
    af_far: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetFogPower`
///
/// ```text
/// Function SetFogPower(float afPower) native
/// ```
fn set_fog_power(
    caller: Caller<'_, State>,
    self_ptr: Object,
    af_power: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPublic`
///
/// ```text
/// Function SetPublic(bool abPublic) native
/// ```
fn set_public(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_public: bool,
) -> Result<(), Exception> {
    todo!()
}
