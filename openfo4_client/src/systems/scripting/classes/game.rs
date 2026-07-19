use openfo4_core::ecs::Entity;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::components::{ActorProperties, Player};
use crate::state::hardcoded::cumulative_xp_for_level;
use crate::systems::controller::InputControllerMask;
use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Game", "AddAchievement", add_achievement);
    engine.register_if_exists("Game", "AddPerkPoints", add_perk_points);
    engine.register_if_exists("Game", "AdvanceSkill", advance_skill);
    engine.register_if_exists("Game", "ClearPrison", clear_prison);
    engine.register_if_exists("Game", "ClearTempEffects", clear_temp_effects);
    engine.register_if_exists("Game", "EnablePipboyHDRMask", enable_pipboy_hdr_mask);
    engine.register_if_exists("Game", "Error", error);
    engine.register_if_exists("Game", "FadeOutGame", fade_out_game);
    engine.register_if_exists("Game", "FastTravel", fast_travel);
    engine.register_if_exists("Game", "FindClosestReferenceOfType", find_closest_reference_of_type);
    engine.register_if_exists("Game", "FindRandomReferenceOfType", find_random_reference_of_type);
    engine.register_if_exists("Game", "FindClosestReferenceOfAnyTypeInList", find_closest_reference_of_any_type_in_list);
    engine.register_if_exists("Game", "FindRandomReferenceOfAnyTypeInList", find_random_reference_of_any_type_in_list);
    engine.register_if_exists("Game", "FindClosestActor", find_closest_actor);
    engine.register_if_exists("Game", "FindRandomActor", find_random_actor);
    engine.register_if_exists("Game", "ForceDisableSSRGodraysDirLight", force_disable_ssr_godrays_dir_light);
    engine.register_if_exists("Game", "ForceThirdPerson", force_third_person);
    engine.register_if_exists("Game", "ForceFirstPerson", force_first_person);
    engine.register_if_exists("Game", "GetXPForLevel", get_xp_for_level);
    engine.register_if_exists("Game", "ShowFirstPersonGeometry", show_first_person_geometry);
    engine.register_if_exists("Game", "ShowAllMapMarkers", show_all_map_markers);
    engine.register_if_exists("Game", "GetAggressionAV", get_aggression_av);
    engine.register_if_exists("Game", "GetAgilityAV", get_agility_av);
    engine.register_if_exists("Game", "GetCharismaAV", get_charisma_av);
    engine.register_if_exists("Game", "GetConfidenceAV", get_confidence_av);
    engine.register_if_exists("Game", "GetDifficulty", get_difficulty);
    engine.register_if_exists("Game", "GetEnduranceAV", get_endurance_av);
    engine.register_if_exists("Game", "GetForm", get_form);
    engine.register_if_exists("Game", "GetFormFromFile", get_form_from_file);
    engine.register_if_exists("Game", "GetGameSettingFloat", get_game_setting_float);
    engine.register_if_exists("Game", "GetGameSettingInt", get_game_setting_int);
    engine.register_if_exists("Game", "GetGameSettingString", get_game_setting_string);
    engine.register_if_exists("Game", "GetHealthAV", get_health_av);
    engine.register_if_exists("Game", "GetIntelligenceAV", get_intelligence_av);
    engine.register_if_exists("Game", "GetLuckAV", get_luck_av);
    engine.register_if_exists("Game", "GetPerceptionAV", get_perception_av);
    engine.register_if_exists("Game", "GetPlayer", get_player);
    engine.register_if_exists("Game", "GetPlayerFollowers", get_player_followers);
    engine.register_if_exists("Game", "GetPlayerGrabbedRef", get_player_grabbed_ref);
    engine.register_if_exists("Game", "GetPlayerRadioFrequency", get_player_radio_frequency);
    engine.register_if_exists("Game", "GetPlayersLastRiddenHorse", get_players_last_ridden_horse);
    engine.register_if_exists("Game", "GetRealHoursPassed", get_real_hours_passed);
    engine.register_if_exists("Game", "GetSuspiciousAV", get_suspicious_av);
    engine.register_if_exists("Game", "GetStrengthAV", get_strength_av);
    engine.register_if_exists("Game", "IncrementSkill", increment_skill);
    engine.register_if_exists("Game", "IncrementStat", increment_stat);
    engine.register_if_exists("Game", "InitializeMarkerDistances", initialize_marker_distances);
    engine.register_if_exists("Game", "IsActivateControlsEnabled", is_activate_controls_enabled);
    engine.register_if_exists("Game", "IsVATSControlsEnabled", is_vats_controls_enabled);
    engine.register_if_exists("Game", "IsVATSPlaybackActive", is_vats_playback_active);
    engine.register_if_exists("Game", "IsCamSwitchControlsEnabled", is_cam_switch_controls_enabled);
    engine.register_if_exists("Game", "IsFastTravelControlsEnabled", is_fast_travel_controls_enabled);
    engine.register_if_exists("Game", "IsFastTravelEnabled", is_fast_travel_enabled);
    engine.register_if_exists("Game", "IsFavoritesControlsEnabled", is_favorites_controls_enabled);
    engine.register_if_exists("Game", "IsFightingControlsEnabled", is_fighting_controls_enabled);
    engine.register_if_exists("Game", "IsJournalControlsEnabled", is_journal_controls_enabled);
    engine.register_if_exists("Game", "IsJumpingControlsEnabled", is_jumping_controls_enabled);
    engine.register_if_exists("Game", "IsLookingControlsEnabled", is_looking_controls_enabled);
    engine.register_if_exists("Game", "IsMenuControlsEnabled", is_menu_controls_enabled);
    engine.register_if_exists("Game", "IsMovementControlsEnabled", is_movement_controls_enabled);
    engine.register_if_exists("Game", "IsPluginInstalled", is_plugin_installed);
    engine.register_if_exists("Game", "IsPlayerInRadioRange", is_player_in_radio_range);
    engine.register_if_exists("Game", "IsPlayerListening", is_player_listening);
    engine.register_if_exists("Game", "IsPlayerRadioOn", is_player_radio_on);
    engine.register_if_exists("Game", "IsSneakingControlsEnabled", is_sneaking_controls_enabled);
    engine.register_if_exists("Game", "PassTime", pass_time);
    engine.register_if_exists("Game", "PlayBink", play_bink);
    engine.register_if_exists("Game", "PrecacheCharGen", precache_char_gen);
    engine.register_if_exists("Game", "PrecacheCharGenClear", precache_char_gen_clear);
    engine.register_if_exists("Game", "QueryStat", query_stat);
    engine.register_if_exists("Game", "QuitToMainMenu", quit_to_main_menu);
    engine.register_if_exists("Game", "RequestAutoSave", request_auto_save);
    engine.register_if_exists("Game", "RequestModel", request_model);
    engine.register_if_exists("Game", "RequestSave", request_save);
    engine.register_if_exists("Game", "RewardPlayerXP", reward_player_xp);
    engine.register_if_exists("Game", "ServeTime", serve_time);
    engine.register_if_exists("Game", "SetCameraTarget", set_camera_target);
    engine.register_if_exists("Game", "SetCharGenHUDMode", set_char_gen_hud_mode);
    engine.register_if_exists("Game", "SetInsideMemoryHUDMode", set_inside_memory_hud_mode);
    engine.register_if_exists("Game", "ShowPerkVaultBoyOnHUD", show_perk_vault_boy_on_hud);
    engine.register_if_exists("Game", "SetInChargen", set_in_chargen);
    engine.register_if_exists("Game", "SetPlayerAIDriven", set_player_ai_driven);
    engine.register_if_exists("Game", "SetPlayerOnElevator", set_player_on_elevator);
    engine.register_if_exists("Game", "SetPlayerRadioFrequency", set_player_radio_frequency);
    engine.register_if_exists("Game", "SetPlayerReportCrime", set_player_report_crime);
    engine.register_if_exists("Game", "SetSittingRotation", set_sitting_rotation);
    engine.register_if_exists("Game", "ShakeCamera", shake_camera);
    engine.register_if_exists("Game", "ShakeController", shake_controller);
    engine.register_if_exists("Game", "ShowFatigueWarningOnHUD", show_fatigue_warning_on_hud);
    engine.register_if_exists("Game", "ShowRaceMenu", show_race_menu);
    engine.register_if_exists("Game", "ShowSPECIALMenu", show_special_menu);
    engine.register_if_exists("Game", "ShowTitleSequenceMenu", show_title_sequence_menu);
    engine.register_if_exists("Game", "HideTitleSequenceMenu", hide_title_sequence_menu);
    engine.register_if_exists("Game", "StartTitleSequence", start_title_sequence);
    engine.register_if_exists("Game", "ShowPipboyBootSequence", show_pipboy_boot_sequence);
    engine.register_if_exists("Game", "ShowPipboyPlugin", show_pipboy_plugin);
    engine.register_if_exists("Game", "ShowTrainingMenu", show_training_menu);
    engine.register_if_exists("Game", "TriggerScreenBlood", trigger_screen_blood);
    engine.register_if_exists("Game", "PlayEventCamera", play_event_camera);
    engine.register_if_exists("Game", "StartDialogueCameraOrCenterOnTarget", start_dialogue_camera_or_center_on_target);
    engine.register_if_exists("Game", "StopDialogueCamera", stop_dialogue_camera);
    engine.register_if_exists("Game", "TurnPlayerRadioOn", turn_player_radio_on);
    engine.register_if_exists("Game", "UsingGamepad", using_gamepad);
    engine.register_if_exists("Game", "Warning", warning);
}

