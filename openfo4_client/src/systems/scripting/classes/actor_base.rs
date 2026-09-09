use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

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
async fn get_class(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let data = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let data = data.data().actor_base().unwrap();
    let Some(class) = data.class else {
        return Ok(Object::NULL);
    };

    let class = caller
        .data()
        .get_or_create_record_class(class, "Class")
        .await?;
    Ok(class.get().object.addr())
}

/// `GetDeadCount`
///
/// ```text
/// int Function GetDeadCount() native
/// ```
fn get_dead_count(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetGiftFilter`
///
/// ```text
/// FormList Function GetGiftFilter() native
/// ```
fn get_gift_filter(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `GetRace`
///
/// ```text
/// Race Function GetRace() native
/// ```
async fn get_race(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    let data = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let data = data.data().actor_base().unwrap();

    let race = caller
        .data()
        .get_or_create_record_class(data.race, "Race")
        .await?;
    Ok(race.get().object.addr())
}

/// `GetLevel`
///
/// ```text
/// int Function GetLevel() native
/// ```
fn get_level(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetLevelExact`
///
/// ```text
/// int Function GetLevelExact() native
/// ```
fn get_level_exact(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetSex`
///
/// ```text
/// int Function GetSex() native
/// ```
fn get_sex(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetUniqueActor`
///
/// ```text
/// Actor Function GetUniqueActor() native
/// ```
fn get_unique_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}

/// `IsEssential`
///
/// ```text
/// bool Function IsEssential() native
/// ```
fn is_essential(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    let data = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let data = data.data().actor_base().unwrap();
    data.is_essential
}

/// `IsInvulnerable`
///
/// ```text
/// bool Function IsInvulnerable() native
/// ```
fn is_invulnerable(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    let data = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let data = data.data().actor_base().unwrap();
    data.is_invulnerable
}

/// `IsProtected`
///
/// ```text
/// bool Function IsProtected() native
/// ```
fn is_protected(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    let data = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let data = data.data().actor_base().unwrap();
    data.is_protected
}

/// `IsUnique`
///
/// ```text
/// bool Function IsUnique() native
/// ```
fn is_unique(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    let data = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let data = data.data().actor_base().unwrap();
    data.is_unique
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    todo!()
}
