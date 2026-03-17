use game_core::hierarchy::Key;
use game_core_pipeline::material::StandardMaterial;
use hashbrown::HashMap;
use hecs::Entity;
use openfo4_asset::Handle;

use crate::assets::nif::{NifScene, animation::ControlledValue};

#[derive(Clone, Debug, Default)]
pub struct AnimationController {
    pub material: HashMap<Key, MaterialController>,
}

impl AnimationController {
    /// Whether the effects of the controller are visible, otherwise the controller can be skipped.
    ///
    pub fn is_visible(&self) -> bool {
        !self.material.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct MaterialController {
    pub material: StandardMaterial,
    pub values: Vec<ControlledValue>,
}

#[derive(Clone, Debug)]
pub struct AnimationControllerInstance {
    pub(super) scene: Handle<NifScene>,
    pub(super) nodes: HashMap<Key, Entity>,
}

impl AnimationControllerInstance {
    pub fn controller(&self) -> &AnimationController {
        &self.scene.controller
    }

    pub fn nodes(&self) -> &HashMap<Key, Entity> {
        &self.nodes
    }
}
