mod buffer;
mod builder;
mod query;

use std::any::TypeId;
use std::iter::FusedIterator;
use std::ops::{Deref, DerefMut};

use hashbrown::HashSet;
use hecs::{ComponentError, NoSuchEntity, TypeIdMap};
use parking_lot::RwLock;
use parking_lot::lock_api::RawRwLock;

pub type Entity = hecs::Entity;

pub use buffer::CommandBuffer;
pub use builder::{BuiltEntity, EntityBuilder};
pub use query::{Query, QueryBorrow, QueryIter, QueryMut, Satisfies, ViewBorrow};

/// Storage for entities and components.
pub struct World {
    world: hecs::World,
    lifecycle: LifecycleEvents,
}

impl World {
    /// Creates a new, empty `World`.
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            lifecycle: LifecycleEvents::default(),
        }
    }

    pub fn reserve_entity(&self) -> Entity {
        self.world.reserve_entity()
    }

    pub unsafe fn find_entity_from_id(&self, id: u32) -> Entity {
        unsafe { self.world.find_entity_from_id(id) }
    }

    pub fn spawn_one(&mut self, component: impl hecs::Component) -> Entity {
        self.world.spawn((component,))
    }

    pub fn spawn(&mut self, components: impl DynamicBundle) -> Entity {
        let entity = self.world.reserve_entity();

        self.insert_unchecked(entity, components);
        entity
    }

    /// Inserts a new set of components into the given [`Entity`].
    ///
    /// # Errors
    ///
    /// Returns an error if the given [`Entity`] does not exist in the current `World`.
    pub fn insert<T>(&mut self, entity: Entity, components: T) -> Result<(), NoSuchEntity>
    where
        T: DynamicBundle,
    {
        if !self.world.contains(entity) {
            return Err(NoSuchEntity);
        }

        self.insert_unchecked(entity, components);
        Ok(())
    }

    /// Inserts a single component into the given [`Entity`].
    ///
    /// # Errors
    ///
    /// Returns an error if the given [`Entity`] does not exist in the current `World`.
    pub fn insert_one<T>(&mut self, entity: Entity, component: T) -> Result<(), NoSuchEntity>
    where
        T: Component,
    {
        self.insert(entity, (component,))
    }

    /// Inserts a new set of components, assuming that the given [`Entity`] exists in this `World`.
    fn insert_unchecked<T>(&mut self, entity: Entity, components: T)
    where
        T: DynamicBundle,
    {
        components.init(self);

        self.world.flush();

        let e = self.world.entity(entity).unwrap();
        components.mark_pre_insert(e, entity, &mut self.lifecycle);

        self.world.insert(entity, components.into_raw()).unwrap();
    }

    pub fn remove<T>(&mut self, entity: Entity) -> Result<T, ComponentError>
    where
        T: Bundle + 'static,
    {
        let components = self.world.remove::<T>(entity)?;
        T::mark_post_removal(entity, &mut self.lifecycle);
        Ok(components)
    }

    pub fn remove_one<T>(&mut self, entity: Entity) -> Result<T, ComponentError>
    where
        T: Component,
    {
        self.remove::<(T,)>(entity).map(|(c,)| c)
    }

    /// Destroys the entity with all its components.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), NoSuchEntity> {
        let e = self.world.entity(entity)?;

        for id in e.component_types() {
            let map = self.lifecycle.removed.get_mut(&id).unwrap();
            map.get_mut().insert(entity);
        }

        self.world.despawn(entity).unwrap();
        Ok(())
    }

    /// Returns a reference to given entity.
    ///
    /// # Errors
    ///
    /// Returns an error if the given [`Entity`] does not exist in the current world.
    pub fn entity(&self, entity: Entity) -> Result<EntityRef<'_>, NoSuchEntity> {
        let inner = self.world.entity(entity)?;
        Ok(EntityRef { world: self, inner })
    }

    /// Borrows the component of a single [`Entity`].
    ///
    /// # Errors
    ///
    /// Returns an error if the entity does not exist, or the entity does not have the requested
    /// component.
    pub fn get<'a, T>(
        &'a self,
        entity: Entity,
    ) -> Result<<T as ComponentRef<'a>>::Ref, ComponentError>
    where
        T: ComponentRef<'a>,
    {
        let r = self.world.get::<'a, T>(entity)?;
        Ok(<T as ComponentRef<'a>>::from_raw(r, self, entity))
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.world.contains(entity)
    }

    /// Returns all entities that got a new component added.
    ///
    /// This yields all entities once that that got the given component `T` added since the last
    /// call to [`clear_trackers`].
    ///
    /// [`clear_trackers`]: Self::clear_trackers
    pub fn component_added<T>(&self) -> ComponentLifecycleIter<'_>
    where
        T: Component,
    {
        let id = TypeId::of::<T>();
        match self.lifecycle.added.get(&id) {
            Some(map) => ComponentLifecycleIter::new(map),
            None => ComponentLifecycleIter {
                iter: None,
                lock: None,
            },
        }
    }

    /// Returns all entities that have an updated component.
    ///
    /// This yields all entities once that got the given component `T` updated since the last call
    /// to [`clear_trackers`].
    ///
    /// [`clear_trackers`]: Self::clear_trackers
    pub fn component_changed<T>(&self) -> ComponentLifecycleIter<'_>
    where
        T: Component,
    {
        let id = TypeId::of::<T>();
        match self.lifecycle.changed.get(&id) {
            Some(map) => ComponentLifecycleIter::new(map),
            None => ComponentLifecycleIter {
                iter: None,
                lock: None,
            },
        }
    }

    /// Returns all entities that had a component removed.
    ///
    /// This yields all entities once that got the given component `T` removed since the last call
    /// to [`clear_trackers`]. Note that this includes components that were removed because the
    /// entity was despawned.
    ///
    /// [`clear_trackers`]: Self::clear_trackers
    pub fn component_removed<T>(&self) -> ComponentLifecycleIter<'_>
    where
        T: Component,
    {
        let id = TypeId::of::<T>();
        match self.lifecycle.removed.get(&id) {
            Some(map) => ComponentLifecycleIter::new(map),
            None => ComponentLifecycleIter {
                iter: None,
                lock: None,
            },
        }
    }

    /// Clears all lifecycle events.
    pub fn clear_trackers(&mut self) {
        self.lifecycle
            .added
            .values_mut()
            .for_each(|v| v.get_mut().clear());
        self.lifecycle
            .changed
            .values_mut()
            .for_each(|v| v.get_mut().clear());
        self.lifecycle
            .removed
            .values_mut()
            .for_each(|v| v.get_mut().clear());
    }

    pub fn query<Q>(&self) -> QueryBorrow<'_, Q>
    where
        Q: Query,
    {
        QueryBorrow {
            inner: self.world.query::<Q::RawQuery>(),
            events: &self.lifecycle,
        }
    }

    pub fn query_mut<Q>(&mut self) -> QueryMut<'_, Q>
    where
        Q: Query,
    {
        QueryMut {
            inner: self.world.query_mut::<Q::RawQuery>(),
            events: &self.lifecycle,
        }
    }

    pub fn view<Q>(&self) -> ViewBorrow<'_, Q>
    where
        Q: Query,
    {
        ViewBorrow {
            inner: self.world.view::<Q::RawQuery>(),
            events: &self.lifecycle,
        }
    }

    pub fn view_mut<Q>(&mut self) -> ViewBorrow<'_, Q>
    where
        Q: Query,
    {
        // FIXME: We can call self.world.view_mut here.
        self.view()
    }
}

