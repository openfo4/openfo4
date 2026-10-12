use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::global::{Faction, FactionReaction};
use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Faction", "CanPayCrimeGold", can_pay_crime_gold);
    engine.register_if_exists("Faction", "GetCrimeGold", get_crime_gold);
    engine.register_if_exists("Faction", "GetCrimeGoldNonViolent", get_crime_gold_non_violent);
    engine.register_if_exists("Faction", "GetCrimeGoldViolent", get_crime_gold_violent);
    engine.register_if_exists("Faction", "GetInfamy", get_infamy);
    engine.register_if_exists("Faction", "GetInfamyNonViolent", get_infamy_non_violent);
    engine.register_if_exists("Faction", "GetInfamyViolent", get_infamy_violent);
    engine.register_if_exists("Faction", "GetFactionReaction", get_faction_reaction);
    engine.register_if_exists("Faction", "GetStolenItemValueCrime", get_stolen_item_value_crime);
    engine.register_if_exists("Faction", "GetStolenItemValueNoCrime", get_stolen_item_value_no_crime);
    engine.register_if_exists("Faction", "IsFactionInCrimeGroup", is_faction_in_crime_group);
    engine.register_if_exists("Faction", "IsPlayerEnemy", is_player_enemy);
    engine.register_if_exists("Faction", "IsPlayerExpelled", is_player_expelled);
    engine.register_if_exists("Faction", "ModCrimeGold", mod_crime_gold);
    engine.register_if_exists("Faction", "PlayerPayCrimeGold", player_pay_crime_gold);
    engine.register_if_exists("Faction", "SendAssaultAlarm", send_assault_alarm);
    engine.register_if_exists("Faction", "SendPlayerToJail", send_player_to_jail);
    engine.register_if_exists("Faction", "SetAlly", set_ally);
    engine.register_if_exists("Faction", "SetCrimeGold", set_crime_gold);
    engine.register_if_exists("Faction", "SetCrimeGoldViolent", set_crime_gold_violent);
    engine.register_if_exists("Faction", "SetEnemy", set_enemy);
    engine.register_if_exists("Faction", "SetPlayerEnemy", set_player_enemy);
    engine.register_if_exists("Faction", "SetPlayerExpelled", set_player_expelled);
}

