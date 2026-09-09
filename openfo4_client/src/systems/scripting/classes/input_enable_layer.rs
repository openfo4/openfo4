use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::components::Player;
use crate::systems::controller::{InputControllerLayer, InputControllerMask};
use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::object_data::ObjectData;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("InputEnableLayer", "Create", create);
    engine.register_if_exists("InputEnableLayer", "Delete", delete);
    engine.register_if_exists("InputEnableLayer", "DisablePlayerControls", disable_player_controls);
    engine.register_if_exists("InputEnableLayer", "EnableActivate", enable_activate);
    engine.register_if_exists("InputEnableLayer", "EnableVATS", enable_vats);
    engine.register_if_exists("InputEnableLayer", "EnableCamSwitch", enable_cam_switch);
    engine.register_if_exists("InputEnableLayer", "EnableFastTravel", enable_fast_travel);
    engine.register_if_exists("InputEnableLayer", "EnableFavorites", enable_favorites);
    engine.register_if_exists("InputEnableLayer", "EnableFighting", enable_fighting);
    engine.register_if_exists("InputEnableLayer", "EnableJournal", enable_journal);
    engine.register_if_exists("InputEnableLayer", "EnableJumping", enable_jumping);
    engine.register_if_exists("InputEnableLayer", "EnableLooking", enable_looking);
    engine.register_if_exists("InputEnableLayer", "EnableMenu", enable_menu);
    engine.register_if_exists("InputEnableLayer", "EnableMovement", enable_movement);
    engine.register_if_exists("InputEnableLayer", "EnableRunning", enable_running);
    engine.register_if_exists("InputEnableLayer", "EnableSprinting", enable_sprinting);
    engine.register_if_exists("InputEnableLayer", "EnableZKey", enable_z_key);
    engine.register_if_exists("InputEnableLayer", "EnablePlayerControls", enable_player_controls);
    engine.register_if_exists("InputEnableLayer", "EnableSneaking", enable_sneaking);
    engine.register_if_exists("InputEnableLayer", "IsActivateEnabled", is_activate_enabled);
    engine.register_if_exists("InputEnableLayer", "IsVATSEnabled", is_vats_enabled);
    engine.register_if_exists("InputEnableLayer", "IsCamSwitchEnabled", is_cam_switch_enabled);
    engine.register_if_exists("InputEnableLayer", "IsFastTravelEnabled", is_fast_travel_enabled);
    engine.register_if_exists("InputEnableLayer", "IsFavoritesEnabled", is_favorites_enabled);
    engine.register_if_exists("InputEnableLayer", "IsFightingEnabled", is_fighting_enabled);
    engine.register_if_exists("InputEnableLayer", "IsJournalEnabled", is_journal_enabled);
    engine.register_if_exists("InputEnableLayer", "IsJumpingEnabled", is_jumping_enabled);
    engine.register_if_exists("InputEnableLayer", "IsLookingEnabled", is_looking_enabled);
    engine.register_if_exists("InputEnableLayer", "IsMenuEnabled", is_menu_enabled);
    engine.register_if_exists("InputEnableLayer", "IsMovementEnabled", is_movement_enabled);
    engine.register_if_exists("InputEnableLayer", "IsRunningEnabled", is_running_enabled);
    engine.register_if_exists("InputEnableLayer", "IsSprintingEnabled", is_sprinting_enabled);
    engine.register_if_exists("InputEnableLayer", "IsZKeyEnabled", is_z_key_enabled);
    engine.register_if_exists("InputEnableLayer", "IsSneakingEnabled", is_sneaking_enabled);
    engine.register_if_exists("InputEnableLayer", "Reset", reset);
}

/// `Create`
///
/// ```text
/// InputEnableLayer Function() native global
/// ```
fn create(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `Delete`
///
/// ```text
/// Function() native
/// ```
async fn delete(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    // Reenable any controls that are still disabled on this layer.
    // The GC will cleanup the object and when that happens the
    // layer will be deleted.
    reset(caller, self_ptr).await
}

/// `DisablePlayerControls`
///
/// ```text
/// Function DisablePlayerControls(bool abMovement, bool abFighting, bool abCamSwitch, bool abLooking, bool abSneaking, bool abMenu, bool abActivate, bool abJournalTabs, bool abVATS, bool abFavorites, bool abRunning) native
/// ```
async fn disable_player_controls(
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
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.movement ^= movement;
        layer.fighting ^= fighting;
        layer.camera_switch ^= cam_switch;
        layer.looking ^= looking;
        layer.sneaking ^= sneaking;
        layer.menu ^= menu;
        layer.activate ^= activate;
        layer.journal ^= journal_tabs;
        layer.vats ^= vats;
        layer.favorites ^= favorites;
        layer.running ^= running;
    })
    .await
}

