use std::any::{Any, TypeId};
use std::iter::FusedIterator;
use std::marker::PhantomData;

use hashbrown::HashMap;

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

    pub fn move_to(&mut self, dst: &mut EventBus) {
        for (id, events) in self.events.drain() {
            dst.events.entry(id).or_default().extend(events);
        }
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
