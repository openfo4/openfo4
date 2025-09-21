use glam::{Quat, Vec4};
use half::f16;
use openfo4_hkx_derive::{HkEnum, HkStruct};

use crate::{Array, OpaquePtr, Ptr, ReferencedObject};

#[derive(Clone, Debug, HkStruct)]
pub struct RefCountedProperties {
    // Offset: 0
    entries: Array<RefCountedPropertiesEntry>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct RefCountedPropertiesEntry {
    // Offset: 0
    object: Ptr<ReferencedObject>,
    // Offset: 8
    key: u16,
    // Offset: 10
    flags: u16,
}

#[derive(Clone, Debug, HkStruct)]
pub struct PhysicsSystemData {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub materials: Array<Material>,
    // Offset: 32
    pub motion_properties: Array<MotionProperties>,
    // Offset: 48
    pub motion_cinfos: Array<MotionCinfo>,
    // Offset: 64
    pub body_cinfos: Array<BodyCinfo>,
    // Offset: 80
    pub constraint_cinfos: Array<ConstraintCinfo>,
    // Offset: 96
    pub referenced_objects: Array<ReferencedObject>,
    // Offset: 112
    pub name: String,
}

#[derive(Clone, Debug, HkStruct)]
pub struct Material {
    // Offset: 0
    pub name: String,
    // Offset: 8
    pub is_exclusive: u32,
    // Offset: 12
    pub flags: u32,
    // Offset: 16
    pub trigger_type: u8,
    // Offset: 17
    pub trigger_manifold_tolerance: u8, // f8
    // Offset: 18
    pub dynamic_friction: f16,
    // Offset: 20
    pub static_friction: f16,
    // Offset: 22
    pub restitution: f16,
    // Offset: 24
    pub friction_combine_policy: CombinePolicy,
    // Offset: 25
    pub restitution_combine_policy: CombinePolicy,
    // Offset: 26
    pub welding_tolerance: i16,
    // Offset: 28
    pub max_contact_impulse: f32,
    // Offset: 32
    pub fraction_of_clipped_impulse_to_apply: f32,
    // Offset: 36
    pub mass_changer_category: u8,
    // Offset: 37
    _pad0: u8,
    // Offset: 38
    pub mass_changer_heavy_object_factor: f16,
    // Offset: 40
    pub soft_contact_force_factor: i16,
    // Offset: 42
    pub soft_contact_damp_factor: i16,
    // Offset: 44
    pub soft_contact_separation_velocity: u8, // f8
    _pad1: [u8; 3],
    // Offset: 48
    pub surface_velocity: Option<Ptr<SurfaceVelocity>>,
    // Offset: 56
    pub disabling_collisions_between_cvx_cvx_dynamic_objects_distance: i16, // f16
    _pad2: [u16; 3],
    // Offset: 64
    pub user_data: u64,
    // Offset: 72
    pub is_shared: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, HkEnum)]
#[size(u8)]
pub enum CombinePolicy {
    Average = 0,
    Min = 1,
    Max = 2,
}

#[derive(Clone, Debug, HkStruct)]
pub struct SurfaceVelocity {
    pub object: ReferencedObject,
}

#[derive(Clone, Debug, HkStruct)]
pub struct MotionProperties {
    // Offset: 0
    is_exclusive: u32,
    // Offset: 4
    flags: u32,
    // Offset: 8
    gravity_factor: f32,
    // Offset: 12
    time_factor: f32,
    // Offset: 16
    max_linear_speed: f32,
    // Offset: 20
    max_angular_speed: f32,
    // Offset: 24
    linear_damping: f32,
    // Offset: 28
    angular_damping: f32,
    // Offset: 32
    solver_stabilization_speed_threshold: f32,
    // Offset: 36
    solver_stabilization_speed_reduction: f32,
    // Offset: 40
    max_dist_squared: f32,
    // Offset: 44
    max_rot_squared: f32,
    // Offset: 48
    inv_block_size: f32,
    // Offset: 52
    pathing_upper_threshold: i16,
    // Offset: 54
    pathing_lower_threshold: i16,
    // Offset: 56
    num_deactivation_frequency_passes: u8,
    // Offset: 57
    deactivation_velocity_scale_square: u8,
    // Offset: 58
    minimum_pathing_velocity_scale_square: u8,
    // Offset: 59
    spiking_velocity_scale_threshold_squared: u8,
    // Offset: 60
    minimum_spiking_velocity_scale_squared: u8,
}

#[derive(Clone, Debug, HkStruct)]
pub struct MotionCinfo {
    // Offset: 0
    pub motion_properties_id: u16,
    // Offset: 2
    pub enable_deactivation: bool,
    // Offset: 4
    pub inverse_mass: f32,
    // Offset: 8
    pub mass_factor: f32,
    // Offset: 12
    pub max_linear_acceleration_distance_per_step: f32,
    // Offset: 16
    pub max_rotation_to_prevent_tunneling: f32,
    _pad0: [u32; 3],
    // Offset: 32
    pub inverse_inertia_local: Vec4,
    // Offset: 48
    pub center_of_mass_world: Vec4,
    // Offset: 64
    pub orientation: Quat,
    // Offset: 80
    pub linear_velocity: Vec4,
    // Offset: 96
    pub angular_velocity: Vec4,
}

#[derive(Clone, Debug, HkStruct)]
pub struct BodyCinfo {
    /// Pointer to the collision shape of the body.
    // Offset: 0
    pub shape: OpaquePtr,
    // Offset: 8
    pub reserved_body_id: u32,
    // Offset: 12
    pub motion_id: u32,
    // Offset: 16
    pub quality_id: u8,
    // Offset: 17
    _pad0: u8,
    // Offset: 18
    pub material_id: u16,
    // Offset: 20
    pub collision_filter_info: u32,
    // Offset: 24
    pub flags: i32,
    // Offset: 28
    pub collision_look_ahead_distance: f32,
    // Offset: 32
    pub name: String,
    // Offset: 40
    pub user_data: u64,
    // Offset: 48
    pub position: Vec4,
    // Offset: 60
    pub orientation: Quat,
    // Offset: 80
    pub spu_flags: u8,
    // Offset: 88
    pub local_frame: Option<Ptr<HkLocalFrame>>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct Shape {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    flags: u16,
    // Offset: 18
    num_shape_key_bits: u8,
    // Offset: 19
    dispatch_type: u8,
    // Offset: 20
    pub convex_radius: f32,
    // Offset: 24
    user_data: u64,
    // Offset: 32
    properties: Option<Ptr<RefCountedProperties>>,
}

#[derive(Clone, Debug, HkStruct)]
pub struct HkLocalFrame {
    // Offset: 0
    pub object: ReferencedObject,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConstraintCinfo {
    // Offset: 0
    pub data: Ptr<ConstaintData>,
    // Offset: 8
    pub body_a: u32,
    // Offset: 12
    pub body_b: u32,
    // Offset: 16
    pub flags: u8,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ConstaintData {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub user_data: u64,
}

#[derive(Clone, Debug, HkStruct)]
pub struct PhysicsSceneData {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub system_datas: Array<Ptr<PhysicsSystemData>>,
    // Offset: 32
    pub world_cinfo: Option<Ptr<RefWorldCinfo>>,
}

/// TODO: Not yet implemented.
#[derive(Clone, Debug, HkStruct)]
pub struct RefWorldCinfo {}
