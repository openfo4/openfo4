use std::borrow::Cow;

use openfo4_core::ecs::Entity;

#[derive(Clone, Debug)]
pub struct StartNifAnimationSequence {
    pub entity: Entity,
    pub name: Cow<'static, str>,
}
