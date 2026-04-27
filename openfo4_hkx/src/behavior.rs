use bitflags::bitflags;
use glam::{Quat, Vec4};
use openfo4_hkx_derive::{HkEnum, HkStruct, signature};

use crate::physics::HkLocalFrame;
use crate::skeleton::{Bindable, BoneWeightArray};
use crate::{Array, OpaquePtr, Ptr, ReferencedObject, impl_decode_for_bitflags};

#[derive(Clone, Debug, HkStruct)]
#[signature(0x08b4251f)]
pub struct Node {
    // Offset: 0
    pub bindable: Bindable,
    // Offset: 48
    pub user_data: u64,
    // Offset: 56
    pub name: String,
    // Offset: 64
    pub id: u16,
    // Offset: 66
    pub clone_state: i8,
    // Offset: 67
    pub type_: i8,
    // Offset: 72
    pub node_info: Option<OpaquePtr>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xad634f7e)]
pub struct Generator {
    // Offset: 0
    pub node: Node,
    // Offset: 80
    pub partition_info: GeneratorPartitionInfo,
    // Offset: 120
    pub sync_info: Option<OpaquePtr>,
    // Offset: 128
    _pad0: [u8; 4],
    _pad1: [u8; 4],
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc5cf46e6)]
pub struct GeneratorPartitionInfo {
    // Offset: 0
    pub bone_mask: [u32; 8],
    // Offset: 32
    pub partition_mask: [u32; 1],
    // Offset: 36
    pub num_bones: i16,
    // OffseT: 38
    pub num_max_partitions: i16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xfdedb83b)]
pub struct BehaviorGraph {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub variable_mode: i8,
    // Offset: 144
    pub unique_id_pool: Array<()>,
    // Offset: 160
    pub id_to_state_machine_template_map: Option<OpaquePtr>,
    // Offset: 168
    pub mirrored_external_map_id: Array<()>,
    // Offset: 184
    pub pseudo_random_generator: Option<OpaquePtr>,
    // Offset: 192
    pub root_generator: Option<Ptr<Generator>>,
    // Offset: 200
    pub data: Ptr<BehaviorGraphData>,
    // Offset: 208
    pub template: Option<OpaquePtr>,
    // Offset: 216
    pub root_generator_clone: Option<OpaquePtr>,
    // Offset: 224
    pub active_nodes: Option<OpaquePtr>,
    // Offset: 232
    pub global_transition_data: Option<OpaquePtr>,
    // Offset: 240
    pub event_id_map: Option<OpaquePtr>,
    // Offset: 248
    pub attribute_id_map: Option<OpaquePtr>,
    // Offset: 256
    pub variable_id_map: Option<OpaquePtr>,
    // Offset: 264
    pub character_property_id_map: Option<OpaquePtr>,
    // Offset: 272
    pub variable_value_set: Option<OpaquePtr>,
    // Offset: 280
    pub node_template_to_clone_map: Option<OpaquePtr>,
    // Offset: 288
    pub state_listener_template_to_clone_map: Option<OpaquePtr>,
    // Offset: 296
    pub recently_created_clones: Array<OpaquePtr>,
    // Offset: 312
    pub node_partition_info: Option<OpaquePtr>,
    // Offset: 320
    pub num_intermediate_outputs: i32,
    // Offset: 328
    pub intermediate_output_sizes: Array<()>,
    // Offset: 344
    pub jobs: Array<OpaquePtr>,
    // Offset: 360
    pub all_partition_memory: Array<OpaquePtr>,
    // Offset: 376
    pub internal_to_root_variable_id_map: Array<()>,
    // Offset: 392
    pub internal_to_character_character_property_id_map: Array<()>,
    // Offset: 408
    pub internal_to_root_attribute_id_map: Array<()>,
    // Offset: 424
    pub next_unique_id: u16,
    // Offset: 426
    pub is_active: bool,
    // Offset: 427
    pub is_linked: bool,
    // Offset: 428
    pub update_active_nodes: bool,
    // Offset: 429
    pub update_active_nodes_for_enable: bool,
    // Offset: 430
    pub check_node_validity: bool,
    // Offset: 431
    pub state_or_transition_changed: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x907a8222)]
pub struct BehaviorGraphData {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub attribute_defaults: Array<f32>,
    // Offset: 32
    pub variable_infos: Array<VariableInfo>,
    // Offset: 48
    pub character_property_infos: Array<VariableInfo>,
    // Offset: 64
    pub event_infos: Array<EventInfo>,
    // Offset: 80
    pub variable_bounds: Array<VariableBounds>,
    // Offset: 96
    pub variable_initial_values: Option<Ptr<VariableValueSet>>,
    // Offset: 104
    pub string_data: Ptr<BehaviorGraphStringData>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xa5ae6be2)]
pub struct VariableInfo {
    // Offset: 0
    pub role: RoleAttribute,
    // Offset: 4
    pub type_: VariableType,
    _pad0: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, HkEnum)]
#[size(i8)]
pub enum VariableType {
    Invalid = -1,
    Bool = 0,
    Int8 = 1,
    Int16 = 2,
    Int32 = 3,
    Real = 4,
    Pointer = 5,
    Vector3 = 6,
    Vector4 = 7,
    Quaternion = 8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xfecef669)]
pub struct RoleAttribute {
    // Offset: 0
    pub role: i16,
    // Offset: 2
    pub flags: i16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x5874eed4)]
pub struct EventInfo {
    // Offset: 0
    pub flags: u32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x9b766364)]
pub struct VariableBounds {
    // Offset: 0
    pub min: VariableValue,
    // Offset: 4
    pub max: VariableValue,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0b99bd6a)]
pub struct VariableValue {
    // Offset: 0
    pub value: i32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xeb5f7e25)]
pub struct VariableValueSet {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub word_variable_values: Array<VariableValue>,
    // Offset: 32
    pub quad_variable_values: Array<Vec4>,
    // Offset: 48
    pub variant_variable_values: Array<Option<OpaquePtr>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x1bd27f38)]
pub struct BehaviorGraphStringData {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub event_names: Array<String>,
    // Offset: 32
    pub attribute_names: Array<String>,
    // Offset: 48
    pub variable_names: Array<String>,
    // Offset: 64
    pub character_property_names: Array<String>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xa5896bcf)]
