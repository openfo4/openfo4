use glam::{Mat4, Vec4};
use openfo4_hkx_derive::{HkStruct, signature};

use crate::behavior::{VariableInfo, VariableValueSet};
use crate::skeleton::{FootIkDriverInfo, HandIkDriverInfo, MirroredSkeletionInfo};
use crate::{Array, Ptr, ReferencedObject};

#[derive(Clone, Debug, HkStruct)]
#[signature(0xfec46c1f)]
pub struct CharacterData {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub character_controller_setup: CharacterControllerSetup,
    // Offset: 64
    pub model_up_ms: Vec4,
    // Offset: 80
    pub model_forward_ms: Vec4,
    // Offset: 96
    pub model_right_ms: Vec4,
    // Offset: 112
    pub character_property_infos: Array<VariableInfo>,
    // Offset: 128
    pub num_bones_per_lod: Array<i32>,
    // Offset: 144
    pub character_property_values: Option<Ptr<VariableValueSet>>,
    // Offset: 152
    pub foot_ik_driver_info: Option<Ptr<FootIkDriverInfo>>,
    // Offset: 160
    pub hand_ik_driver_info: Option<Ptr<HandIkDriverInfo>>,
    // Offset: 168
    pub ai_control_driver_info: Option<Ptr<ReferencedObject>>,
    // Offset: 176
    pub string_data: Ptr<CharacterStringData>,
    // Offset: 184
    pub mirrored_skeleton_info: Option<Ptr<MirroredSkeletionInfo>>,
    // Offset: 192
    pub bone_attachment_bone_indices: Array<i16>,
    // Offset: 208
    pub bone_attachment_transforms: Array<Mat4>,
    // Offset: 224
    pub scale: f32,
    // Offset: 228
    pub num_hands: i16,
    // Offset: 230
    pub num_float_slots: i16,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xaf5f7339)]
pub struct CharacterControllerSetup {
    // Offset: 0
    pub rigid_body_setup: RigidBodySetup,
    // Offset: 32
    pub controller_cinfo: Option<Ptr<ReferencedObject>>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x3b082f95)]
pub struct RigidBodySetup {
    // Offset: 0
    pub collision_filter_info: u32,
    // Offset: 4
    pub type_: i8,
    // Offset: 8
    pub shape_setup: ShapeSetup,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0xd7ff86be)]
pub struct ShapeSetup {
    // Offset: 0
    pub capsule_height: f32,
    // Offset: 4
    pub capsule_radius: f32,
    // Offset: 8
    pub file_name: String,
    // Offset: 16
    pub type_: i8,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x0b9d8a52)]
pub struct CharacterStringData {
    // Offset: 0
    pub parent: ReferencedObject,
    // Offset: 16
    pub skin_names: Array<CharacterStringDataFileNameMeshNamePair>,
    // Offset: 32
    pub bone_attachment_names: Array<CharacterStringDataFileNameMeshNamePair>,
    // Offset: 48
    pub animation_bundle_name_data: Array<AssetBundleStringData>,
    // Offset: 64
    pub animation_bundle_filename_data: Array<AssetBundleStringData>,
    // Offset: 80
    pub character_property_names: Array<String>,
    // Offset: 96
    pub retargeting_skeleton_mapper_filenames: Array<String>,
    // Offset: 112
    pub lod_names: Array<String>,
    // Offset: 128
    pub mirrored_sync_point_substrings_a: Array<String>,
    // Offset: 144
    pub mirrored_sync_point_substrings_b: Array<String>,
    // Offset: 160
    pub name: String,
    // Offset: 168
    pub rig_name: String,
    // Offset: 176
    pub ragdoll_name: String,
    // Offset: 184
    pub behavior_filename: String,
    // Offset: 192
    pub lua_script_on_character_activated: String,
    // Offset: 200
    pub lua_script_on_character_deactivated: String,
    // Offset: 208
    pub lua_files: Array<String>,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x026c3a96)]
pub struct CharacterStringDataFileNameMeshNamePair {
    // Offset: 0
    pub file_name: String,
    // Offset: 8
    pub mesh_name: String,
}

#[derive(Clone, Debug, HkStruct)]
#[signature(0x46132bad)]
pub struct AssetBundleStringData {
    // Offset: 0
    pub bundle_name: String,
    // Offset: 8
    pub asset_names: Array<String>,
}
