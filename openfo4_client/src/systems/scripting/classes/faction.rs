use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Faction", "CanPayCrimeGold", can_pay_crime_gold);
    engine.register_native_function("Faction", "GetCrimeGold", get_crime_gold);
    engine.register_native_function("Faction", "GetCrimeGoldNonViolent", get_crime_gold_non_violent);
    engine.register_native_function("Faction", "GetCrimeGoldViolent", get_crime_gold_violent);
    engine.register_native_function("Faction", "GetInfamy", get_infamy);
    engine.register_native_function("Faction", "GetInfamyNonViolent", get_infamy_non_violent);
    engine.register_native_function("Faction", "GetInfamyViolent", get_infamy_violent);
    engine.register_native_function("Faction", "GetFactionReaction", get_faction_reaction);
    engine.register_native_function("Faction", "GetStolenItemValueCrime", get_stolen_item_value_crime);
    engine.register_native_function("Faction", "GetStolenItemValueNoCrime", get_stolen_item_value_no_crime);
    engine.register_native_function("Faction", "IsFactionInCrimeGroup", is_faction_in_crime_group);
    engine.register_native_function("Faction", "IsPlayerEnemy", is_player_enemy);
    engine.register_native_function("Faction", "IsPlayerExpelled", is_player_expelled);
    engine.register_native_function("Faction", "ModCrimeGold", mod_crime_gold);
    engine.register_native_function("Faction", "PlayerPayCrimeGold", player_pay_crime_gold);
    engine.register_native_function("Faction", "SendAssaultAlarm", send_assault_alarm);
    engine.register_native_function("Faction", "SendPlayerToJail", send_player_to_jail);
    engine.register_native_function("Faction", "SetAlly", set_ally);
    engine.register_native_function("Faction", "SetCrimeGold", set_crime_gold);
    engine.register_native_function("Faction", "SetCrimeGoldViolent", set_crime_gold_violent);
    engine.register_native_function("Faction", "SetEnemy", set_enemy);
    engine.register_native_function("Faction", "SetPlayerEnemy", set_player_enemy);
    engine.register_native_function("Faction", "SetPlayerExpelled", set_player_expelled);
}

/// `CanPayCrimeGold`
///
/// ```text
/// bool Function CanPayCrimeGold() native
/// ```
fn can_pay_crime_gold(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `GetCrimeGold`
///
/// ```text
/// int Function GetCrimeGold() native
/// ```
fn get_crime_gold(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetCrimeGoldNonViolent`
///
/// ```text
/// int Function GetCrimeGoldNonViolent() native
/// ```
fn get_crime_gold_non_violent(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetCrimeGoldViolent`
///
/// ```text
/// int Function GetCrimeGoldViolent() native
/// ```
fn get_crime_gold_violent(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetInfamy`
///
/// ```text
/// int Function GetInfamy() native
/// ```
fn get_infamy(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetInfamyNonViolent`
///
/// ```text
/// int Function GetInfamyNonViolent() native
/// ```
fn get_infamy_non_violent(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetInfamyViolent`
///
/// ```text
/// int Function GetInfamyViolent() native
/// ```
fn get_infamy_violent(caller: Caller<'_, State>, self_ptr: Object) -> Result<i32, Exception> {
    todo!()
}

/// `GetFactionReaction`
///
/// ```text
/// int Function GetFactionReaction(Actor akOther) native
/// ```
fn get_faction_reaction(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetStolenItemValueCrime`
///
/// ```text
/// int Function GetStolenItemValueCrime() native
/// ```
fn get_stolen_item_value_crime(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetStolenItemValueNoCrime`
///
/// ```text
/// int Function GetStolenItemValueNoCrime() native
/// ```
fn get_stolen_item_value_no_crime(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `IsFactionInCrimeGroup`
///
/// ```text
/// bool Function IsFactionInCrimeGroup(Faction akOther) native
/// ```
fn is_faction_in_crime_group(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsPlayerEnemy`
///
/// ```text
/// bool Function IsPlayerEnemy() native
/// ```
fn is_player_enemy(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsPlayerExpelled`
///
/// ```text
/// bool Function IsPlayerExpelled() native
/// ```
fn is_player_expelled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `ModCrimeGold`
///
/// ```text
/// Function ModCrimeGold(int aiAmount, bool abViolent) native
/// ```
fn mod_crime_gold(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_amount: i32,
    ab_violent: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `PlayerPayCrimeGold`
///
/// ```text
/// Function PlayerPayCrimeGold(bool abRemoveStolenItems, bool abGoToJail) native
/// ```
fn player_pay_crime_gold(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_remove_stolen_items: bool,
    ab_go_to_jail: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SendAssaultAlarm`
///
/// ```text
/// Function SendAssaultAlarm() native
/// ```
fn send_assault_alarm(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `SendPlayerToJail`
///
/// ```text
/// Function SendPlayerToJail(bool abRemoveInventory, bool abRealJail) native
/// ```
fn send_player_to_jail(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_remove_inventory: bool,
    ab_real_jail: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetAlly`
///
/// ```text
/// Function SetAlly(Faction akOther, bool abSelfIsFriendToOther, bool abOtherIsFriendToSelf) native
/// ```
fn set_ally(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
    ab_self_is_friend_to_other: bool,
    ab_other_is_friend_to_self: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCrimeGold`
///
/// ```text
/// Function SetCrimeGold(int aiGold) native
/// ```
fn set_crime_gold(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_gold: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCrimeGoldViolent`
///
/// ```text
/// Function SetCrimeGoldViolent(int aiGold) native
/// ```
fn set_crime_gold_violent(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ai_gold: i32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetEnemy`
///
/// ```text
/// Function SetEnemy(Faction akOther, bool abSelfIsNeutralToOther, bool abOtherIsNeutralToSelf) native
/// ```
fn set_enemy(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
    ab_self_is_neutral_to_other: bool,
    ab_other_is_neutral_to_self: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPlayerEnemy`
///
/// ```text
/// Function SetPlayerEnemy(bool abIsEnemy) native
/// ```
fn set_player_enemy(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_is_enemy: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `SetPlayerExpelled`
///
/// ```text
/// Function SetPlayerExpelled(bool abIsExpelled) native
/// ```
fn set_player_expelled(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_is_expelled: bool,
) -> Result<(), Exception> {
    todo!()
}