pub struct StateMachine {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub event_to_send_when_state_or_transition_changes: Event,
    // Offset: 160
    pub start_state_id_selector: Option<Ptr<CustomIdSelector>>,
    // Offset: 168
    pub start_state_id: i32,
    // Offset: 172
    pub return_to_previous_state_event_id: i32,
    // Offset: 176
    pub random_transition_event_id: i32,
    // Offset: 180
    pub transition_to_next_higher_state_event_id: i32,
    // Offset: 184
    pub transition_to_next_lower_state_event_id: i32,
    // Offset: 188
    pub sync_variable_index: i32,
    // Offset: 192
    pub current_state_id: i32,
    // Offset: 196
    pub wrap_around_state_id: bool,
    // Offset: 197
    pub max_simultaneous_transitions: i8,
    // Offset: 198
    pub start_state_mode: StartStateMode,
    // Offset: 199
    pub self_transition_mode: StateMachineSelfTransitionMode,
    // Offset: 200
    pub is_active: bool,
    // Offset: 208
    pub states: Array<Ptr<StateMachineStateInfo>>,
    // Offset: 224
    pub wildcard_transitions: Option<Ptr<StateMachineTransitionInfoArray>>,
    // Offset: 232
    pub state_id_to_index_map: Option<OpaquePtr>,
    // Offset: 240
    pub active_transitions: Array<()>,
    // Offset: 256
    pub transition_flags: Array<()>,
    // Offset: 272
    pub wildcard_transition_flags: Array<()>,
    // Offset: 288
    pub delayed_transitions: Array<()>,
    // Offset: 304
    pub time_in_state: f32,
    // Offset: 308
    pub last_local_time: f32,
    // Offset: 312
    pub previous_state_id: i32,
    // Offset: 316
    pub next_start_state_index_override: i32,
    // Offset: 320
    pub state_of_transition_changed: bool,
    // Offset: 321
    pub echo_next_update: bool,
    // Offset: 322
    pub has_eventless_wildcard_transitions: bool,
    // Offset: 324
    pub s_current_state_index_and_entered: u16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x39d76713)]
pub struct StateMachineStateInfo {
    // Offset: 0
    pub bindable: Bindable,
    // Offset: 48
    pub listeners: Array<StateListener>,
    // Offset: 64
    pub enter_notify_events: Option<Ptr<StateMachineEventPropertyArray>>,
    // Offset: 72
    pub exit_notify_events: Option<Ptr<StateMachineEventPropertyArray>>,
    // Offset: 80
    pub transitions: Option<Ptr<StateMachineTransitionInfoArray>>,
    // Offset: 88
    pub generator: Option<Ptr<Generator>>,
    // Offset: 96
    pub name: String,
    // Offset: 104
    pub state_id: i32,
    // Offset: 108
    pub probability: f32,
    // Offset: 112
    pub enable: bool,
    // Offset: 113
    pub has_eventless_transitions: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb5efa236)]
pub struct StateListener {
    // Offset: 0
    pub object: ReferencedObject,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x704a19af)]
pub struct StateMachineTransitionInfoArray {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub transitions: Array<StateMachineTransitionInfo>,
    // Offset: 32
    pub has_eventless_transitions: bool,
    // Offset: 33
    pub has_time_bounded_transitions: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xcdec8025)]
pub struct StateMachineTransitionInfo {
    // Offset: 0
    pub trigger_interval: StateMachineTimeInterval,
    // Offset: 16
    pub initiate_interval: StateMachineTimeInterval,
    // Offset: 32
    pub transition: Option<Ptr<TransitionEffect>>,
    // Offset: 40
    pub condition: Option<Ptr<Condition>>,
    // Offset: 48
    pub event_id: i32,
    // Offset: 52
    pub to_state_id: i32,
    // Offset: 56
    pub from_nested_state_id: i32,
    // Offset: 64
    pub priority: i16,
    // Offset: 66
    pub flags: i16,
    // Pad struct to 64-bit.
    _pad0: u32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x60a881e5)]
pub struct StateMachineTimeInterval {
    // Offset: 0
    pub enter_event_id: i32,
    // Offset: 4
    pub exit_event_id: i32,
    // Offset: 8
    pub enter_time: f32,
    // Offset: 12
    pub exit_time: f32,
}

#[derive(Copy, Clone, Debug, HkEnum)]
#[size(i8)]
pub enum StartStateMode {
    Default = 0,
    Sync = 1,
    Random = 2,
    Chooser = 3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, HkEnum)]
#[size(i8)]
pub enum StateMachineSelfTransitionMode {
    NoTransition = 0,
    TransitionToStartState = 1,
    ForceTransitionToStartState = 2,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xeca9d564)]
pub struct TransitionEffect {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub self_transition_mode: i8,
    // Offset: 137
    pub event_mode: i8,
    // Offset: 138
    pub default_event_mode: i8,
    // Offset: 144
    pub patched_binding_info: Option<OpaquePtr>,
    // Offset: 152
    pub from_generator: Option<OpaquePtr>,
    // Offset: 160
    pub to_generator: Option<OpaquePtr>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb5efa236)]
pub struct Condition {
    // Offset: 0
    pub object: ReferencedObject,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x3e0fd810)]
pub struct Event {
    // Offset: 0
    pub base: EventBase,
    // Offset: 16
    pub sender: Option<OpaquePtr>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x76bddb31)]
pub struct EventBase {
    // Offset: 0
    pub id: i32,
    // Offset: 8
    pub payload: Option<Ptr<EventPayload>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb5efa236)]
pub struct EventPayload {
    // Offset: 0
    pub object: ReferencedObject,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb5efa236)]
pub struct CustomIdSelector {
    // Offset: 0
    pub object: ReferencedObject,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x71957c2d)]
pub struct StateMachineEventPropertyArray {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub events: Array<EventProperty>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0db38a15)]
pub struct EventProperty {
    // Offset: 0
    pub base: EventBase,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc499fc9e)]
pub struct ModifierGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub modifier: Ptr<Modifier>,
    // Offset: 144
    pub generator: Ptr<Generator>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc811e80c)]
pub struct Modifier {
    // Offset: 0
    pub node: Node,
    // Offset: 80
    pub enable: bool,
    // Offset: 81
    _pad0: [u8; 3],
    _pad1: [u8; 4],
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0ded564c)]
pub struct ModifierList {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub modifiers: Array<Ptr<Modifier>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x35ef356b)]
pub struct EventDrivenModifier {
    // Offset: 0
    pub parent: ModifierWrapper,
    // Offset: 96
    pub activate_event_id: i32,
    // Offset: 100
    pub deactivate_event_id: i32,
    // Offset: 104
    pub active_by_default: bool,
    // OffseT: 105
    pub is_active: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xfe7050f4)]