/// `EnableActivate`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_activate(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.activate = enable;
    })
    .await
}

/// `EnableVATS`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_vats(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.vats = enable;
    })
    .await
}

/// `EnableCamSwitch`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_cam_switch(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.camera_switch = enable;
    })
    .await
}

/// `EnableFastTravel`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_fast_travel(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.fast_travel = enable;
    })
    .await
}

/// `EnableFavorites`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_favorites(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.favorites = enable;
    })
    .await
}

/// `EnableFighting`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_fighting(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.fighting = enable;
    })
    .await
}

/// `EnableJournal`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_journal(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.journal = enable;
    })
    .await
}

/// `EnableJumping`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_jumping(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.jumping = enable;
    })
    .await
}

/// `EnableLooking`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_looking(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.looking = enable;
    })
    .await
}

/// `EnableMenu`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_menu(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.menu = enable;
    })
    .await
}

/// `EnableMovement`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_movement(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.movement = enable;
    })
    .await
}

/// `EnableRunning`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_running(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.running = enable;
    })
    .await
}

/// `EnableSprinting`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_sprinting(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.sprinting = enable;
    })
    .await
}

/// `EnableZKey`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_z_key(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.z_key = enable;
    })
    .await
}

/// `EnablePlayerControls`
///
/// ```text
/// Function EnablePlayerControls(bool abMovement, bool abFighting, bool abCamSwitch, bool abLooking, bool abSneaking, bool abMenu, bool abActivate, bool abJournalTabs, bool abVATS, bool abFavorites, bool abRunning) native
/// ```
async fn enable_player_controls(
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
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.movement |= movement;
        layer.fighting |= fighting;
        layer.camera_switch |= cam_switch;
        layer.looking |= looking;
        layer.sneaking |= sneaking;
        layer.menu |= menu;
        layer.activate |= activate;
        layer.journal |= journal_tabs;
        layer.vats |= vats;
        layer.favorites |= favorites;
        layer.running |= running;
    })
    .await
}

/// `EnableSneaking`
///
/// ```text
/// Function(bool abEnable) native
/// ```
async fn enable_sneaking(
    caller: Caller<'_, State>,
    self_ptr: Object,
    enable: bool,
) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        layer.sneaking = enable;
    })
    .await
}

/// `IsActivateEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_activate_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.activate).await
}

/// `IsVATSEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_vats_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.vats).await
}

/// `IsCamSwitchEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_cam_switch_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.camera_switch).await
}

/// `IsFastTravelEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_fast_travel_enabled(
    caller: Caller<'_, State>,
    self_ptr: Object,
) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.fast_travel).await
}

/// `IsFavoritesEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_favorites_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.favorites).await
}

/// `IsFightingEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_fighting_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.fighting).await
}

/// `IsJournalEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_journal_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.journal).await
}

/// `IsJumpingEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_jumping_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.jumping).await
}

/// `IsLookingEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_looking_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.looking).await
}

/// `IsMenuEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_menu_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.menu).await
}

/// `IsMovementEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_movement_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.movement).await
}

/// `IsRunningEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_running_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.running).await
}

/// `IsSprintingEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_sprinting_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.sprinting).await
}

/// `IsZKeyEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_z_key_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.z_key).await
}

/// `IsSneakingEnabled`
///
/// ```text
/// bool Function() native
/// ```
async fn is_sneaking_enabled(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Error> {
    with_layer(caller, self_ptr, |layer| layer.sneaking).await
}

/// `Reset`
///
/// ```text
/// Function() native
/// ```
async fn reset(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Error> {
    with_layer(caller, self_ptr, |layer| {
        *layer = InputControllerLayer::default();
    })
    .await
}

async fn with_layer<T, F>(caller: Caller<'_, State>, object: Object, f: F) -> Result<T, Error>
where
    F: FnOnce(&mut InputControllerLayer) -> T,
{
    let script = caller.data().objects.get_by_addr(object).unwrap();
    let data = script.data();

    let ObjectData::InputEnableLayer { layer } = data else {
        unreachable!();
    };

    let world = caller.data().world().await;

    let mut q = world.query::<(&Player, &mut InputControllerMask)>();
    let Some((_, mut mask)) = q.iter().next() else {
        // Player is not loaded yet.
        // return Err(Error::Error);
        todo!();
    };

    let layer = mask.layers.get_mut(*layer).unwrap();
    Ok(f(layer))
}
