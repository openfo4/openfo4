use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::FusedIterator;
use std::marker::PhantomData;

use crate::ecs::Entity;

#[derive(Debug, Default)]
pub struct EventBus {
    events: HashMap<TypeId, Vec<Box<dyn Any>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    pub fn send<T>(&mut self, event: T)
    where
        T: Event,
    {
        let events = self.events.entry(TypeId::of::<T>()).or_default();
        events.push(Box::new(event));
    }

    pub fn read<T>(&self) -> EventReader<'_, T>
    where
        T: Event,
    {
        let events = self
            .events
            .get(&TypeId::of::<T>())
            .map(|v| v.as_slice())
            .unwrap_or_default();

        EventReader {
            events,
            _marker: PhantomData,
        }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl<T> Extend<T> for EventBus
where
    T: Event,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for event in iter {
            self.send(event);
        }
    }
}

pub trait Event: Send + Sync + 'static {}

impl<T> Event for T where T: Send + Sync + 'static {}

pub struct EventReader<'a, T> {
    events: &'a [Box<dyn Any>],
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for EventReader<'a, T>
where
    T: Event,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.events
            .split_off_first()
            .map(|v| v.downcast_ref().unwrap())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.events.len(), Some(self.events.len()))
    }
}

impl<'a, T> ExactSizeIterator for EventReader<'a, T>
where
    T: Event,
{
    fn len(&self) -> usize {
        self.events.len()
    }
}

impl<'a, T> FusedIterator for EventReader<'a, T> where T: Event {}

pub struct ComponentChanged<T> {
    entity: Entity,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ComponentChanged<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> Entity {
        self.entity
    }
}

impl<T> Clone for ComponentChanged<T> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for ComponentChanged<T> {}

impl<T> std::fmt::Debug for ComponentChanged<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentChanged")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

impl<T> PartialEq for ComponentChanged<T> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl<T> Eq for ComponentChanged<T> {}

impl<T> Hash for ComponentChanged<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.entity.hash(state);
    }
}