pub struct ModifierWrapper {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub modifer: Ptr<Modifier>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x56341150)]
pub struct BSGetTimeStepModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub time_step: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x4a3ac449)]
pub struct EvaluateExpressionModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub expressions: Ptr<ExpressionDataArray>,
    // Offset: 96
    pub compiled_expression_set: Option<OpaquePtr>,
    // Offset: 104
    pub internal_expression_data: Array<()>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x1ebfc6d7)]
pub struct ExpressionDataArray {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub expressions_data: Array<ExpressionData>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x6740042a)]
pub struct ExpressionData {
    // Offset: 0
    pub expression: String,
    // Offset: 8
    pub assignment_variable_index: i32,
    // Offset: 12
    pub assignment_event_index: i32,
    // Offset: 16
    pub event_mode: i8,
    // Offset: 17
    pub raised_event: bool,
    // Offset: 18
    pub was_true_in_previous_frame: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x98d623bc)]
pub struct TwistModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 96
    pub axis_of_rotation: Vec4,
    // Offset: 112
    pub twist_angle: f32,
    // Offset: 116
    pub start_bone_index: i16,
    // Offset: 118
    pub end_bone_index: i16,
    // Offset: 120
    pub set_angle_method: i8,
    // Offset: 121
    pub rotation_axis_coordinates: i8,
    // Offset: 122
    pub is_additive: bool,
    // Offset: 128
    pub bone_chain_indices: Array<()>,
    // Offset: 144
    pub parent_bone_indices: Array<()>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x4c10e936)]
pub struct BSInterpValueModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub source: f32,
    // Offset: 92
    pub target: f32,
    // Offset: 96
    pub result: f32,
    // Offset: 100
    pub gain: f32,
    // Offset: 104
    pub time_step: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc9ece705)]
pub struct EventsFromRangeModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub input_value: f32,
    // Offset: 92
    pub lower_bound: f32,
    // Offset: 96
    pub event_ranges: Ptr<EventRangeDataArray>,
    // Offset: 104
    pub was_active_in_previous_frame: Array<()>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x83502c2d)]
pub struct EventRangeDataArray {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub event_data: Array<EventRangeData>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x6cb92c76)]
pub struct EventRangeData {
    // Offset: 0
    pub upper_bound: f32,
    _pad0: u32,
    // Offset: 8
    pub event: EventProperty,
    // Offset: 24
    pub event_mode: EventRangeMode,
    // Pad to 64 bit.
    _pad1: [u8; 7],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, HkEnum)]
#[size(i8)]
pub enum EventRangeMode {
    OnEnterRange = 0,
    WhenInRange = 1,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xf0393a52)]
pub struct FootIkControlsModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    _pad0: u64,
    // Offset: 96
    pub control_data: FootIkControlData,
    // Offset: 176
    pub legs: Array<FootIkControlsModifierLeg>,
    // Offset: 192
    pub error_out_translation: Vec4,
    // Offset: 208
    pub align_with_ground_rotation: Quat,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xd613e941)]
pub struct FootIkControlData {
    // Offset: 0
    pub gains: FootIkGains,
    // Offset: 48
    pub enabled: [f32; 8],
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xdf79ffa5)]
pub struct FootIkGains {
    // Offset: 0
    pub on_off_gain: f32,
    // Offset: 4
    pub ground_ascending_gain: f32,
    // Offset: 8
    pub ground_descending_gain: f32,
    // Offset: 12
    pub foot_planted_gain: f32,
    // Offset: 16
    pub foot_raised_gain: f32,
    // Offset: 20
    pub foot_locking_gain: f32,
    // Offset: 24
    pub world_from_model_feedback_gain: f32,
    // Offset: 28
    pub error_up_down_bias: f32,
    // Offset: 32
    pub align_world_from_model_gain: f32,
    // Offset: 36
    pub hip_orientation_gain: f32,
    // Offset: 40
    pub max_knee_angle_difference: f32,
    // Offset: 44
    pub ankle_orientation_gain: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb68b350a)]
pub struct FootIkControlsModifierLeg {
    // Offset: 0
    pub ground_position: Vec4,
    // Offset: 16
    pub ungrounded_event: EventProperty,
    // Offset: 32
    pub vertical_error: f32,
    // Offset: 36
    pub hit_something: bool,
    // Offset: 37
    pub is_planted_ms: bool,
    // Offset: 38
    pub enabled: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x5901a37d)]
pub struct BSLookAtModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub additive: bool,
    // Offset: 89
    pub clamp_additive: bool,
    // Offset: 90
    pub target: i8,
    // Offset: 91
    pub head_tracking_on_1: bool,
    // Offset: 92
    pub head_tracking_on_2: bool,
    // Offset: 93
    pub character_type: i8,
    // Offset: 96
    pub root_bone: BSLookAtModifierBoneData,
    // Offset: 624
    pub bones: Array<BSLookAtModifierBoneData>,
    // Offset: 640
    pub eye_bones: Array<BSLookAtModifierBoneData>,
    // Offset: 656
    pub use_individual_limit_angles: bool,
    // Offset: 660
    pub limit_angle_threshold_degrees: f32,
    // Offset: 664
    pub continue_look_outside_of_limit: bool,
    // Offset: 668
    pub on_gain: f32,
    // Offset: 672
    pub off_gain: f32,
    // Offset: 676
    pub on_lead_in: f32,
    // Offset: 680
    pub off_lead_in: f32,
    // Offset: 684
    pub use_bone_gains: bool,
    // Offset: 688
    pub ref_pose_blend_gain: f32,
    // Offset: 692
    pub restore_default_ref_pose_event_id: i32,
    // Offset: 704
    pub target_location: Vec4,
    // Offset: 720
    pub target_outside_limits: bool,
    // Offset: 728
    pub target_outside_of_limit_event: EventProperty,
    // Offset: 744
    pub snap_to_target_event_id: i32,
    // Offset: 748
    pub look_active: bool,
    // Offset: 749
    pub modification_active: bool,
    // Offset: 752
    pub look_at_camera_x: f32,
    // Offset: 756
    pub look_at_camera_y: f32,
    // Offset: 760
    pub look_at_camera_z: f32,
    // Offset: 764
    pub draw_debug_info: bool,
    // Offset: 765
    pub prev_look_at_target: bool,
    // Offset: 766
    pub target_outside_limits_prev: bool,
    // Offset: 767
    pub buse_custom_ref_pose: bool,
    // Offset: 768
    pub ref_pose_gain: f32,
    // Offset: 772
    pub snap_to_target: bool,
    // Offset: 776
    pub time_step: f32,
    // Offset: 780
    pub cur_look_at_target: bool,
    // Offset: 784
    pub cur_target_location: Vec4,
    // Offset: 800
    pub ball_bones_valid: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xf3b4d9be)]
