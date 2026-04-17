use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use hecs::Entity;

pub use hecs::Satisfies;

use crate::ecs::{Component, LifecycleEvents};

#[expect(
    private_interfaces,
    reason = "The private types are part of the private trait implementation"
)]
pub trait Query {
    type Item<'a>;

    #[doc(hidden)]
    type RawQuery: hecs::Query;

    #[doc(hidden)]
    fn from_raw<'a>(
        raw: <Self::RawQuery as hecs::Query>::Item<'a>,
        events: &'a LifecycleEvents,
    ) -> Self::Item<'a>;
}

macro_rules! impl_query_tuple {
    ($($t:ident),*) => {
        #[expect(
            private_interfaces,
            reason = "The private types are part of the private trait implementation"
        )]
        impl<$($t),*> Query for ($($t),*,)
        where
            $(
                $t: Query,
            )*
        {
            type Item<'a> = (
                $(
                    <$t as Query>::Item<'a>,
                )*
            );

            type RawQuery = (
                $(
                    <$t as Query>::RawQuery,
                )*
            );

            #[expect(non_snake_case)]
            fn from_raw<'a>(( $($t),*, ): <Self::RawQuery as hecs::Query>::Item<'a>, events: &'a LifecycleEvents) -> Self::Item<'a> {
                (

                    $(
                        <$t as Query>::from_raw($t, events),
                    )*
                )
            }
        }
    };
}
impl_query_tuple!(T0);
impl_query_tuple!(T0, T1);
impl_query_tuple!(T0, T1, T2);
impl_query_tuple!(T0, T1, T2, T3);
impl_query_tuple!(T0, T1, T2, T3, T4);
impl_query_tuple!(T0, T1, T2, T3, T4, T5);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_query_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);

impl<'b, T> Query for &'b T
where
    T: Component,
{
    type Item<'a> = Ref<'a, T>;
    type RawQuery = (Entity, &'b T);

    #[expect(
        private_interfaces,
        reason = "The private types are part of the private trait implementation"
    )]
    fn from_raw<'a>(
        (_, value): <Self::RawQuery as hecs::Query>::Item<'a>,
        _: &'a LifecycleEvents,
    ) -> Self::Item<'a> {
        Ref { value }
    }
}

pub struct Ref<'a, T> {
    value: &'a T,
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'b, T> Query for &'b mut T
where
    T: Component,
{
    type Item<'a> = RefMut<'a, T>;
    type RawQuery = (Entity, &'b mut T);

    #[expect(
        private_interfaces,
        reason = "The private types are part of the private trait implementation"
    )]
    fn from_raw<'a>(
        (entity, value): <Self::RawQuery as hecs::Query>::Item<'a>,
        events: &'a LifecycleEvents,
    ) -> Self::Item<'a> {
        RefMut {
            value,
            events,
            entity,
            changed: false,
        }
    }
}

pub struct RefMut<'a, T>
where
    T: Component,
{
    value: &'a mut T,
    events: &'a LifecycleEvents,
    entity: Entity,
    changed: bool,
}

impl<T> Deref for RefMut<'_, T>
where
    T: Component,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> DerefMut for RefMut<'_, T>
where
    T: Component,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        self.value
    }
}

impl<T> Drop for RefMut<'_, T>
where
    T: Component,
{
    fn drop(&mut self) {
        let id = TypeId::of::<T>();
        let map = self.events.changed.get(&id).unwrap();
        map.try_write().expect("Cannot create mutable reference to component while lifecycle event iterator exists").insert(self.entity);
    }
}

impl Query for Entity {
    type Item<'a> = Entity;
    type RawQuery = Entity;

    #[expect(
        private_interfaces,
        reason = "The private types are part of the private trait implementation"
    )]
    fn from_raw<'a>(
        raw: <Self::RawQuery as hecs::Query>::Item<'a>,
        _: &'a LifecycleEvents,
    ) -> Self::Item<'a> {
        raw
    }
}

pub struct QueryBorrow<'a, Q>
where
    Q: Query,
{
    pub(super) inner: hecs::QueryBorrow<'a, Q::RawQuery>,
    pub(super) events: &'a LifecycleEvents,
}

impl<'a, Q> QueryBorrow<'a, Q>
where
    Q: Query,
{
    pub fn iter(&mut self) -> QueryIter<'_, Q> {
        QueryIter {
            inner: self.inner.iter(),
            events: self.events,
        }
    }
}

pub struct QueryIter<'a, Q>
where
    Q: Query,
{
    inner: hecs::QueryIter<'a, Q::RawQuery>,
    events: &'a LifecycleEvents,
}

impl<'a, Q> Iterator for QueryIter<'a, Q>
where
    Q: Query,
{
    type Item = <Q as Query>::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item| Q::from_raw(item, self.events))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, Q> ExactSizeIterator for QueryIter<'a, Q>
where
    Q: Query,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub struct QueryMut<'a, Q>
where
    Q: Query,
{
    pub(super) inner: hecs::QueryMut<'a, Q::RawQuery>,
    pub(super) events: &'a LifecycleEvents,
}

impl<'a, Q> IntoIterator for QueryMut<'a, Q>
where
    Q: Query,
{
    type Item = <QueryIter<'a, Q> as Iterator>::Item;
    type IntoIter = QueryIter<'a, Q>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter {
            inner: self.inner.into_iter(),
            events: self.events,
        }
    }
}

impl<Q> Query for Satisfies<Q>
where
    Q: Query,
{
    type Item<'a> = bool;
    type RawQuery = Satisfies<Q::RawQuery>;

    #[expect(
        private_interfaces,
        reason = "The private types are part of the private trait implementation"
    )]
    fn from_raw<'a>(
        raw: <Self::RawQuery as hecs::Query>::Item<'a>,
        _: &'a LifecycleEvents,
    ) -> Self::Item<'a> {
        raw
    }
}

impl<Q> Query for Option<Q>
where
    Q: Query,
{
    type Item<'a> = Option<Q::Item<'a>>;
    type RawQuery = Option<Q::RawQuery>;

    #[expect(
        private_interfaces,
        reason = "The private types are part of the private trait implementation"
    )]
    fn from_raw<'a>(
        raw: <Self::RawQuery as hecs::Query>::Item<'a>,
        events: &'a LifecycleEvents,
    ) -> Self::Item<'a> {
        raw.map(|v| Q::from_raw(v, events))
    }
}

pub struct ViewBorrow<'a, Q>
where
    Q: Query,
{
    pub(super) inner: hecs::ViewBorrow<'a, Q::RawQuery>,
    pub(super) events: &'a LifecycleEvents,
}

impl<'a, Q> ViewBorrow<'a, Q>
where
    Q: Query,
{
    pub fn get(&self, entity: Entity) -> Option<Q::Item<'_>>
    where
        Q: Query,
        Q::RawQuery: hecs::QueryShared,
    {
        self.inner
            .get(entity)
            .map(|item| Q::from_raw(item, self.events))
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<Q::Item<'_>> {
        self.inner
            .get_mut(entity)
            .map(|item| Q::from_raw(item, self.events))
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.inner.contains(entity)
    }
}