#[derive(Debug, Default)]
struct LifecycleEvents {
    added: TypeIdMap<RwLock<HashSet<Entity>>>,
    changed: TypeIdMap<RwLock<HashSet<Entity>>>,
    removed: TypeIdMap<RwLock<HashSet<Entity>>>,
}

pub struct ComponentLifecycleIter<'a> {
    iter: Option<hashbrown::hash_set::Iter<'a, Entity>>,
    lock: Option<&'a RwLock<HashSet<Entity>>>,
}

impl<'a> ComponentLifecycleIter<'a> {
    fn new(map: &'a RwLock<HashSet<Entity>>) -> Self {
        // We cannot store a lock guard here since the iterator
        // is owned, which would create a self-referential struct,
        // so we will do our own locking.

        // SAFETY: We don't unlock anything here (which could break the
        // `RwLock` invariants).
        if unsafe { !map.raw().try_lock_shared() } {
            panic!(
                "cannot access lifecycle events for component while a mutable reference to that component exists"
            );
        }

        // SAFETY: We have acquired a shared lock here and only
        // access the locked data immutably.
        let data = unsafe { &*map.data_ptr() };
        let iter = data.iter();

        Self {
            iter: Some(iter),
            lock: Some(map),
        }
    }
}

impl<'a> Iterator for ComponentLifecycleIter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let iter = self.iter.as_mut()?;
        iter.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for ComponentLifecycleIter<'a> {
    fn len(&self) -> usize {
        self.iter
            .as_ref()
            .map(|iter| iter.len())
            .unwrap_or_default()
    }
}

impl<'a> FusedIterator for ComponentLifecycleIter<'a> {}