pub struct BSLookAtModifierBoneData {
    // Offset: 0
    pub index: i16,
    // Offset: 16
    pub fwd_axis_ls: Vec4,
    // Offset: 32
    pub up_axis_ls: Vec4,
    // Offset: 48
    pub specify_axes_by_color: bool,
    // Offset: 49
    pub forward_axis_color: i8,
    // Offset: 50
    pub up_axis_color: i8,
    // Offset: 52
    pub limit_angle_degrees: f32,
    // Offset: 56
    pub limit_angle_degrees_left: f32,
    // Offset: 60
    pub limit_angle_degrees_right: f32,
    // Offset: 64
    pub limit_angle_degrees_up: f32,
    // Offset: 68
    pub limit_angle_degrees_down: f32,
    // Offset: 72
    pub on_gain: f32,
    // Offset: 76
    pub off_gain: f32,
    // Offset: 80
    pub on_lead_in: f32,
    // Offset: 84
    pub off_lead_in: f32,
    // Offset: 88
    pub rotate_about_axes: bool,
    // Offset: 89
    pub enabled: bool,
    // Offset: 90
    pub draw_debug_info: bool,
    // Offset: 96
    pub axis_fwd_ls: Vec4,
    // Offset: 112
    pub axis_up_ls: Vec4,
    // Offset: 128
    pub axis_side_ls: Vec4,
    // Offset: 144
    pub current_fwd_axis_ls: Vec4,
    // Offset: 160
    pub current_rot_ms: Quat,
    // Offset: 176
    pub current_gain: f32,
    // Offset: 192
    pub default_ref_transform_ms: QsTransform,
    // Offset: 240
    pub default_ref_transform_ls: QsTransform,
    // Offset: 288
    pub default_offset_fwd: f32,
    // Offset: 292
    pub default_offset_vert: f32,
    // Offset: 304
    pub custom_ref_transform_ms: QsTransform,
    // Offset: 352
    pub custom_ref_transform_ls: QsTransform,
    // Offset: 400
    pub custom_offset_fwd: f32,
    // Offset: 404
    pub custom_offset_vert: f32,
    // Offset: 416
    pub current_ref_transform_ms: QsTransform,
    // Offset: 464
    pub current_ref_transform_ls: QsTransform,
    // Offset: 512
    pub current_offset_fwd: f32,
    // Offset: 516
    pub current_offset_vert: f32,
}

// TODO: Is this the correct order?
#[derive(Clone, Debug, HkStruct)]
pub struct QsTransform {
    // Offset: 0
    pub translation: Vec4,
    // Offset: 16
    pub rotation: Quat,
    // Offset: 32
    pub scale: Vec4,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x1fc841a6)]
pub struct BSOffsetAnimationGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    _pad0: u64,
    // Offset: 144
    pub p_default_generator: Option<Ptr<Generator>>,
    // Offset: 152
    _pad1: u64,
    // Offset: 160
    pub p_offset_clip_generator: Option<Ptr<Generator>>,
    // Offset: 168
    pub f_offset_variable: f32,
    // Offset: 172
    pub f_offset_range_start: f32,
    // Offset: 176
    pub f_offset_range_end: f32,
    // Offset: 184
    pub bone_offset_a: Array<()>,
    // Offset: 200
    pub bone_index_a: Array<()>,
    // Offset: 216
    pub f_current_percentage: f32,
    // Offset: 220
    pub i_current_frame: u32,
    // Offset: 224
    pub b_zero_offset: bool,
    // Offset: 225
    pub b_offset_valid: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xce45c088)]
pub struct BlenderGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub reference_pose_weight_threshold: f32,
    // Offset: 140
    pub blend_parameter: f32,
    // Offset: 144
    pub min_cyclic_blend_parameter: f32,
    // Offset: 148
    pub max_cyclic_blend_parameter: f32,
    // Offset: 152
    pub index_of_sync_master_child: i16,
    // Offset: 154
    pub flags: BlenderGeneratorFlags,
    // Offset: 156
    pub subtract_last_child: bool,
    // Offset: 160
    pub children: Array<Ptr<BlenderGeneratorChild>>,
    // Offset: 176
    pub children_internal_states: Array<()>,
    // Offset: 192
    pub sorted_children: Array<()>,
    // Offset: 208
    pub end_interval_weight: f32,
    // Offset: 212
    pub num_active_children: i32,
    // Offset: 216
    pub begin_interval_index: i16,
    // Offset: 218
    pub end_interval_index: i16,
    // Offset: 220
    pub init_sync: bool,
    // Offset: 221
    pub do_subtractive_blend: bool,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct BlenderGeneratorFlags: u16 {
        const SYNC = 0x0001;
        const SMOOTH_WEIGHTS = 0x0004;
        const PARAMETRIC_BLEND = 0x0010;
        const PARAMETRIC_BLEND_CYLIC = 0x0020;
    }
}

impl_decode_for_bitflags!(BlenderGeneratorFlags: u16);

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb35bbfd3)]
pub struct BlenderGeneratorChild {
    // Offset: 0
    pub parent: Bindable,
    // Offset: 48
    pub generator: Option<Ptr<Generator>>,
    // Offset: 56
    pub bone_weights: Option<Ptr<BoneWeightArray>>,
    // Offset: 64
    pub weight: f32,
    // Offset: 68
    pub world_from_model_weight: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xe7633191)]
pub struct BSBehaviorGraphSwapGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    _pad0: u64,
    // Offset: 144
    pub p_default_generator: Option<Ptr<Generator>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0d4cc9f6)]
