use glam::Vec4;
use openfo4_hkx_derive::HkStruct;

use crate::{Array, Ptr, ReferencedObject};

#[derive(Clone, Debug, HkStruct)]
pub struct ProjectStringData {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub animation_filenames: Array<String>,
    // Offset: 32
    pub behavior_filenames: Array<String>,
    // Offset: 48
    pub character_filenames: Array<String>,
    // Offset: 64
    pub event_names: Array<String>,
    // Offset: 80
    pub animation_path: String,
    // Offset: 96
    pub behavior_path: String,
    // Offset: 104
    pub scripts_path: String,
    // Offset: 112
    pub full_path_to_source: String,
    // Offset: 120
    pub root_path: String,
}

#[derive(Clone, Debug, HkStruct)]
pub struct ProjectData {
    // Offset: 0
    pub object: ReferencedObject,
    // Offset: 16
    pub world_up_ws: Vec4,
    // Offset: 32
    pub string_data: Ptr<ProjectStringData>,
    // Offset: 40
    pub default_event_mode: i8,
}