impl<'a> Drop for ComponentLifecycleIter<'a> {
    fn drop(&mut self) {
        // Drop the iterator before unlocking the lock.
        self.iter = None;

        if let Some(lock) = self.lock.take() {
            // SAFETY: We have acquired a single shared lock when
            // we constructed this type.
            // At this current point we have dropped all data depending
            // on the locked data.
            unsafe {
                lock.raw().unlock_shared();
            }
        }
    }
}

/// Handle to a an entity inside a [`World`].
pub struct EntityRef<'a> {
    inner: hecs::EntityRef<'a>,
    world: &'a World,
}

impl<'a> EntityRef<'a> {
    /// Borrows a single component from the entity, if it exists.
    pub fn get<T>(&'a self) -> Option<<T as ComponentRef<'a>>::Ref>
    where
        T: ComponentRef<'a>,
    {
        let raw = self.inner.get::<T>()?;
        Some(<T as ComponentRef<'a>>::from_raw(
            raw,
            self.world,
            self.inner.entity(),
        ))
    }

    /// Returns `true` if the entity has the given [`Component`].
    pub fn has<T>(&self) -> bool
    where
        T: Component,
    {
        self.inner.has::<T>()
    }
}

/// A shared reference to the component of an entity.
pub struct Ref<'a, T> {
    r: hecs::Ref<'a, T>,
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.r
    }
}

/// A unique reference to the component of an entity.
pub struct RefMut<'a, T>
where
    T: 'static,
{
    r: hecs::RefMut<'a, T>,
    world: &'a World,
    entity: Entity,
}

impl<'a, T> Deref for RefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.r
    }
}

impl<'a, T> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.r
    }
}

impl<'a, T> Drop for RefMut<'a, T>
where
    T: 'static,
{
    fn drop(&mut self) {
        let id = TypeId::of::<T>();
        let map = self.world.lifecycle.changed.get(&id).unwrap();
        map.try_write().expect("Cannot create mutable reference to component while lifecycle event iterator exists").insert(self.entity);
    }
}

pub trait ComponentRef<'a>: hecs::ComponentRef<'a> {
    type Ref;

    fn from_raw(
        r: <Self as hecs::ComponentRef<'a>>::Ref,
        world: &'a World,
        entity: Entity,
    ) -> <Self as ComponentRef<'a>>::Ref;
}

impl<'a, T> ComponentRef<'a> for &'a T
where
    T: Send + Sync + 'static,
{
    type Ref = Ref<'a, T>;

    fn from_raw(
        r: <Self as hecs::ComponentRef<'a>>::Ref,
        _: &'a World,
        _: Entity,
    ) -> <Self as ComponentRef<'a>>::Ref {
        Ref { r }
    }
}

impl<'a, T> ComponentRef<'a> for &'a mut T
where
    T: Send + Sync + 'static,
{
    type Ref = RefMut<'a, T>;

    fn from_raw(
        r: <Self as hecs::ComponentRef<'a>>::Ref,
        world: &'a World,
        entity: Entity,
    ) -> <Self as ComponentRef<'a>>::Ref {
        RefMut { r, world, entity }
    }
}

#[expect(
    private_interfaces,
    reason = "The private types are part of the private trait implementation"
)]
pub trait DynamicBundle {
    #[doc(hidden)]
    type RawBundle: hecs::DynamicBundle;

    #[doc(hidden)]
    fn init(&self, world: &mut World);

    #[doc(hidden)]
    fn mark_pre_insert(&self, entity: hecs::EntityRef<'_>, e: Entity, events: &mut LifecycleEvents);

    #[doc(hidden)]
    fn for_each_type_id<F>(&self, f: F)
    where
        F: FnMut(TypeId);

    #[doc(hidden)]
    fn into_raw(self) -> Self::RawBundle;
}

#[expect(
    private_interfaces,
    reason = "The private types are part of the private trait implementation"
)]
pub trait Bundle: hecs::Bundle {
    #[doc(hidden)]
    fn mark_post_removal(entity: Entity, events: &mut LifecycleEvents);
}