pub struct ClipGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub animation_bundle_name: String,
    // Offset: 144
    pub animation_name: String,
    // Offset: 152
    pub triggers: Option<Ptr<ClipTriggerArray>>,
    // Offset: 160
    pub user_partition_mask: u32,
    // Offset: 164
    pub crop_start_amount_local_time: f32,
    // Offset: 168
    pub crop_end_amount_local_time: f32,
    // Offset: 172
    pub start_time: f32,
    // Offset: 176
    pub playback_speed: f32,
    // Offset: 180
    pub enforced_duration: f32,
    // Offset: 184
    pub user_controlled_time_fraction: f32,
    // Offset: 188
    pub animation_binding_index: i16,
    // Offset: 190
    pub mode: PlaybackMode,
    // Offset: 191
    pub flags: ClipGeneratorFlags,
    // Offset: 192
    pub anim_datas: Array<()>,
    // Offset: 208
    pub animation_control: Option<OpaquePtr>,
    // Offset: 216
    pub original_triggers: Option<OpaquePtr>,
    // Offset: 224
    pub mapper_data: Option<OpaquePtr>,
    // Offset: 232
    pub binding: Option<OpaquePtr>,
    // Offset: 240
    pub num_animation_tracks: i32,
    // Offset: 256
    pub extraced_motion: QsTransform,
    // Offset: 304
    pub echos: Array<()>,
    // Offset: 320
    pub local_time: f32,
    // Offset: 324
    pub time: f32,
    // Offset: 328
    pub previous_user_controlled_time_fraction: f32,
    // Offset: 332
    pub buffer_size: i32,
    // Offset: 336
    pub at_end: bool,
    // Offset: 337
    pub ignore_start_time: bool,
    // Offset: 338
    pub ping_pong_backward: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xf757cd66)]
pub struct ClipTriggerArray {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub triggers: Array<ClipTrigger>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x7eb45cea)]
pub struct ClipTrigger {
    // Offset: 0
    pub local_time: f32,
    _pad0: u32,
    // Offset: 8
    pub event: EventProperty,
    // Offset: 24
    pub relative_to_end_of_clip: bool,
    // Offset: 25
    pub acyclic: bool,
    // Offset: 26
    pub is_annotation: bool,
    // Pad struct to 64 bits.
    _pad1: [u8; 5],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, HkEnum)]
#[size(i8)]
pub enum PlaybackMode {
    Single = 0,
    Looping = 1,
    UserControlled = 2,
    PingPong = 3,
    Count = 4,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct ClipGeneratorFlags: u8 {
        const NO_ANNOTATION_TRIGGERS = 0x10;
    }
}

impl_decode_for_bitflags!(ClipGeneratorFlags: u8);

#[derive(Clone, Debug, HkStruct)]
#[signature(0x4e45d0f7)]
pub struct KeyframesBonesModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub keyframe_info: Array<KeyframesBonesModifierKeyframeInfo>,
    // Offset: 104
    pub keyframed_bones_list: Option<Ptr<BoneIndexArray>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x72deb7a6)]
pub struct KeyframesBonesModifierKeyframeInfo {
    // Offset: 0
    pub keyframed_position: Vec4,
    // Offset: 16
    pub keyframed_rotation: Quat,
    // Offset: 32
    pub bone_index: i16,
    // Offset: 34
    pub is_valid: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x3d26f425)]
pub struct BoneIndexArray {
    // Offset: 0
    pub parent: Bindable,
    // Offset: 48
    pub bone_indices: Array<i16>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x7e6d66c9)]
pub struct BSRagdollContactListenerModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub contact_event: EventProperty,
    // Offset: 104
    pub bones: Option<Ptr<BoneIndexArray>>,
    // Offset: 112
    pub throw_event: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x5bf1f5cf)]
pub struct BSIsActiveModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub b_is_active_0: bool,
    // Offset: 89
    pub b_invert_active_0: bool,
    // Offset: 90
    pub b_is_active_1: bool,
    // Offset: 91
    pub b_invert_active_1: bool,
    // Offset: 92
    pub b_is_active_2: bool,
    // Offset: 93
    pub b_invert_active_2: bool,
    // Offset: 94
    pub b_is_active_3: bool,
    // Offset: 95
    pub b_invert_active_3: bool,
    // Offset: 96
    pub b_is_active_4: bool,
    // Offset: 97
    pub b_invert_active_4: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xeed8d5cd)]
pub struct ManualSelectorGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub generators: Array<Ptr<Generator>>,
    // Offset: 152
    pub selected_generator_index: i16,
    // Offset: 160
    pub index_selector: Option<Ptr<CustomIdSelector>>,
    // Offset: 168
    pub selected_index_can_change_after_active: bool,
    // Offset: 176
    pub generator_changed_transition_effect: Option<Ptr<TransitionEffect>>,
    // Offset: 184
    pub current_generator_index: i16,
    // Offset: 186
    pub generator_index_at_activate: i16,
    // Offset: 192
    pub active_transitions: Array<()>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x14e54c5c)]
pub struct BlendingTransitionEffect {
    // Offset: 0
    pub parent: TransitionEffect,
    // Offset: 168
    pub duration: f32,
    // Offset: 172
    pub to_generator_start_time_fraction: f32,
    // Offset: 176
    pub flags: u16,
    // Offset: 178
    pub end_mode: i8,
    // Offset: 179
    pub blend_curve: i8,
    // Offset: 180
    pub alignment_bone: i16,
    // Offset: 192
    pub from_pos: Vec4,
    // Offset: 208
    pub from_rot: Quat,
    // Offset: 224
    pub to_pos: Vec4,
    // Offset: 240
    pub to_rot: Quat,
    // Offset: 256
    pub last_pos: Vec4,
    // Offset: 272
    pub last_rot: Quat,
    // Offset: 288
    pub character_pose_at_beginning_of_transition: Array<()>,
    // Offset: 304
    pub time_remaining: f32,
    // Offset: 308
    pub time_in_transition: f32,
    // Offset: 312
    pub to_generator_self_transition_mode: i8,
    // Offset: 313
    pub initialize_character_pose: bool,
    // Offset: 314
    pub align_this_frame: bool,
    // Offset: 315
    pub alignment_finished: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xdf5fe694)]
pub struct StringEventPayload {
    // Offset: 0
    pub parent: EventPayload,
    // Offset: 16
    pub data: String,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x84519a60)]
pub struct GetUpModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 96
    pub ground_normal: Vec4,
    // Offset: 112
    pub duration: f32,
    // Offset: 116
    pub align_with_ground_duration: f32,
    // Offset: 120
    pub root_bone_index: i16,
    // Offset: 122
    pub other_bone_index: i16,
    // Offset: 124
    pub another_bone_index: i16,
    // Offset: 128
    pub time_since_begin: f32,
    // Offset: 132
    pub time_step: f32,
    // Offset: 136
    pub init_next_modify: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb7c3fe7d)]
pub struct RigidBodyRagdollControlsModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    _pad0: u64,
    // Offset: 96
    pub control_data: RigidBodyRagdollControlData,
    // Offset: 160
    pub bones: Option<Ptr<BoneIndexArray>>,
    // Offset: 168
    pub animation_blend_fraction: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x34a56bef)]