/// `CanPayCrimeGold`
///
/// ```text
/// bool Function CanPayCrimeGold() native
/// ```
fn can_pay_crime_gold(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `GetCrimeGold`
///
/// ```text
/// int Function GetCrimeGold() native
/// ```
fn get_crime_gold(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetCrimeGoldNonViolent`
///
/// ```text
/// int Function GetCrimeGoldNonViolent() native
/// ```
fn get_crime_gold_non_violent(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetCrimeGoldViolent`
///
/// ```text
/// int Function GetCrimeGoldViolent() native
/// ```
fn get_crime_gold_violent(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetInfamy`
///
/// ```text
/// int Function GetInfamy() native
/// ```
fn get_infamy(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetInfamyNonViolent`
///
/// ```text
/// int Function GetInfamyNonViolent() native
/// ```
fn get_infamy_non_violent(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetInfamyViolent`
///
/// ```text
/// int Function GetInfamyViolent() native
/// ```
fn get_infamy_violent(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetFactionReaction`
///
/// ```text
/// int Function GetFactionReaction(Actor akOther) native
/// ```
fn get_faction_reaction(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetStolenItemValueCrime`
///
/// ```text
/// int Function GetStolenItemValueCrime() native
/// ```
fn get_stolen_item_value_crime(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetStolenItemValueNoCrime`
///
/// ```text
/// int Function GetStolenItemValueNoCrime() native
/// ```
fn get_stolen_item_value_no_crime(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `IsFactionInCrimeGroup`
///
/// ```text
/// bool Function IsFactionInCrimeGroup(Faction akOther) native
/// ```
fn is_faction_in_crime_group(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsPlayerEnemy`
///
/// ```text
/// bool Function IsPlayerEnemy() native
/// ```
fn is_player_enemy(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    with_faction(&caller, self_ptr, |faction| faction.player_enemy).unwrap_or(false)
}

/// `IsPlayerExpelled`
///
/// ```text
/// bool Function IsPlayerExpelled() native
/// ```
fn is_player_expelled(caller: Caller<'_, State>, self_ptr: Object) -> bool {
    with_faction(&caller, self_ptr, |faction| faction.player_expelled).unwrap_or(false)
}

/// `ModCrimeGold`
///
/// ```text
/// Function ModCrimeGold(int aiAmount, bool abViolent) native
/// ```
fn mod_crime_gold(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_amount: i32,
    _ab_violent: bool,
) -> Result<(), Error> {
    todo!()
}

/// `PlayerPayCrimeGold`
///
/// ```text
/// Function PlayerPayCrimeGold(bool abRemoveStolenItems, bool abGoToJail) native
/// ```
fn player_pay_crime_gold(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_remove_stolen_items: bool,
    _ab_go_to_jail: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SendAssaultAlarm`
///
/// ```text
/// Function SendAssaultAlarm() native
/// ```
fn send_assault_alarm(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SendPlayerToJail`
///
/// ```text
/// Function SendPlayerToJail(bool abRemoveInventory, bool abRealJail) native
/// ```
fn send_player_to_jail(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_remove_inventory: bool,
    _ab_real_jail: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetAlly`
///
/// ```text
/// Function SetAlly(Faction akOther, bool abSelfIsFriendToOther, bool abOtherIsFriendToSelf) native
/// ```
fn set_ally(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
    self_is_friend_to_other: bool,
    other_is_friend_to_self: bool,
) {
    let Some(self_faction) = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
    else {
        debug_assert!(self_ptr.is_null());
        return;
    };
    let Some(other_faction) = caller
        .data()
        .objects
        .get_by_addr(other)
        .and_then(|s| s.record())
    else {
        debug_assert!(other.is_null());
        return;
    };

    with_faction_mut(&mut caller, self_ptr, |faction| {
        let reaction = if self_is_friend_to_other {
            FactionReaction::Friend
        } else {
            FactionReaction::Ally
        };

        faction.relations.insert(other_faction, reaction);
    });

    with_faction_mut(&mut caller, other, |faction| {
        let reaction = if other_is_friend_to_self {
            FactionReaction::Friend
        } else {
            FactionReaction::Ally
        };

        faction.relations.insert(self_faction, reaction);
    });
}

/// `SetCrimeGold`
///
/// ```text
/// Function SetCrimeGold(int aiGold) native
/// ```
fn set_crime_gold(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_gold: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetCrimeGoldViolent`
///
/// ```text
/// Function SetCrimeGoldViolent(int aiGold) native
/// ```
fn set_crime_gold_violent(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ai_gold: i32,
) -> Result<(), Error> {
    todo!()
}

/// `SetEnemy`
///
/// ```text
/// Function SetEnemy(Faction akOther, bool abSelfIsNeutralToOther, bool abOtherIsNeutralToSelf) native
/// ```
fn set_enemy(
    mut caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
    self_is_neutral_to_other: bool,
    other_is_neutral_to_self: bool,
) {
    let Some(self_faction) = caller
        .data()
        .objects
        .get_by_addr(self_ptr)
        .and_then(|s| s.record())
    else {
        debug_assert!(self_ptr.is_null());
        return;
    };
    let Some(other_faction) = caller
        .data()
        .objects
        .get_by_addr(other)
        .and_then(|s| s.record())
    else {
        debug_assert!(other.is_null());
        return;
    };

    with_faction_mut(&mut caller, self_ptr, |faction| {
        let reaction = if self_is_neutral_to_other {
            FactionReaction::Neutral
        } else {
            FactionReaction::Enemy
        };

        faction.relations.insert(other_faction, reaction);
    });

    with_faction_mut(&mut caller, other, |faction| {
        let reaction = if other_is_neutral_to_self {
            FactionReaction::Neutral
        } else {
            FactionReaction::Enemy
        };

        faction.relations.insert(self_faction, reaction);
    });
}

/// `SetPlayerEnemy`
///
/// ```text
/// Function SetPlayerEnemy(bool abIsEnemy) native
/// ```
fn set_player_enemy(mut caller: Caller<'_, State>, self_ptr: Object, is_enemy: bool) {
    with_faction_mut(&mut caller, self_ptr, |faction| {
        faction.player_enemy = is_enemy;
    });
}

/// `SetPlayerExpelled`
///
/// ```text
/// Function SetPlayerExpelled(bool abIsExpelled) native
/// ```
fn set_player_expelled(mut caller: Caller<'_, State>, self_ptr: Object, is_expelled: bool) {
    with_faction_mut(&mut caller, self_ptr, |faction| {
        faction.player_expelled = is_expelled;
    });
}

fn with_faction<T, F>(caller: &Caller<'_, State>, faction: Object, f: F) -> Option<T>
where
    F: FnOnce(&Faction) -> T,
{
    let faction = caller
        .data()
        .objects
        .get_by_addr(faction)
        .and_then(|s| s.record())?;
    let factions = caller.data().globals.factions.lock();
    let faction = factions.get(&faction)?;
    Some(f(faction))
}

fn with_faction_mut<T, F>(caller: &mut Caller<'_, State>, faction: Object, f: F) -> Option<T>
where
    F: FnOnce(&mut Faction) -> T,
{
    let faction = caller
        .data()
        .objects
        .get_by_addr(faction)
        .and_then(|s| s.record())?;
    let mut factions = caller.data().globals.factions.lock();
    let faction = factions.get_mut(&faction)?;
    Some(f(faction))
}
