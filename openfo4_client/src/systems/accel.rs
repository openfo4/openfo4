use glam::Vec3;
use openfo4_core::ecs::Entity;
use rstar::iterators::LocateWithinDistanceIterator;
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{PointDistance, RTree};

type Elem = GeomWithData<Rectangle<[f32; 3]>, Entity>;

#[derive(Clone, Debug, Default)]
pub struct AccelerationStructure {
    tree: RTree<Elem>,
}

impl AccelerationStructure {
    pub fn query_within_radius(&self, center: Vec3, radius: f32) -> QueryRadiusIter<'_> {
        let iter = self
            .tree
            .locate_within_distance(center.to_array(), radius * radius);
        QueryRadiusIter {
            iter,
            center: center.to_array(),
        }
    }
}

pub struct QueryRadiusIter<'a> {
    center: [f32; 3],
    iter: LocateWithinDistanceIterator<'a, Elem>,
}

impl<'a> Iterator for QueryRadiusIter<'a> {
    type Item = IterElem;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|aabb| {
            let dist = aabb.distance_2(&self.center).sqrt();
            IterElem {
                entity: aabb.data,
                distance_sq: dist,
            }
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct IterElem {
    entity: Entity,
    distance_sq: f32,
}

impl IterElem {
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn distance_squared(&self) -> f32 {
        self.distance_sq
    }
}
