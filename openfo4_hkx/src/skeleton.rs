use glam::{Mat3, Mat4, Quat, Vec4};
use openfo4_hkx_derive::{HkEnum, HkStruct, signature};

use crate::behavior::QsTransform;
use crate::physics::{HkLocalFrame, PhysicsSystemData};
use crate::{Array, OpaquePtr, Ptr, ReferencedObject};

#[derive(Clone, Debug, HkStruct)]
pub struct Skeleton {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub name: String,
    // Offset: 24
    pub parent_indices: Array<u16>,
    // Offset: 40
    pub bones: Array<Bone>,
    // Offset: 56
    pub reference_pose: Array<QsTransform>,
    // Offset: 72
    pub reference_floats: Array<f32>,
    // Offset: 88
    pub float_slots: Array<String>,
    // Offset: 104
    pub local_frames: Array<SkeletonLocalFrameOnBone>,
    // Offset: 120
    pub partitions: Array<SkeletonPartition>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct Bone {
    // Offset: 0
    pub name: String,
    // Offset: 8
    pub lock_translation: bool,
}

#[derive(Clone, Debug, HkStruct)]
pub struct QTransform {
    // Offset: 0
    pub rotation: Quat,
    // Offset: 16
    pub translation: Vec4,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonLocalFrameOnBone {
    // Offset: 0
    pub local_frame: Option<Ptr<HkLocalFrame>>,
    // Offset: 8
    pub bone_index: i16,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonPartition {
    // Offset: 0
    pub name: String,
    // Offset: 8
    pub start_bone_index: i16,
    // Offset: 10
    pub num_bones: i16,
}

#[derive(Clone, Debug, HkStruct)]
pub struct RagdollData {
    // Offset: 0
    pub physics_system: PhysicsSystemData,
    // Offset: 120
    pub skeleton: Ptr<Skeleton>,
    // Offset: 128
    pub bone_to_body_map: Array<i32>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct RagdollConstraintData {
    pub _pad0: u64,
    pub _pad1: u64,
    pub _pad2: u64,
    pub _pad3: u64,
    // Offset: 32
    pub atoms: RagdollConstraintDataAtoms,
}

#[derive(Clone, Debug, HkStruct)]
pub struct RagdollConstraintDataAtoms {
    // Offset: 0
    pub transforms: SetLocalTransformsConstarintAtom,
    // Offset: 144
    pub setup_stabilization: SetupStabilizationAtom,
    // Offset: 160
    pub ragdoll_motors: RagdollMotorConstraintAtom,
    _pad0: u64,
    _pad1: u64,
    _pad2: u64,
    // Offset: 256
    pub ang_friction: AngFrictionContstraintAtom,
    // Offset: 272
    pub twist_limit: TwistLimitConstraintAtom,
    // Offset: 304
    pub cone_limit: ConeLimitConstraintAtom,
    // Offset: 336
    pub planes_limit: ConeLimitConstraintAtom,
    // Offset: 368
    pub ball_socket: BallSocketConstraintAtom,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConstraintAtom {
    // Offset: 0
    pub ty: u16,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SetLocalTransformsConstarintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    _pad0: u16,
    _pad1: u32,
    _pad2: u64,
    // Offset: 16
    pub transform_a: Mat4,
    // Offset: 80
    pub transform_b: Mat4,
}

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub enum AtomType {
//     Invalid,
//     Bridge,
//     SetLocalTransforms,
//     SetLocalTranslations,
//     SetLocalRotations,
//     BallSocket,
//     StiffSpring,
//     Lin,
//     LinSoft,
//     LinLimit,
//     LinFriction,
// }

#[derive(Clone, Debug, HkStruct)]
pub struct SetupStabilizationAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    _pad0: u8,
    // Offset: 4
    pub max_lin_impulse: f32,
    // Offset: 8
    pub max_ang_impulse: f32,
    // Offset: 12
    pub max_angle: f32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct RagdollMotorConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    _pad0: u8,
    // Offset: 4
    pub initialized_offset: i16,
    // Offset: 6
    pub previous_target_angles_offset: i16,
    // Offset: 8
    _pad1: u64,
    // Offset: 16
    pub target_brca: Mat3,
    // Offset: 64
    pub motors: Option<Ptr<ConstraintMotor>>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConstraintMotor {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub motor_type: u8,
    _pad0: u64,
}

#[derive(Clone, Debug, HkStruct)]
pub struct AngFrictionContstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    pub first_friction_axis: u8,
    // Offset: 4
    pub num_friction_axes: u8,
    _pad0: u8,
    _pad1: u16,
    // Offset: 8
    pub max_friction_torque: f32,
    _pad2: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct TwistLimitConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    pub twist_axis: u8,
    // Offset: 4
    pub ref_axis: u8,
    _pad0: u8,
    _pad1: u16,
    // Offset: 8
    pub min_angle: f32,
    // Offset: 12
    pub max_angle: f32,
    // Offset: 16
    pub angular_limits_tau_factor: f32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConeLimitConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    pub twist_axis_in_a: u8,
    // Offset: 4
    pub ref_axis_in_b: u8,
    // Offset: 5
    pub angle_measurement_mode: u8,
    // Offset: 6
    pub mem_offset_to_angle_offset: u8,
    _pad0: u8,
    // Offset:  8
    pub min_angle: f32,
    // Offset: 12
    pub max_angle: f32,
    // Offset: 16
    pub angular_limits_tau_factor: f32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct BallSocketConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub solving_method: u8,
    // Offset: 3
    pub bodies_to_notify: u8,
    // Offset: 4
    pub velocity_stabilization_factor: u8,
    // Offset: 5
    pub enable_linear_impulse_limit: bool,
    _pad0: u16,
    // Offset: 8
    pub breach_impulse: f32,
    // Offset: 12
    pub insertia_stabilization_factor: f32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct PositionConstraintMotor {
    // Offset: 32
    pub tau: f32,
    // Offset: 36
    pub damping: f32,
    // Offset: 40
    pub proportional_recovery_velocity: f32,
    // Offset: 44
    pub constant_recovery_velocity: f32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct LimitedForceConstraintMotor {
    // Offset: 0
    pub constraint_motor: ConstraintMotor,
    // Offset: 24
    pub min_force: f32,
    // Offset: 28
    pub max_force: f32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct LimitedHingeConstraintData {
    // Offset: 0
    pub data: ConstraintData,
    _pad0: u64,
    // Offset: 32
    pub atoms: LimitedHingeConstraintDataAtoms,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConstraintData {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub user_data: u64,
}

#[derive(Clone, Debug, HkStruct)]
pub struct LimitedHingeConstraintDataAtoms {
    // Offset: 0
    pub transforms: SetLocalTransformsConstarintAtom,
    // Offset: 144
    pub setup_stabilization: SetupStabilizationAtom,
    // Offset: 160
    pub ang_motor: AngMotorConstraintAtom,
    _pad0: u32,
    // Offset: 200
    pub ang_friction: AngFrictionContstraintAtom,
    // Offset: 216
    pub ang_limit: AngLimitConstraintAtom,
    // Offset: 232
    pub d2_ang: D2AngConstraintAtom,
    // Offset: 248
    pub ball_socket: BallSocketConstraintAtom,
}

#[derive(Clone, Debug, HkStruct)]
pub struct AngMotorConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    pub motor_axis: u8,
    // Offset: 4
    pub initialized_offset: i16,
    // Offset: 6
    pub previous_target_angle_offset: i16,
    // Offset: 8
    pub corresponding_ang_limit_solver_result_offset: i16,
    _pad0: u16,
    // Offset: 12
    pub target_angle: f32,
    // Offset: 16
    pub motor: Option<Ptr<ConstraintMotor>>,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct AngLimitConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub is_enabled: bool,
    // Offset: 3
    pub limit_axis: u8,
    // Offset: 4
    pub min_angle: f32,
    // Offset: 8
    pub max_angle: f32,
    // Offset: 12
    pub angular_limits_tau_factor: f32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct D2AngConstraintAtom {
    // Offset: 0
    pub atom: ConstraintAtom,
    // Offset: 2
    pub free_rotation_axis: u8,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonMapper {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub mapping: SkeletonMapperData,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonMapperData {
    // Offset: 0
    pub skeleton_a: Ptr<Skeleton>,
    // Offset: 8
    pub skeleton_b: Ptr<Skeleton>,
    // Offset: 16
    pub partition_map: Array<i16>,
    // Offset: 32
    pub simple_mapping_partition_ranges: Array<SkeletonMapperDataPartitionMappingRange>,
    // Offset: 48
    pub chain_mapping_partition_ranges: Array<SkeletonMapperDataPartitionMappingRange>,
    // Offset: 64
    pub simple_mappings: Array<SkeletonMapperDataSimpleMapping>,
    // Offset: 80
    pub chain_mappings: Array<SkeletonMapperDataChainMapping>,
    // Offset: 96
    pub unmapped_bones: Array<u16>,
    // Offset: 112
    pub extracted_motion_mapping: QTransform,
    // Offset: 160
    pub keep_unmapped_local: bool,
    _pad0: u8,
    _pad1: u16,
    // Offset: 164
    pub mapping_type: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonMapperDataPartitionMappingRange {
    // Offset: 0
    pub start_mapping_index: u32,
    // Offset: 4
    pub num_mappings: u32,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonMapperDataSimpleMapping {
    // Offset: 0
    pub bone_a: u16,
    // Offset: 2
    pub bone_b: u16,
    _pad0: u32,
    _pad1: u64,
    // Offset: 16
    pub a_from_b_transform: QsTransform,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SkeletonMapperDataChainMapping {
    // Offset: 0
    pub start_bone_a: u16,
    // Offset: 2
    pub end_bone_a: u16,
    // Offset: 4
    pub start_bone_b: u16,
    // Offset: 6
    pub end_bone_b: u16,
    _pad0: u64,
    // Offset: 16
    pub start_a_from_b_transform: QsTransform,
    // Offset: 64
    pub end_a_from_b_transform: QsTransform,
}

#[derive(Clone, Debug, HkStruct)]
pub struct MirroredSkeletionInfo {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub mirror_axis: Vec4,
    // Offset: 32
    pub bone_pair_map: Array<i16>,
    // Offset: 48
    pub partition_pair_map: Array<i16>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct FootIkDriverInfo {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub legs: Array<FootIkDriverInfoLeg>,
    // Offset: 32
    pub raycast_distance_up: f32,
    // Offset: 36
    pub raycast_distance_down: f32,
    // Offset: 40
    pub original_gound_height_ms: f32,
    // Offset: 44
    pub vertical_offset: f32,
    // Offset: 48
    pub collision_filter_info: u32,
    // Offset: 52
    pub forward_align_fraction: f32,
    // Offset: 56
    pub sideways_align_fraction: f32,
    // Offset: 60
    pub sideways_sample_width: f32,
    // Offset: 64
    pub lock_feet_when_planted: bool,
    // Offset: 65
    pub use_character_up_vector: bool,
    // Offset: 66
    pub is_quadruped_narrow: bool,
    // Offset: 67
    pub keep_source_foot_end_above_ground: bool,
}

#[derive(Clone, Debug, HkStruct)]
pub struct FootIkDriverInfoLeg {
    // Offset: 0
    pub knee_axis_ls: Vec4,
    // Offset: 16
    pub foot_end_ls: Vec4,
    // Offset: 32
    pub foot_planted_ankle_height_ms: f32,
    // Offset: 36
    pub foot_raised_ankle_height_ms: f32,
    // Offset: 40
    pub max_ankle_height_ms: f32,
    // Offset: 44
    pub min_ankle_height_ms: f32,
    // Offset: 48
    pub max_knee_angle_degrees: f32,
    // Offset: 52
    pub min_knee_angle_degrees: f32,
    // Offset: 56
    pub max_ankle_angle_degrees: f32,
    // Offset: 60
    pub hip_index: i16,
    // Offset: 62
    pub hip_sibling_index: i16,
    // Offset: 64
    pub knee_index: i16,
    // Offset: 66
    pub knee_sibling_index: i16,
    // Offset: 68
    pub ankle_index: i16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x3eab47f1)]
pub struct HandIkDriverInfo {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub hands: Array<HandIkDriverInfoHand>,
    // Offset: 32
    pub fade_in_out_curve: i8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x14dfe1dd)]
pub struct HandIkDriverInfoHand {
    // Offset: 0
    pub elbow_axis_ls: Vec4,
    // Offset: 16
    pub back_hand_normal_ls: Vec4,
    // Offset: 32
    pub hand_offset_ls: Vec4,
    // Offset: 48
    pub hand_orientation_offset_ls: Quat,
    // Offset: 64
    pub max_elbow_angle_degrees: f32,
    // Offset: 68
    pub min_elbow_angle_degrees: f32,
    // Offset: 72
    pub shoulder_index: i16,
    // Offset: 74
    pub shoulder_sibling_index: i16,
    // Offset: 76
    pub elbow_index: i16,
    // Offset: 78
    pub elbow_sibling_index: i16,
    // Offset: 80
    pub wrist_index: i16,
    // Offset: 82
    pub enforce_end_position: bool,
    // Offset: 83
    pub enforce_end_rotation: bool,
    // Offset: 88
    pub local_frame_name: String,
}

#[derive(Clone, Debug, HkStruct)]
pub struct BoneWeightArray {
    // Offset: 0
    pub bindable: Bindable,
    // Offset: 48
    pub bone_weights: Array<f32>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct Bindable {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub variable_binding_set: Option<Ptr<VariableBindingSet>>,
    // Offset: 32
    pub cached_bindables: Array<OpaquePtr>,
    // Offset: 40
    pub are_bindables_cached: bool,
    // Offset: 41
    pub has_enable_changed: bool,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xe942f339)]
pub struct VariableBindingSet {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub bindings: Array<VariableBindingSetBinding>,
    // Offset: 32
    pub index_of_binding_to_enable: i32,
    // Offset: 36
    pub has_output_binding: bool,
    // Offset: 37
    pub initialized_offsets: bool,
}

#[derive(Clone, Debug, HkStruct)]
pub struct VariableBindingSetBinding {
    // Offset: 0
    pub member_path: String,
    // Offset: 8
    pub member_class: Option<OpaquePtr>,
    // Offset: 16
    pub offset_in_object_plus_one: i32,
    // Offset: 20
    pub offset_in_array_plus_one: i32,
    // Offset: 24
    pub root_variable_index: i32,
    // Offset: 28
    pub variable_index: i32,
    // Offset: 32
    pub bit_index: i8,
    // Offset: 33
    pub binding_type: BindingType,
    // Offset: 34
    pub member_type: u8,
    // Offset: 35
    pub variable_type: i8,
    // Offset: 36
    pub flags: i8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, HkEnum)]
#[size(i8)]
pub enum BindingType {
    Variable = 0,
    CharacterProperty = 1,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xc40485c7)]
pub struct BreakableConstraintData {
    // Offset: 0
    pub parent: WrappedConstraintData,
    // Offset: 32
    pub threshold: f32,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x5a93639c)]
pub struct WrappedConstraintData {
    // Offset: 0
    pub parent: ConstraintData,
    // Offset: 24
    pub constraint_data: Option<Ptr<ConstraintData>>,
}