/// `AddAchievement`
///
/// ```text
/// Function(int aiAchievementID) native global
/// ```
fn add_achievement(_caller: Caller<'_, State>, _achievement_id: i32) {}

/// `AddPerkPoints`
///
/// ```text
/// Function(int aiPerkPoints) native global
/// ```
async fn add_perk_points(caller: Caller<'_, State>, perk_points: i32) -> Result<(), Error> {
    // Cannot reduce perk points.
    let Ok(perk_points) = u32::try_from(perk_points) else {
        return Ok(());
    };

    with_player_props_mut(caller, |props| {
        props.perk_points = props.perk_points.saturating_add(perk_points);
    })
    .await
}

/// `AdvanceSkill`
///
/// ```text
/// Function(string asSkillName, float afMagnitude) native global
/// ```
fn advance_skill(
    caller: Caller<'_, State>,
    as_skill_name: Object,
    af_magnitude: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ClearPrison`
///
/// ```text
/// Function() native global
/// ```
fn clear_prison(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `ClearTempEffects`
///
/// ```text
/// Function() native global
/// ```
fn clear_temp_effects(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `EnablePipboyHDRMask`
///
/// ```text
/// Function(bool abEnable) native global
/// ```
fn enable_pipboy_hdr_mask(caller: Caller<'_, State>, ab_enable: bool) -> Result<(), Error> {
    todo!()
}

/// `Error`
///
/// ```text
/// Function(string asMessage) native global
/// ```
fn error(caller: Caller<'_, State>, msg: Object) -> Result<(), Error> {
    let msg = caller.string(msg);
    tracing::error!("script message: {}", msg);
    Ok(())
}

/// `FadeOutGame`
///
/// ```text
/// Function(bool abFadingOut, bool abBlackFade, float afSecsBeforeFade, float afFadeDuration, bool abStayFaded) native global
/// ```
fn fade_out_game(
    caller: Caller<'_, State>,
    ab_fading_out: bool,
    ab_black_fade: bool,
    af_secs_before_fade: f32,
    af_fade_duration: f32,
    ab_stay_faded: bool,
) -> Result<(), Error> {
    todo!()
}

/// `FastTravel`
///
/// ```text
/// Function(ObjectReference akDestination) native global
/// ```
fn fast_travel(caller: Caller<'_, State>, ak_destination: Object) -> Result<(), Error> {
    todo!()
}

/// `FindClosestReferenceOfType`
///
/// ```text
/// ObjectReference Function(Form arBaseObject, float afX, float afY, float afZ, float afRadius) native global
/// ```
fn find_closest_reference_of_type(
    caller: Caller<'_, State>,
    ar_base_object: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_radius: f32,
) -> Result<Object, Error> {
    todo!()
}

/// `FindRandomReferenceOfType`
///
/// ```text
/// ObjectReference Function(Form arBaseObject, float afX, float afY, float afZ, float afRadius) native global
/// ```
fn find_random_reference_of_type(
    caller: Caller<'_, State>,
    ar_base_object: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_radius: f32,
) -> Result<Object, Error> {
    todo!()
}

/// `FindClosestReferenceOfAnyTypeInList`
///
/// ```text
/// ObjectReference Function(FormList arBaseObjects, float afX, float afY, float afZ, float afRadius) native global
/// ```
fn find_closest_reference_of_any_type_in_list(
    caller: Caller<'_, State>,
    ar_base_objects: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_radius: f32,
) -> Result<Object, Error> {
    todo!()
}

/// `FindRandomReferenceOfAnyTypeInList`
///
/// ```text
/// ObjectReference Function(FormList arBaseObjects, float afX, float afY, float afZ, float afRadius) native global
/// ```
fn find_random_reference_of_any_type_in_list(
    caller: Caller<'_, State>,
    ar_base_objects: Object,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_radius: f32,
) -> Result<Object, Error> {
    todo!()
}

/// `FindClosestActor`
///
/// ```text
/// Actor Function(float afX, float afY, float afZ, float afRadius) native global
/// ```
fn find_closest_actor(
    caller: Caller<'_, State>,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_radius: f32,
) -> Result<Object, Error> {
    todo!()
}

/// `FindRandomActor`
///
/// ```text
/// Actor Function(float afX, float afY, float afZ, float afRadius) native global
/// ```
fn find_random_actor(
    caller: Caller<'_, State>,
    af_x: f32,
    af_y: f32,
    af_z: f32,
    af_radius: f32,
) -> Result<Object, Error> {
    todo!()
}

/// `ForceDisableSSRGodraysDirLight`
///
/// ```text
/// Function(bool abDisableSSR, bool abDisableGodrays, bool abDisableDirLight) native global
/// ```
fn force_disable_ssr_godrays_dir_light(
    caller: Caller<'_, State>,
    ab_disable_ssr: bool,
    ab_disable_godrays: bool,
    ab_disable_dir_light: bool,
) -> Result<(), Error> {
    todo!()
}

/// `ForceThirdPerson`
///
/// ```text
/// Function() native global
/// ```
fn force_third_person(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `ForceFirstPerson`
///
/// ```text
/// Function() native global
/// ```
fn force_first_person(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `GetXPForLevel`
///
/// ```text
/// int Function(int auiLevel) native global
/// ```
fn get_xp_for_level(_caller: Caller<'_, State>, level: i32) -> i32 {
    let level = u32::try_from(level).unwrap_or_default();
    let xp = cumulative_xp_for_level(level);
    xp.try_into().unwrap_or(i32::MAX)
}

/// `ShowFirstPersonGeometry`
///
/// ```text
/// Function(bool abShow) native global
/// ```
fn show_first_person_geometry(caller: Caller<'_, State>, ab_show: bool) -> Result<(), Error> {
    todo!()
}

/// `ShowAllMapMarkers`
///
/// ```text
/// Function() native global
/// ```
fn show_all_map_markers(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `GetAggressionAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_aggression_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.aggression.addr()
}

/// `GetAgilityAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_agility_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.agility.addr()
}

/// `GetCharismaAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_charisma_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.charisma.addr()
}

/// `GetConfidenceAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_confidence_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.confidence.addr()
}

/// `GetDifficulty`
///
/// ```text
/// int Function() native global
/// ```
fn get_difficulty(caller: Caller<'_, State>) -> i32 {
    caller.data().globals.game_rules.difficulty.to_i32()
}

/// `GetEnduranceAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_endurance_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.endurance.addr()
}

/// `GetForm`
///
/// ```text
/// Form Function(int aiFormID) native global
/// ```
fn get_form(caller: Caller<'_, State>, ai_form_id: i32) -> Result<Object, Error> {
    todo!()
}

/// `GetFormFromFile`
///
/// ```text
/// Form Function(int aiFormID, string asFilename) native global
/// ```
fn get_form_from_file(
    caller: Caller<'_, State>,
    ai_form_id: i32,
    as_filename: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `GetGameSettingFloat`
///
/// ```text
/// float Function(string asGameSetting) native global
/// ```
fn get_game_setting_float(
    caller: Caller<'_, State>,
    as_game_setting: Object,
) -> Result<f32, Error> {
    todo!()
}

/// `GetGameSettingInt`
///
/// ```text
/// int Function(string asGameSetting) native global
/// ```
fn get_game_setting_int(caller: Caller<'_, State>, as_game_setting: Object) -> Result<i32, Error> {
    todo!()
}

/// `GetGameSettingString`
///
/// ```text
/// string Function(string asGameSetting) native global
/// ```
fn get_game_setting_string(
    caller: Caller<'_, State>,
    as_game_setting: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `GetHealthAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_health_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.health.addr()
}

/// `GetIntelligenceAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_intelligence_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.intelligence.addr()
}

/// `GetLuckAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_luck_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.luck.addr()
}

/// `GetPerceptionAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_perception_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.perception.addr()
}

/// `GetPlayer`
///
/// ```text
/// Actor Function() native global
/// ```
async fn get_player(mut caller: Caller<'_, State>) -> Result<Object, Error> {
    let entity = {
        let world = caller.data().world().await;

        let Some((entity, _)) = world.query::<(Entity, &Player)>().iter().next() else {
            return Err(Error::msg("Game.GetPlayer found no player"));
        };

        entity
    };

    // We can assume here that the player always has a script attached here,
    // so we never need to suspend.
    let scripts = caller.data_mut().get_or_create_entity(entity).await?;
    let actor = scripts.get("Actor").unwrap();
    Ok(actor.get().object.addr())
}

/// `GetPlayerFollowers`
///
/// ```text
/// Actor[] Function() native global
/// ```
fn get_player_followers(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetPlayerGrabbedRef`
///
/// ```text
/// ObjectReference Function() native global
/// ```
fn get_player_grabbed_ref(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetPlayerRadioFrequency`
///
/// ```text
/// float Function() native global
/// ```
fn get_player_radio_frequency(caller: Caller<'_, State>) -> Result<f32, Error> {
    todo!()
}

/// `GetPlayersLastRiddenHorse`
///
/// ```text
/// Actor Function() native global
/// ```
fn get_players_last_ridden_horse(caller: Caller<'_, State>) -> Result<Object, Error> {
    todo!()
}

/// `GetRealHoursPassed`
///
/// ```text
/// float Function() native global
/// ```
fn get_real_hours_passed(caller: Caller<'_, State>) -> Result<f32, Error> {
    todo!()
}

/// `GetSuspiciousAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_suspicious_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.suspicious.addr()
}

/// `GetStrengthAV`
///
/// ```text
/// ActorValue Function() native global
/// ```
fn get_strength_av(caller: Caller<'_, State>) -> Object {
    caller.data().core_objects.strength.addr()
}

/// `IncrementSkill`
///
/// ```text
/// Function(ActorValue akActorValue, int aiCount) native global
/// ```
fn increment_skill(
    caller: Caller<'_, State>,
    ak_actor_value: Object,
    ai_count: i32,
) -> Result<(), Error> {
    todo!()
}

/// `IncrementStat`
///
/// ```text
/// Function(string asStatName, int aiModAmount) native global
/// ```
fn increment_stat(
    caller: Caller<'_, State>,
    as_stat_name: Object,
    ai_mod_amount: i32,
) -> Result<(), Error> {
    todo!()
}

/// `InitializeMarkerDistances`
///
/// ```text
/// Function() native global
/// ```
fn initialize_marker_distances(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `IsActivateControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_activate_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.activate()).await
}

/// `IsVATSControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_vats_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.vats()).await
}

/// `IsVATSPlaybackActive`
///
/// ```text
/// bool Function() native global
/// ```
fn is_vats_playback_active(caller: Caller<'_, State>) -> Result<bool, Error> {
    todo!()
}

/// `IsCamSwitchControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_cam_switch_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.camera_switch()).await
}

/// `IsFastTravelControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_fast_travel_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.fast_travel()).await
}