pub struct RigidBodyRagdollControlData {
    // Offset: 0
    pub keyframe_control_data: KeyframeControlData,
    // Offset: 48
    pub duration_to_blend: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xa3d0ac71)]
pub struct KeyframeControlData {
    // Offset: 0
    pub hierarchy_gain: f32,
    // Offset: 4
    pub velocity_damping: f32,
    // Offset: 8
    pub acceleration_gain: f32,
    // Offset: 12
    pub velocity_gain: f32,
    // Offset: 16
    pub position_gain: f32,
    // Offset: 20
    pub position_max_linear_velocity: f32,
    // Offset: 24
    pub position_max_angular_velocity: f32,
    // Offset: 28
    pub snap_gain: f32,
    // Offset: 32
    pub snap_max_linear_velocity: f32,
    // Offset: 36
    pub snap_max_angular_velocity: f32,
    // Offset: 40
    pub snap_max_linear_distance: f32,
    // Offset: 44
    pub snap_max_angular_distance: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x14f075d2)]
pub struct BSModifyOnceModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    _pad0: u64,
    // Offset: 96
    pub p_on_activate_modifier: Option<Ptr<Modifier>>,
    // Offset: 112
    pub p_on_deactivate_modifier: Option<Ptr<Modifier>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb2aa2982)]
pub struct PoseMatchingGenerator {
    // Offset: 0
    pub parent: BlenderGenerator,
    // Offset: 224
    pub world_from_model_rotation: Quat,
    // Offset: 240
    pub blend_speed: f32,
    // Offset: 244
    pub min_speed_to_switch: f32,
    // Offset: 248
    pub min_switch_time_no_error: f32,
    // Offset: 252
    pub min_switch_time_full_error: f32,
    // Offset: 256
    pub start_playing_event_id: i32,
    // Offset: 260
    pub start_matching_event_id: i32,
    // Offset: 264
    pub root_bone_index: i16,
    // Offset: 266
    pub other_bone_index: i16,
    // Offset: 268
    pub another_bone_index: i16,
    // Offset: 270
    pub pelvis_index: i16,
    // Offset: 272
    pub mode: i8,
    // Offset: 276
    pub current_match: i32,
    // Offset: 280
    pub best_match: i32,
    // Offset: 284
    pub time_since_better_match: i32,
    // Offset: 288
    pub error: f32,
    // Offset: 292
    pub reset_current_match_local_time: f32,
    // Offset: 296
    pub pose_matching_utility: Option<Ptr<()>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x522211cb)]
pub struct PoweredRagdollControlsModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    _pad0: u64,
    // Offset: 96
    pub control_data: PoweredRagdollControlData,
    // Offset: 116
    _pad1: [u32; 3],
    // Offset: 128
    pub bones: Option<Ptr<BoneIndexArray>>,
    // Offset: 136
    pub world_from_model_mode_data: WorldFromModelModeData,
    // Offset: 144
    pub bone_weights: Option<Ptr<BoneWeightArray>>,
    // Offset: 152
    pub animation_blend_fraction: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0f5ba21b)]
pub struct PoweredRagdollControlData {
    // Offset: 0
    pub max_force: f32,
    // Offset: 4
    pub tau: f32,
    // Offset: 8
    pub damping: f32,
    // Offset: 12
    pub proportional_recovery_velocity: f32,
    // Offset: 16
    pub constant_recovery_velocity: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xa3af8783)]
pub struct WorldFromModelModeData {
    // Offset: 0
    pub pose_matching_bone_0: i16,
    // Offset: 2
    pub pose_matching_bone_1: i16,
    // Offset: 4
    pub pose_matching_bone_2: i16,
    // Offset: 6
    pub mode: i8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x7880c5e2)]
pub struct TimerModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub alarm_time_seconds: f32,
    _pad0: u32,
    // Offset: 96
    pub alarm_event: EventProperty,
    // Offset: 112
    pub seconds_elapsed: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x64a6ca08)]
pub struct BSAssignVariablesModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub float_variable_1: f32,
    // Offset: 92
    pub float_value_1: f32,
    // Offset: 96
    pub float_variable_2: f32,
    // Offset: 100
    pub float_value_2: f32,
    // Offset: 104
    pub float_variable_3: f32,
    // Offset: 108
    pub float_value_3: f32,
    // Offset: 112
    pub float_variable_4: f32,
    // Offset: 116
    pub float_value_4: f32,
    // Offset: 120
    pub float_variable_5: f32,
    // Offset: 124
    pub float_value_5: f32,
    // Offset: 128
    pub float_variable_6: f32,
    // Offset: 132
    pub float_value_6: f32,
    // Offset: 136
    pub float_variable_7: f32,
    // Offset: 140
    pub float_value_7: f32,
    // Offset: 144
    pub float_variable_8: f32,
    // Offset: 148
    pub float_value_8: f32,
    // Offset: 152
    pub float_variable_9: f32,
    // Offset: 156
    pub float_value_9: f32,
    // Offset: 160
    pub float_variable_10: f32,
    // Offset: 164
    pub float_value_10: f32,
    // Offset: 168
    pub float_variable_11: f32,
    // Offset: 172
    pub float_value_11: f32,
    // Offset: 176
    pub float_variable_12: f32,
    // Offset: 180
    pub float_value_12: f32,
    // Offset: 184
    pub float_variable_13: f32,
    // Offset: 188
    pub float_value_13: f32,
    // Offset: 192
    pub float_variable_14: f32,
    // Offset: 196
    pub float_value_14: f32,
    // Offset: 200
    pub float_variable_15: f32,
    // Offset: 204
    pub float_value_15: f32,
    // Offset: 208
    pub float_variable_16: f32,
    // Offset: 212
    pub float_value_16: f32,
    // Offset: 216
    pub float_variable_17: f32,
    // Offset: 220
    pub float_value_17: f32,
    // Offset: 224
    pub float_variable_18: f32,
    // Offset: 228
    pub float_value_18: f32,
    // Offset: 232
    pub float_variable_19: f32,
    // Offset: 236
    pub float_value_19: f32,
    // Offset: 240
    pub float_variable_20: f32,
    // Offset: 244
    pub float_value_20: f32,
    // Offset: 248
    pub int_variable_1: i32,
    // Offset: 252
    pub int_value_1: i32,
    // Offset: 256
    pub int_variable_2: i32,
    // Offset: 260
    pub int_value_2: i32,
    // Offset: 264
    pub int_variable_3: i32,
    // Offset: 268
    pub int_value_3: i32,
    // Offset: 272
    pub int_variable_4: i32,
    // Offset: 276
    pub int_value_4: i32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc6b82462)]
