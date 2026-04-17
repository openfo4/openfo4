use std::any::TypeId;

use hashbrown::HashSet;
use hecs::EntityRef;

use crate::ecs::{
    Component, DynamicBundle, Entity, LifecycleEvents, World, register_type_once_dynamic,
};

pub struct EntityBuilder {
    inner: hecs::EntityBuilder,
    components: HashSet<TypeId>,
}

impl EntityBuilder {
    pub fn new() -> Self {
        Self {
            inner: hecs::EntityBuilder::new(),
            components: HashSet::new(),
        }
    }

    pub fn add<T>(&mut self, component: T)
    where
        T: Component,
    {
        self.components.insert(TypeId::of::<T>());
        self.inner.add(component);
    }

    pub fn add_bundle<T>(&mut self, components: T)
    where
        T: DynamicBundle,
    {
        components.for_each_type_id(|id| {
            self.components.insert(id);
        });

        self.inner.add_bundle(components.into_raw());
    }

    pub fn build(&mut self) -> BuiltEntity<'_> {
        BuiltEntity {
            inner: self.inner.build(),
            entities: &self.components,
        }
    }
}

pub struct BuiltEntity<'a> {
    inner: hecs::BuiltEntity<'a>,
    entities: &'a HashSet<TypeId>,
}

#[expect(
    private_interfaces,
    reason = "The private types are part of the private trait implementation"
)]
impl<'a> DynamicBundle for BuiltEntity<'a> {
    type RawBundle = hecs::BuiltEntity<'a>;

    fn init(&self, world: &mut World) {
        for id in self.entities {
            register_type_once_dynamic(world, *id);
        }
    }

    fn mark_pre_insert(&self, entity: EntityRef<'_>, e: Entity, events: &mut LifecycleEvents) {
        // FIXME: This always iterates over all current components on the entity.
        // There is no `has_dynamic` or similar that allows us to query by `TypeId` directly.
        // We need to rework this somehow to avoid this unnecessary overhead.
        let mut entities = self.entities.clone();
        for id in entity.component_types() {
            if entities.remove(&id) {
                // The caller guarantees that it called `Self::init` before calling
                // this function, which will setup the lifecycle event set for all our components.
                let set = events.changed.get_mut(&id).unwrap();
                set.get_mut().insert(e);
            }
        }

        for id in entities {
            // The caller guarantees that it called `Self::init` before calling
            // this function, which will setup the lifecycle event set for all our components.
            let set = events.added.get_mut(&id).unwrap();
            set.get_mut().insert(e);
        }
    }

    fn for_each_type_id<F>(&self, mut f: F)
    where
        F: FnMut(TypeId),
    {
        for id in self.entities {
            f(*id);
        }
    }

    fn into_raw(self) -> Self::RawBundle {
        self.inner
    }
}