/// `IsFastTravelEnabled`
///
/// ```text
/// bool Function() native global
/// ```
fn is_fast_travel_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    todo!()
}

/// `IsFavoritesControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_favorites_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.favorites()).await
}

/// `IsFightingControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_fighting_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.fighting()).await
}

/// `IsJournalControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_journal_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.journal()).await
}

/// `IsJumpingControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_jumping_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.jumping()).await
}

/// `IsLookingControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_looking_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.looking()).await
}

/// `IsMenuControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_menu_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.menu()).await
}

/// `IsMovementControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_movement_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.movement()).await
}

/// `IsPluginInstalled`
///
/// ```text
/// bool Function(string asName) native global
/// ```
fn is_plugin_installed(caller: Caller<'_, State>, as_name: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsPlayerInRadioRange`
///
/// ```text
/// bool Function(float afFrequency) native global
/// ```
fn is_player_in_radio_range(caller: Caller<'_, State>, af_frequency: f32) -> Result<bool, Error> {
    todo!()
}

/// `IsPlayerListening`
///
/// ```text
/// bool Function(float afFrequency) native global
/// ```
fn is_player_listening(caller: Caller<'_, State>, af_frequency: f32) -> Result<bool, Error> {
    todo!()
}

/// `IsPlayerRadioOn`
///
/// ```text
/// bool Function() native global
/// ```
fn is_player_radio_on(caller: Caller<'_, State>) -> Result<bool, Error> {
    todo!()
}

/// `IsSneakingControlsEnabled`
///
/// ```text
/// bool Function() native global
/// ```
async fn is_sneaking_controls_enabled(caller: Caller<'_, State>) -> Result<bool, Error> {
    with_player_control_mask(caller, |mask| mask.sneaking()).await
}

/// `PassTime`
///
/// ```text
/// Function(int aiHours) native global
/// ```
fn pass_time(caller: Caller<'_, State>, ai_hours: i32) -> Result<(), Error> {
    todo!()
}

/// `PlayBink`
///
/// ```text
/// Function PlayBink(string asFileName, bool abInterruptible = false, bool abMuteAudio = true, bool abMuteMusic = true, bool abLetterbox = true, bool abIsNewGameBink = false) native global
/// ```
fn play_bink(
    caller: Caller<'_, State>,
    file_name: Object,
    interruptible: bool,
    mute_audio: bool,
    mute_music: bool,
    letterbox: bool,
    is_new_game_bink: bool,
) -> Result<(), Error> {
    todo!()
}

/// `PrecacheCharGen`
///
/// ```text
/// Function() native global
/// ```
fn precache_char_gen(_caller: Caller<'_, State>) {}

/// `PrecacheCharGenClear`
///
/// ```text
/// Function() native global
/// ```
fn precache_char_gen_clear(_caller: Caller<'_, State>) {}

/// `QueryStat`
///
/// ```text
/// int Function(string asStat) native global
/// ```
fn query_stat(caller: Caller<'_, State>, as_stat: Object) -> Result<i32, Error> {
    todo!()
}

/// `QuitToMainMenu`
///
/// ```text
/// Function() native global
/// ```
fn quit_to_main_menu(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `RequestAutoSave`
///
/// ```text
/// Function() native global
/// ```
fn request_auto_save(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `RequestModel`
///
/// ```text
/// Function(string asModelName) native global
/// ```
fn request_model(_caller: Caller<'_, State>, _model_name: Object) {}

/// `RequestSave`
///
/// ```text
/// Function() native global
/// ```
fn request_save(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `RewardPlayerXP`
///
/// ```text
/// Function(int auiXPAmount, bool abDirect) native global
/// ```
fn reward_player_xp(
    caller: Caller<'_, State>,
    aui_xp_amount: i32,
    ab_direct: bool,
) -> Result<(), Error> {
    todo!()
}

/// `ServeTime`
///
/// ```text
/// Function() native global
/// ```
fn serve_time(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `SetCameraTarget`
///
/// ```text
/// Function(Actor arTarget) native global
/// ```
fn set_camera_target(caller: Caller<'_, State>, ar_target: Object) -> Result<(), Error> {
    todo!()
}

/// `SetCharGenHUDMode`
///
/// ```text
/// Function(int aiCGHUDMode) native global
/// ```
fn set_char_gen_hud_mode(caller: Caller<'_, State>, ai_cghud_mode: i32) -> Result<(), Error> {
    todo!()
}

/// `SetInsideMemoryHUDMode`
///
/// ```text
/// Function(bool aInsideMemory) native global
/// ```
fn set_inside_memory_hud_mode(
    caller: Caller<'_, State>,
    a_inside_memory: bool,
) -> Result<(), Error> {
    todo!()
}

/// `ShowPerkVaultBoyOnHUD`
///
/// ```text
/// Function(string aVaultBoySwf, Sound aSoundDescriptor) native global
/// ```
fn show_perk_vault_boy_on_hud(
    caller: Caller<'_, State>,
    a_vault_boy_swf: Object,
    a_sound_descriptor: Object,
) -> Result<(), Error> {
    todo!()
}

/// `SetInChargen`
///
/// ```text
/// Function(bool abDisableSaving, bool abDisableWaiting, bool abShowControlsDisabledMessage) native global
/// ```
fn set_in_chargen(
    caller: Caller<'_, State>,
    ab_disable_saving: bool,
    ab_disable_waiting: bool,
    ab_show_controls_disabled_message: bool,
) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerAIDriven`
///
/// ```text
/// Function(bool abAIDriven) native global
/// ```
fn set_player_ai_driven(caller: Caller<'_, State>, ab_ai_driven: bool) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerOnElevator`
///
/// ```text
/// Function(bool abOnElevator=) native global
/// ```
fn set_player_on_elevator(caller: Caller<'_, State>, ab_on_elevator: bool) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerRadioFrequency`
///
/// ```text
/// Function(float afFrequency) native global
/// ```
fn set_player_radio_frequency(caller: Caller<'_, State>, af_frequency: f32) -> Result<(), Error> {
    todo!()
}

/// `SetPlayerReportCrime`
///
/// ```text
/// Function(bool abReportCrime) native global
/// ```
fn set_player_report_crime(caller: Caller<'_, State>, ab_report_crime: bool) -> Result<(), Error> {
    todo!()
}

/// `SetSittingRotation`
///
/// ```text
/// Function(float afValue) native global
/// ```
fn set_sitting_rotation(caller: Caller<'_, State>, af_value: f32) -> Result<(), Error> {
    todo!()
}

/// `ShakeCamera`
///
/// ```text
/// Function(ObjectReference akSource, float afStrength, float afDuration) native global
/// ```
fn shake_camera(
    caller: Caller<'_, State>,
    ak_source: Object,
    af_strength: f32,
    af_duration: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ShakeController`
///
/// ```text
/// Function(float afSmallMotorStrength, float afBigMotorStreangth, float afDuration) native global
/// ```
fn shake_controller(
    caller: Caller<'_, State>,
    af_small_motor_strength: f32,
    af_big_motor_streangth: f32,
    af_duration: f32,
) -> Result<(), Error> {
    todo!()
}

/// `ShowFatigueWarningOnHUD`
///
/// ```text
/// Function() native global
/// ```
fn show_fatigue_warning_on_hud(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `ShowRaceMenu`
///
/// ```text
/// Function(ObjectReference akMenuTarget, int uiMode, ObjectReference akMenuSpouseFemale, ObjectReference akMenuSpouseMale, ObjectReference akVendor) native global
/// ```
fn show_race_menu(
    caller: Caller<'_, State>,
    ak_menu_target: Object,
    ui_mode: i32,
    ak_menu_spouse_female: Object,
    ak_menu_spouse_male: Object,
    ak_vendor: Object,
) -> Result<(), Error> {
    todo!()
}

/// `ShowSPECIALMenu`
///
/// ```text
/// Function() native global
/// ```
fn show_special_menu(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `ShowTitleSequenceMenu`
///
/// ```text
/// Function() native global
/// ```
fn show_title_sequence_menu(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `HideTitleSequenceMenu`
///
/// ```text
/// Function() native global
/// ```
fn hide_title_sequence_menu(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `StartTitleSequence`
///
/// ```text
/// Function(string asSequenceName) native global
/// ```
fn start_title_sequence(caller: Caller<'_, State>, as_sequence_name: Object) -> Result<(), Error> {
    todo!()
}

/// `ShowPipboyBootSequence`
///
/// ```text
/// Function(string asAnimationName) native global
/// ```
fn show_pipboy_boot_sequence(
    caller: Caller<'_, State>,
    as_animation_name: Object,
) -> Result<(), Error> {
    todo!()
}

/// `ShowPipboyPlugin`
///
/// ```text
/// Function() native global
/// ```
fn show_pipboy_plugin(caller: Caller<'_, State>) -> Result<(), Error> {
    todo!()
}

/// `ShowTrainingMenu`
///
/// ```text
/// Function(Actor aTrainer) native global
/// ```
fn show_training_menu(caller: Caller<'_, State>, a_trainer: Object) -> Result<(), Error> {
    todo!()
}

/// `TriggerScreenBlood`
///
/// ```text
/// Function(int aiValue) native global
/// ```
fn trigger_screen_blood(caller: Caller<'_, State>, ai_value: i32) -> Result<(), Error> {
    todo!()
}

/// `PlayEventCamera`
///
/// ```text
/// Function(CameraShot akCamera, ObjectReference akRef) native global
/// ```
fn play_event_camera(
    caller: Caller<'_, State>,
    ak_camera: Object,
    ak_ref: Object,
) -> Result<(), Error> {
    todo!()
}

/// `StartDialogueCameraOrCenterOnTarget`
///
/// ```text
/// Function(ObjectReference akCameraTarget) native global
/// ```
fn start_dialogue_camera_or_center_on_target(
    caller: Caller<'_, State>,
    ak_camera_target: Object,
) -> Result<(), Error> {
    todo!()
}

/// `StopDialogueCamera`
///
/// ```text
/// Function(bool abConsiderResume, bool abSwitchingTo1stP) native global
/// ```
fn stop_dialogue_camera(
    caller: Caller<'_, State>,
    ab_consider_resume: bool,
    ab_switching_to1st_p: bool,
) -> Result<(), Error> {
    todo!()
}

/// `TurnPlayerRadioOn`
///
/// ```text
/// Function(bool abRadioOn) native global
/// ```
fn turn_player_radio_on(caller: Caller<'_, State>, ab_radio_on: bool) -> Result<(), Error> {
    todo!()
}

/// `UsingGamepad`
///
/// ```text
/// bool Function() native global
/// ```
fn using_gamepad(caller: Caller<'_, State>) -> Result<bool, Error> {
    todo!()
}

/// `Warning`
///
/// ```text
/// Function(string asMessage) native global
/// ```
fn warning(caller: Caller<'_, State>, msg: Object) -> Result<(), Error> {
    let msg = caller.string(msg);
    tracing::warn!("script message: {}", msg);
    Ok(())
}

async fn with_player_control_mask<T, F>(caller: Caller<'_, State>, f: F) -> Result<T, Error>
where
    F: FnOnce(&InputControllerMask) -> T,
{
    let world = caller.data().world().await;
    let Some((entity, _)) = world.query::<(Entity, &Player)>().iter().next() else {
        return Err(Error::msg("found no player"));
    };

    let Ok(mask) = world.get::<&InputControllerMask>(entity) else {
        return Err(Error::msg("player has no controller mask"));
    };

    Ok(f(&*mask))
}

async fn with_player_props_mut<T, F>(caller: Caller<'_, State>, f: F) -> Result<T, Error>
where
    F: FnOnce(&mut ActorProperties) -> T,
{
    let world = caller.data().world().await;

    let mut query = world.query::<(&Player, &mut ActorProperties)>();
    let Some((_, mut props)) = query.iter().next() else {
        return Err(Error::msg("found no player"));
    };

    Ok(f(&mut *props))
}