pub struct BSEventOnDeactivateModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub event: EventProperty,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb4e0c52f)]

pub struct LayerGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub layers: Array<Ptr<Layer>>,
    // Offset: 152
    pub index_of_sync_master_child: i16,
    // Offset: 154
    pub flags: u16,
    // Offset: 156
    pub num_active_layers: i32,
    // Offset: 160
    pub layer_internal_states: Array<()>,
    // Offset: 176
    pub init_sync: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x2916a243)]
pub struct Layer {
    // Offset: 0
    pub parent: Bindable,
    // Offset: 48
    pub generator: Ptr<Generator>,
    // Offset: 56
    pub weight: f32,
    // Offset: 64
    pub bone_weights: Option<Ptr<BoneWeightArray>>,
    // Offset: 72
    pub fade_in_duration: f32,
    // Offset: 76
    pub fade_out_duration: f32,
    // Offset: 80
    pub on_event_id: i32,
    // Offset: 84
    pub off_event_id: i32,
    // Offset: 88
    pub on_by_default: bool,
    // Offset: 89
    pub use_motion: bool,
    // Offset: 90
    pub force_full_fade_durations: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x154fcfaa)]
pub struct BehaviorReferenceGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub behavior_name: String,
    // Offset: 144
    pub behavior: Option<OpaquePtr>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0320ef75)]
pub struct BSEventEveryNEventsModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub event_to_check_for: EventProperty,
    // Offset: 104
    pub event_to_send: EventProperty,
    // Offset: 120
    pub number_of_events_before_send: i8,
    // Offset: 121
    pub minimum_number_of_events_before_send: i8,
    // Offset: 122
    pub randomize_number_of_events: bool,
    // Offset: 124
    pub number_of_events_seen: i32,
    // Offset: 128
    pub calculate_number_of_events_before_send: i8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x86401931)]
pub struct BSCyclicBlendTransitionGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    _pad0: u64,
    // Offset: 144
    pub p_blender_generator: Option<Ptr<BlenderGenerator>>,
    // Offset: 152
    pub event_to_freeze_blend_value: EventProperty,
    // Offset: 168
    pub event_to_cross_blend: EventProperty,
    // Offset: 184
    pub transition_out_event: EventProperty,
    // Offset: 200
    pub transition_in_event: EventProperty,
    // Offset: 216
    pub f_blend_parameter: f32,
    // Offset: 220
    pub f_transition_duration: f32,
    // Offset: 224
    pub e_blend_curve: i8,
    // Offset: 232
    pub p_transition_blender_generators_a: Array<()>,
    // Offset: 248
    pub sorted_children: Array<()>,
    // Offset: 264
    pub p_temp_output: Option<OpaquePtr>,
    // Offset: 272
    pub current_mode: i8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc932a82a)]
pub struct BSiStateTaggingGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    _pad0: u64,
    // Offset: 144
    pub p_default_generator: Option<Ptr<Generator>>,
    // Offset: 152
    pub i_state_to_set_as: i32,
    // Offset: 156
    pub i_priority: i32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0d282dc7)]
pub struct HandIkControlsModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub hands: Array<HandIkControlsModifierHand>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x9c72e9e3)]
pub struct HandIkControlsModifierHand {
    // Offset: 0
    pub control_data: HandIkControlData,
    // Offset: 96
    pub hand_index: i32,
    // Offset: 100
    pub enable: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xd72b8d17)]
pub struct HandIkControlData {
    // Offset: 0
    pub target_position: Vec4,
    // Offset: 16
    pub target_rotation: Quat,
    // Offset: 32
    pub target_normal: Vec4,
    // Offset: 48
    pub target_handle: Option<Ptr<Handle>>,
    // Offset: 56
    pub transform_on_fraction: f32,
    // Offset: 60
    pub normal_on_fraction: f32,
    // Offset: 64
    pub fade_in_duration: f32,
    // Offset: 68
    pub fade_out_duration: f32,
    // Offset: 72
    pub extrapolation_time_step: f32,
    // Offset: 76
    pub handle_change_speed: f32,
    // Offset: 80
    pub handle_change_mode: i8,
    // Offset: 81
    pub fix_up: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x43c54f34)]

pub struct Handle {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub frame: Option<Ptr<HkLocalFrame>>,
    // Offset: 24
    pub rigid_body: Option<OpaquePtr>,
    // Offset: 32
    pub character: Option<OpaquePtr>,
    // Offset: 40
    pub animation_bone_index: i16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x702ba6fc)]
pub struct GetHandleOnBoneModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub handle: Option<Ptr<Handle>>,
    // Offset: 96
    pub local_frame_name: String,
    // Offset: 104
    pub ragdoll_bone_index: i16,
    // Offset: 106
    pub animation_bone_index: i16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x9266ffac)]
pub struct BSTimerModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub alarm_time_seconds: f32,
    _pad0: u32,
    // Offset: 96
    pub alarm_event: EventProperty,
    // Offset: 112
    pub reset_alarm: bool,
    // Offset: 116
    pub seconds_elapsed: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xcda56038)]
