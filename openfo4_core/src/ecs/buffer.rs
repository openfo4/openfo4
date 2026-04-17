use hecs::Entity;

use crate::ecs::{Bundle, Component, DynamicBundle, World};

/// A buffer to record changes to a [`World`].
pub struct CommandBuffer {
    events: Vec<Event>,
}

impl CommandBuffer {
    /// Creates a new, empty `CommandBuffer`.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Spawns a new entity with the given components.
    pub fn spawn<T>(&mut self, components: T)
    where
        T: DynamicBundle,
    {
        self.insert_or_spawn(None, components);
    }

    /// Inserts components into the given [`Entity`], if it exists.
    ///
    /// If the [`Entity`] does not exist when the `CommandBuffer` is executed, the operation is
    /// ignored.
    pub fn insert<T>(&mut self, entity: Entity, components: T)
    where
        T: DynamicBundle,
    {
        self.insert_or_spawn(Some(entity), components);
    }

    /// Inserts a single component into the given [`Entity`].
    pub fn insert_one<T>(&mut self, entity: Entity, component: T)
    where
        T: Component,
    {
        self.insert(entity, (component,));
    }

    fn insert_or_spawn<T>(&mut self, entity: Option<Entity>, components: T)
    where
        T: DynamicBundle,
    {
        fn insert_deferred<T>(world: &mut World, entity: Option<Entity>, components: *mut ())
        where
            T: DynamicBundle,
        {
            let boxed: Box<T> = unsafe { Box::from_raw(components.cast::<T>()) };
            let components: T = *boxed;

            match entity {
                Some(entity) => {
                    world.insert(entity, components).ok();
                }
                None => {
                    world.spawn(components);
                }
            }
        }

        let boxed = Box::into_raw(Box::new(components));

        self.events.push(Event::Insert {
            func: insert_deferred::<T>,
            entity,
            components: boxed.cast::<()>(),
        });
    }

    pub fn remove<T>(&mut self, entity: Entity)
    where
        T: Bundle + 'static,
    {
        fn remove_deferred<T>(world: &mut World, entity: Entity)
        where
            T: Bundle + 'static,
        {
            world.remove::<T>(entity).ok();
        }

        self.events.push(Event::Remove {
            func: remove_deferred::<T>,
            entity,
        });
    }

    pub fn remove_one<T>(&mut self, entity: Entity)
    where
        T: Component,
    {
        self.remove::<(T,)>(entity);
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.events.push(Event::Despawn(entity))
    }

    pub fn run_on(&mut self, world: &mut World) {
        for event in self.events.drain(..) {
            match event {
                Event::Insert {
                    func,
                    entity,
                    components,
                } => {
                    func(world, entity, components);
                }
                Event::Remove { func, entity } => {
                    func(world, entity);
                }
                Event::Despawn(entity) => {
                    world.despawn(entity).ok();
                }
            }
        }
    }
}

enum Event {
    Insert {
        func: fn(world: &mut World, entity: Option<Entity>, components: *mut ()),
        entity: Option<Entity>,
        components: *mut (),
    },
    Remove {
        func: fn(world: &mut World, entity: Entity),
        entity: Entity,
    },
    Despawn(Entity),
}
