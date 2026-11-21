use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::Entity;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_esp::bstr::BString;
use openfo4_esp::records::{MovementSpeeds, SubgraphFlags};

use crate::state::actor::character::ActorCharacter;

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

/// Keywords for ObjectReferences.
#[derive(Clone, Debug, Default)]
pub struct ObjectKeywords {
    keywords: HashSet<RecordId>,
    /// Dynamically added keywords.
    added: HashSet<RecordId>,
    /// Dynamically removed keywords.
    removed: HashSet<RecordId>,
}

impl ObjectKeywords {
    pub fn new(keywords: HashSet<RecordId>) -> Self {
        Self {
            keywords,
            added: HashSet::new(),
            removed: HashSet::new(),
        }
    }

    pub fn contains(&self, keyword: RecordId) -> bool {
        self.keywords.contains(&keyword)
    }

    pub fn add(&mut self, keyword: RecordId) {
        if self.keywords.contains(&keyword) {
            return;
        }

        self.keywords.insert(keyword);
        self.added.insert(keyword);
        self.removed.remove(&keyword);
    }

    pub fn remove(&mut self, keyword: RecordId) {
        if !self.keywords.contains(&keyword) {
            return;
        }

        self.keywords.remove(&keyword);
        self.added.remove(&keyword);
        self.removed.insert(keyword);
    }

    /// Resets the dynamically added/removed keyword to the default state.
    pub fn reset(&mut self, keyword: RecordId) {
        // Keyword was either removed or added, but not both.
        debug_assert!(!(self.added.contains(&keyword) && self.removed.contains(&keyword)));

        if self.removed.remove(&keyword) {
            self.keywords.insert(keyword);
        } else if self.added.remove(&keyword) {
            self.keywords.remove(&keyword);
        }
    }
}

/// A body part of an actor.
#[derive(Copy, Clone, Debug)]
pub struct BodyPartBone {
    /// The actor that this body part is attached to.
    pub actor: Entity,
    /// The unique ID of the body part.
    pub id: u32,
}

#[derive(Clone, Debug)]
pub struct ActorBodyParts(pub Vec<ActorBodyPart>);

impl ActorBodyParts {
    pub fn get(&self, id: u32) -> Option<&ActorBodyPart> {
        self.0.get(id as usize)
    }
}

#[derive(Clone, Debug)]
pub struct ActorBodyPart {
    /// The actor value of the body part local health condition.
    pub condition: RecordId,
    /// Factor applied to incoming damage hitting this body part.
    pub damage_factor: f32,
}

#[derive(Clone, Debug)]
pub struct BodyParts(HashMap<Entity, BodyPart>);

#[derive(Clone, Debug)]
pub struct BodyPart {
    pub condition: RecordId,
    pub damage_factor: f32,
}

#[derive(Clone, Debug)]
pub struct RaceSubgraphData {
    pub actor_keywords: Vec<RecordId>,
    pub target_keywords: Vec<RecordId>,
    pub behavior_graph: BString,
    pub animation_paths: Vec<BString>,
    pub flags: SubgraphFlags,
}

#[derive(Clone, Debug)]
pub struct ActorAnimationGraphData {
    pub character: ActorCharacter,
    pub root_path: DataPathBuf,
    pub root_behavior_path: DataPathBuf,
    pub subgraphs: Vec<ActorAnimationGraph>,
}

#[derive(Clone, Debug)]
pub struct ActorAnimationGraph {
    // pub graph: Arc<AnimationGraph>,
    pub behavior: DataPathBuf,
    // pub animation: AnimationSet,
    pub actor_keywords: Vec<RecordId>,
    pub target_keywords: Vec<RecordId>,
    pub flags: SubgraphFlags,
    pub search_paths: Vec<DataPathBuf>,
}

#[derive(Clone, Debug)]
pub struct ActorMovementSpeeds {
    pub default: MovementSpeeds,
    pub sneak: MovementSpeeds,
}