pub struct BSDirectAtModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub direct_at_on: bool,
    // Offset: 89
    pub target: i8,
    // Offset: 90
    pub context: i8,
    // Offset: 92
    pub source_bone_index: i16,
    // Offset: 94
    pub offsets_character_relative: bool,
    // Offset: 96
    pub start_bone_index: i16,
    // Offset: 98
    pub end_bone_index: i16,
    // Offset: 100
    pub limit_heading_degrees_ccw: f32,
    // Offset: 104
    pub limit_heading_degrees_cw: f32,
    // Offset: 108
    pub limit_pitch_degrees_up: f32,
    // Offset: 112
    pub limit_pitch_degrees_down: f32,
    // Offset: 116
    pub on_gain: f32,
    // Offset: 120
    pub off_gain: f32,
    // Offset: 124
    pub on_lead_in: f32,
    // Offset: 128
    pub off_lead_in: f32,
    // Offset: 132
    pub initial_heading_offset: f32,
    // Offset: 136
    pub initial_pitch_offset: f32,
    // Offset: 140
    pub initial_gain: f32,
    // Offset: 144
    pub allow_source_bone_pose_capture: bool,
    // offset: 148
    pub source_bone_pose_capture_blend_gain: f32,
    // offset: 152
    pub use_current_source_bone_pose_event_id: i32,
    // Offset: 156
    pub counteract_source_bone_world_space_delta: bool,
    // Offset: 160
    pub snap_to_target_event_id: i32,
    // Offset: 164
    pub direct_at_camera_x: f32,
    // Offset: 168
    pub direct_at_camera_y: f32,
    // Offset: 172
    pub direct_at_camera_z: f32,
    // Offset: 176
    pub draw_debug_info: bool,
    // Offset: 177
    pub active: bool,
    // Offset: 180
    pub current_heading_offset: f32,
    // Offset: 184
    pub current_pitch_offset: f32,
    // Offset: 188
    pub current_gain: f32,
    // Offset: 192
    pub first_modify: bool,
    // Offset: 196
    pub heading_offset: f32,
    // Offset: 200
    pub pitch_offset: f32,
    // Offset: 204
    pub buse_source_bone_pose_capture: bool,
    // Offset: 208
    pub source_bone_pose_gain: f32,
    // Offset: 224
    pub source_bone_pose_capture: QsTransform,
    // Offset: 272
    pub source_bone_pose: QsTransform,
    // Offset: 320
    pub source_bone_dir_ws: Vec4,
    // Offset: 336
    pub snap_to_target: bool,
    // Offset: 340
    pub time_step: f32,
    // Offset: 344
    pub direct_at_target: bool,
    // Offset: 352
    pub target_location: Vec4,
    // Offset: 368
    pub world_transform: QsTransform,
    // Offset: 416
    pub bone_chain_indices: Array<()>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x78a69526)]

pub struct ExpressionCondition {
    // Offset: 0
    pub parent: Condition,
    // Offset: 16
    pub expression: String,
    // Offset: 24
    pub compiled_expression_set: Option<Ptr<()>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x44208f07)]
pub struct GeneratorTransitionEffect {
    // Offset: 0
    pub parent: TransitionEffect,
    // Offset: 168
    pub transition_generator: Option<Ptr<Generator>>,
    // Offset: 176
    pub blend_in_duration: f32,
    // Offset: 180
    pub blend_out_duration: f32,
    // Offset: 184
    pub sync_to_generator_start_time: bool,
    // Offset: 185
    pub child_state: i8,
    // Offset: 188
    pub time_in_transition: f32,
    // Offset: 192
    pub duration: f32,
    // Offset: 196
    pub effective_blend_in_duration: f32,
    // Offset: 200
    pub effective_blend_out_duration: f32,
    // Offset: 204
    pub to_generator_state: i8,
    // Offset: 205
    pub echo_transition_generator: bool,
    // Offset: 206
    pub to_generator_self_transition_mode: i8,
    // Offset: 207
    pub just_activated: bool,
    // Offset: 208
    pub update_active_nodes: bool,
    // Offset: 209
    pub stage: i8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xbc1536ee)]
pub struct ReferencePoseGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    pub skeleton: Option<OpaquePtr>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x10a3e22b)]
pub struct BSDirectAtCapturePoseModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub capture_pose_event_id: i32,
    // Offset: 92
    pub capture_pose_on_activate: bool,
    // Offset: 93
    pub capture_pose: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xb3ef2ee5)]
pub struct MirrorModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub is_additive: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x921ad1a5)]
pub struct BSEventOnFalseToTrueModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub b_enable_event_1: bool,
    // Offset: 89
    pub b_variable_to_test_1: bool,
    _pad0: [u8; 6],
    // Offset: 96
    pub event_to_send_1: EventProperty,
    // Offset: 112
    pub b_enable_event_2: bool,
    // Offset: 113
    pub b_variable_to_test_2: bool,
    _pad1: [u8; 6],
    // Offset: 120
    pub event_to_send_2: EventProperty,
    // Offset: 136
    pub b_enable_event_3: bool,
    // Offset: 137
    pub b_variable_to_test_3: bool,
    _pad2: [u8; 6],
    // Offset: 144
    pub event_to_send_3: EventProperty,
    // Offset: 160
    pub b_slot1_activated_last_frame: bool,
    // Offset: 161
    pub b_slot2_activated_last_frame: bool,
    // Offset: 162
    pub b_slot3_activated_last_frame: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xcbc0b864)]
pub struct BSRootTwistModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub root_twist_angle: f32,
    // Offset: 92
    pub twist_type: TwistType,
    // Offset: 96
    pub calculated_rotation: QsTransform,
}

#[derive(Copy, Clone, Debug, HkEnum)]
#[size(i8)]
pub enum TwistType {
    AngleInDegrees = 0,
    Direction = 1,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x550fec31)]
pub struct BSRandomAlarmModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub alarm_time_seconds_min: f32,
    // Offset: 92
    pub alarm_time_seconds_max: f32,
    // Offset: 96
    pub alarm_event: EventProperty,
    // Offset: 112
    pub reset_alarm: bool,
    // Offset: 116
    pub seconds_remaining: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x450fc55e)]
pub struct BSBoneSwitchGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    _pad0: u64,
    // Offset: 144
    pub p_default_generator: Option<Ptr<Generator>>,
    // Offset: 152
    pub children_a: Array<Ptr<BSBoneSwitchGeneratorData>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0977b904)]
pub struct BSBoneSwitchGeneratorData {
    // Offset: 0
    pub parent: Bindable,
    // Offset: 48
    pub p_generator: Option<Ptr<Generator>>,
    // Offset: 56
    pub sp_bone_weight: Ptr<BoneWeightArray>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xe7633191)]
pub struct DynamicAnimationTaggingGenerator {
    // Offset: 0
    pub parent: Generator,
    // Offset: 136
    _pad0: u64,
    // Offset: 144
    pub p_default_generator: Option<Ptr<Generator>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xe49602af)]
pub struct BSPassByTargetTriggerModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 96
    pub target_position: Vec4,
    // Offset: 112
    pub radius: f32,
    // Offset: 128
    pub movement_direction: Vec4,
    // Offset: 144
    pub trigger_event: EventProperty,
    // Offset: 160
    pub target_passed: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xfe0a9cb4)]
pub struct BSTweenerModifier {
    // Offset: 0
    pub parent: Modifier,
    // Offset: 88
    pub tween_position: bool,
    // Offset: 89
    pub tween_rotation: bool,
    // Offset: 90
    pub use_tween_duration: bool,
    // Offset: 92
    pub tween_duration: f32,
    // Offset: 96
    pub target_position: Vec4,
    // Offset: 112
    pub target_rotation: Quat,
    // Offset: 128
    pub duration: f32,
    // Offset: 144
    pub start_transform: QsTransform,
    // Offset: 192
    pub time: f32,
}
