use std::time::Duration;

use game_core::hierarchy::Key;
use game_core_pipeline::material::StandardMaterial;
use hashbrown::HashMap;
use openfo4_asset::Handle;
use openfo4_core::ecs::Entity;

use crate::assets::nif::animation::TransformInterpolator;
use crate::assets::nif::{NifScene, animation::ControlledValue};

#[derive(Clone, Debug, Default)]
pub struct AnimationController {
    pub material: HashMap<Key, MaterialController>,
    /// Named animation sequences.
    pub sequences: HashMap<String, Sequence>,
}

impl AnimationController {
    /// Whether the effects of the controller are visible, otherwise the controller can be skipped.
    ///
    pub fn is_visible(&self) -> bool {
        !self.material.is_empty() || !self.sequences.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct MaterialController {
    pub material: StandardMaterial,
    pub values: Vec<ControlledValue>,
}

#[derive(Clone, Debug)]
pub struct Sequence {
    pub transforms: HashMap<Key, TransformInterpolator>,
    /// The time after which the sequence is considered finished.
    pub duration: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct AnimationControllerInstance {
    pub(super) scene: Handle<NifScene>,
    pub(super) nodes: HashMap<Key, Entity>,
    pub sequences: HashMap<String, SequencePlaybackState>,
}

impl AnimationControllerInstance {
    pub fn scene(&self) -> &NifScene {
        &self.scene
    }

    pub fn controller(&self) -> &AnimationController {
        &self.scene.controller
    }

    pub fn nodes(&self) -> &HashMap<Key, Entity> {
        &self.nodes
    }
}

#[derive(Clone, Debug)]
pub struct SequencePlaybackState {
    pub time: Duration,
}