macro_rules! impl_bundle_tuple {
    ($($t:ident),*) => {
        #[expect(
            private_interfaces,
            reason = "The private types are part of the private trait implementation"
        )]
        impl<$($t),*> DynamicBundle for ($($t),*,)
        where
            $(
                $t: Component,
            )*
        {
            type RawBundle = Self;

            fn init(&self, world: &mut World) {
                $(
                    register_type_once::<$t>(world);
                )*
            }

            fn mark_pre_insert(&self, entity: hecs::EntityRef<'_>, e: Entity, events: &mut LifecycleEvents) {
                $(
                    {
                        let map = if entity.has::<$t>() { &mut events.changed } else { &mut events.added };
                        // The caller guarantees that it called `Self::init` before calling
                        // this function, which will setup the lifecycle event set for all our components.
                        let set = map.get_mut(&TypeId::of::<$t>()).unwrap();
                        set.get_mut().insert(e);
                    }
                )*
            }

            fn for_each_type_id<F>(&self, mut f: F)
            where
                F: FnMut(TypeId),
            {
                $(
                    f(TypeId::of::<$t>());
                )*
            }

            fn into_raw(self) -> Self {
                self
            }
        }

        #[expect(
            private_interfaces,
            reason = "The private types are part of the private trait implementation"
        )]
        impl<$($t),*> Bundle for ($($t),*,)
        where
            $(
                $t: Component,
            )*
        {
            fn mark_post_removal(entity: Entity, events: &mut LifecycleEvents) {
                $(
                    {
                        // The caller guarantees that it called `Self::init` before calling
                        // this function, which will setup the lifecycle event set for all our components.
                        let set = events.removed.get_mut(&TypeId::of::<$t>()).unwrap();
                        set.get_mut().insert(entity);
                    }
                )*
            }
        }
    };
}

impl_bundle_tuple!(T0);
impl_bundle_tuple!(T0, T1);
impl_bundle_tuple!(T0, T1, T2);
impl_bundle_tuple!(T0, T1, T2, T3);
impl_bundle_tuple!(T0, T1, T2, T3, T4);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_bundle_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);

fn register_type_once<T>(world: &mut World)
where
    T: Send + Sync + 'static,
{
    let id = TypeId::of::<T>();
    register_type_once_dynamic(world, id);
}

fn register_type_once_dynamic(world: &mut World, id: TypeId) {
    world.lifecycle.added.entry(id).or_default();
    world.lifecycle.changed.entry(id).or_default();
    world.lifecycle.removed.entry(id).or_default();
}

pub trait Component: hecs::Component {}

impl<T> Component for T where T: hecs::Component {}

#[cfg(test)]
mod tests {
    use hashbrown::HashSet;

    use crate::ecs::World;

    struct A;
    struct B;
    struct C;

    #[test]
    fn world_lifecycle_spawn() {
        let mut world = World::new();

        let e0 = world.spawn((A, B));
        let e1 = world.spawn((B, C));

        assert_eq!(
            world.component_added::<A>().collect::<HashSet<_>>(),
            [e0].into_iter().collect()
        );
        assert_eq!(
            world.component_added::<B>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
        assert_eq!(
            world.component_added::<C>().collect::<HashSet<_>>(),
            [e1].into_iter().collect()
        );

        world.clear_trackers();

        assert_eq!(world.component_added::<A>().len(), 0);
        assert_eq!(world.component_added::<B>().len(), 0);
        assert_eq!(world.component_added::<C>().len(), 0);
    }

    #[test]
    fn world_lifecycle_insert_new() {
        let mut world = World::new();

        let e0 = world.spawn((A, B));
        let e1 = world.spawn((A, B));

        world.clear_trackers();

        world.insert_one(e0, C).unwrap();
        world.insert_one(e1, C).unwrap();

        assert_eq!(
            world.component_added::<C>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
    }

    #[test]
    fn world_lifecycle_insert_update() {
        let mut world = World::new();

        let e0 = world.spawn((A, B));
        let e1 = world.spawn((A, B));

        world.clear_trackers();

        world.insert_one(e0, A).unwrap();
        world.insert_one(e1, A).unwrap();

        assert_eq!(world.component_added::<A>().len(), 0);
        assert_eq!(
            world.component_changed::<A>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
    }

    #[test]
    fn world_lifecycle_remove() {
        let mut world = World::new();

        let e0 = world.spawn((A, B));
        let e1 = world.spawn((A, B, C));

        world.clear_trackers();

        world.remove::<(A, B)>(e0).unwrap();
        world.remove::<(A, B)>(e1).unwrap();

        assert_eq!(
            world.component_removed::<A>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
        assert_eq!(
            world.component_removed::<B>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
    }

    #[test]
    fn world_lifecycle_despawn() {
        let mut world = World::new();

        let e0 = world.spawn((A, B));
        let e1 = world.spawn((A, B, C));

        world.clear_trackers();

        world.despawn(e0).unwrap();
        world.despawn(e1).unwrap();

        assert_eq!(
            world.component_removed::<A>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
        assert_eq!(
            world.component_removed::<B>().collect::<HashSet<_>>(),
            [e0, e1].into_iter().collect()
        );
        assert_eq!(
            world.component_removed::<C>().collect::<HashSet<_>>(),
            [e1].into_iter().collect()
        );
    }
}
