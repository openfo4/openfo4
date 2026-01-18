use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("InputEnableLayer", "Create", create);
    engine.register_native_function("InputEnableLayer", "Delete", delete);
    engine.register_native_function("InputEnableLayer", "DisablePlayerControls", disable_player_controls);
    engine.register_native_function("InputEnableLayer", "EnableActivate", enable_activate);
    engine.register_native_function("InputEnableLayer", "EnableVATS", enable_vats);
    engine.register_native_function("InputEnableLayer", "EnableCamSwitch", enable_cam_switch);
    engine.register_native_function("InputEnableLayer", "EnableFastTravel", enable_fast_travel);
    engine.register_native_function("InputEnableLayer", "EnableFavorites", enable_favorites);
    engine.register_native_function("InputEnableLayer", "EnableFighting", enable_fighting);
    engine.register_native_function("InputEnableLayer", "EnableJournal", enable_journal);
    engine.register_native_function("InputEnableLayer", "EnableJumping", enable_jumping);
    engine.register_native_function("InputEnableLayer", "EnableLooking", enable_looking);
    engine.register_native_function("InputEnableLayer", "EnableMenu", enable_menu);
    engine.register_native_function("InputEnableLayer", "EnableMovement", enable_movement);
    engine.register_native_function("InputEnableLayer", "EnableRunning", enable_running);
    engine.register_native_function("InputEnableLayer", "EnableSprinting", enable_sprinting);
    engine.register_native_function("InputEnableLayer", "EnableZKey", enable_z_key);
    engine.register_native_function("InputEnableLayer", "EnablePlayerControls", enable_player_controls);
    engine.register_native_function("InputEnableLayer", "EnableSneaking", enable_sneaking);
    engine.register_native_function("InputEnableLayer", "IsActivateEnabled", is_activate_enabled);
    engine.register_native_function("InputEnableLayer", "IsVATSEnabled", is_vats_enabled);
    engine.register_native_function("InputEnableLayer", "IsCamSwitchEnabled", is_cam_switch_enabled);
    engine.register_native_function("InputEnableLayer", "IsFastTravelEnabled", is_fast_travel_enabled);
    engine.register_native_function("InputEnableLayer", "IsFavoritesEnabled", is_favorites_enabled);
    engine.register_native_function("InputEnableLayer", "IsFightingEnabled", is_fighting_enabled);
    engine.register_native_function("InputEnableLayer", "IsJournalEnabled", is_journal_enabled);
    engine.register_native_function("InputEnableLayer", "IsJumpingEnabled", is_jumping_enabled);
    engine.register_native_function("InputEnableLayer", "IsLookingEnabled", is_looking_enabled);
    engine.register_native_function("InputEnableLayer", "IsMenuEnabled", is_menu_enabled);
    engine.register_native_function("InputEnableLayer", "IsMovementEnabled", is_movement_enabled);
    engine.register_native_function("InputEnableLayer", "IsRunningEnabled", is_running_enabled);
    engine.register_native_function("InputEnableLayer", "IsSprintingEnabled", is_sprinting_enabled);
    engine.register_native_function("InputEnableLayer", "IsZKeyEnabled", is_z_key_enabled);
    engine.register_native_function("InputEnableLayer", "IsSneakingEnabled", is_sneaking_enabled);
    engine.register_native_function("InputEnableLayer", "Reset", reset);
}

/// `Create`
///
/// ```text
/// InputEnableLayer Function() native global
/// ```
fn create(caller: Caller<'_, State>) -> Result<Object, Exception> {
    todo!()
}

/// `Delete`
///
/// ```text
/// Function() native
/// ```
fn delete(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `DisablePlayerControls`
///
/// ```text
// Function DisablePlayerControls(bool abMovement, bool abFighting, bool abCamSwitch, bool abLooking, bool abSneaking, bool abMenu, bool abActivate, bool abJournalTabs, bool abVATS, bool abFavorites, bool abRunning) native
/// ```
fn disable_player_controls(
    caller: Caller<'_, State>,
    self_ptr: Object,
    movement: bool,
    fighting: bool,
    cam_switch: bool,
    looking: bool,
    sneaking: bool,
    menu: bool,
    activate: bool,
    journal_tabs: bool,
    vats: bool,
    favorites: bool,
    running: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableActivate`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_activate(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableVATS`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_vats(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableCamSwitch`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_cam_switch(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableFastTravel`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_fast_travel(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableFavorites`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_favorites(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableFighting`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_fighting(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableJournal`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_journal(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableJumping`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_jumping(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableLooking`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_looking(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableMenu`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_menu(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableMovement`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_movement(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableRunning`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_running(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableSprinting`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_sprinting(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableZKey`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_z_key(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnablePlayerControls`
///
/// ```text
/// Function EnablePlayerControls(bool abMovement, bool abFighting, bool abCamSwitch, bool abLooking, bool abSneaking, bool abMenu, bool abActivate, bool abJournalTabs, bool abVATS, bool abFavorites, bool abRunning) native
/// ```
fn enable_player_controls(
    caller: Caller<'_, State>,
    self_ptr: Object,
    movement: bool,
    fighting: bool,
    cam_switch: bool,
    looking: bool,
    sneaking: bool,
    menu: bool,
    activate: bool,
    journal_tabs: bool,
    vats: bool,
    favorites: bool,
    running: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `EnableSneaking`
///
/// ```text
/// Function(bool abEnable) native
/// ```
fn enable_sneaking(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_enable: bool,
) -> Result<(), Exception> {
    todo!()
}

/// `IsActivateEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_activate_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsVATSEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_vats_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsCamSwitchEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_cam_switch_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsFastTravelEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_fast_travel_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsFavoritesEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_favorites_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsFightingEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_fighting_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsJournalEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_journal_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsJumpingEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_jumping_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsLookingEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_looking_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsMenuEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_menu_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsMovementEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_movement_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsRunningEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_running_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsSprintingEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_sprinting_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsZKeyEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_z_key_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsSneakingEnabled`
///
/// ```text
/// bool Function() native
/// ```
fn is_sneaking_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function() native
/// ```
fn reset(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}
