//! Inverse kinematics solver
//!
//! The current impelementation is based on FABRIK:
//! http://www.andreasaristidou.com/publications/papers/FABRIK.pdf

use core::f32;

use game_common::components::Transform;
use glam::Vec3;

const FABRIK_TOLERANCE: f32 = 0.01;
const FABRIK_MAX_ITERATIONS: usize = 1000;

const FORWARD: Vec3 = Vec3::NEG_Z;
const UP: Vec3 = Vec3::Y;

#[derive(Clone, Debug)]
pub struct Joint {
    pub transform: Transform,
    pub length: f32,
    pub constraint: Option<Constraint>,
}

#[derive(Copy, Clone, Debug)]
pub enum Constraint {
    Spherical(SphericalConstraint),
}

impl Constraint {
    fn apply(&self, orientation: Vec3, direction: Vec3) -> Vec3 {
        match self {
            Self::Spherical(constraint) => {
                todo!()
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SphericalConstraint {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl SphericalConstraint {
    fn apply(&self, start: Transform, target: Vec3) -> Transform {
        let distance = start.translation.distance(target);

        let dir = start.rotation * FORWARD;

        todo!()
    }
}

pub fn solve(joints: &[Joint], target: Vec3) -> Option<Vec<Transform>> {
    if joints.is_empty() {
        return Some(Vec::new());
    }

    let mut segments: Vec<_> = joints
        .iter()
        .map(|joint| Segment {
            is_edge: false,
            transform: joint.transform,
            length: joint.length,
        })
        .collect();

    segments.first_mut().unwrap().is_edge = true;
    segments.last_mut().unwrap().is_edge = true;

    if solve_internal(&mut segments, target).is_some() {
        Some(segments.into_iter().map(|v| v.transform).collect())
    } else {
        None
    }
}

fn solve_internal(segments: &mut [Segment], target: Vec3) -> Option<Vec3> {
    // let total_length = segments.iter().map(|v| v.length).sum();

    // If the distance from root to target is greater than all joints
    // combined we will never find a solution.
    // if target.distance(segments[0].translation).abs() > total_length {
    //     return None;
    // }

    let last = segments.last_mut().unwrap();

    // If the end effector is already close enough we don't need
    // to solve anything.
    if last.transform.translation.distance(target).abs() <= FABRIK_TOLERANCE {
        return Some(last.transform.translation);
    }

    last.transform.translation = target;

    // Overwritten in the first iteration.
    let mut actual_target = Vec3::ZERO;

    for _ in 0..FABRIK_MAX_ITERATIONS {
        // Backwards
        let mut iter = WindowsMut::backwards(segments);
        while let Some((a, b)) = iter.next() {
            // Don't overwrite starting point.
            if a.is_edge {
                continue;
            }

            let direction = (a.transform.translation - b.transform.translation).normalize();
            let delta = direction * a.length;

            a.transform.translation = b.transform.translation + delta;
        }

        // Forwards
        let mut iter = WindowsMut::forwards(segments);
        while let Some((a, b)) = iter.next() {
            let direction = (b.transform.translation - a.transform.translation).normalize();

            let delta = direction * a.length;

            if !b.is_edge {
                b.transform.translation = a.transform.translation + delta;
            } else {
                actual_target = a.transform.translation + delta;
            }
        }

        if actual_target.distance(target).abs() <= FABRIK_TOLERANCE {
            break;
        }
    }

    let last = segments.last_mut().unwrap();
    last.transform.translation = actual_target;

    Some(actual_target)
}

#[derive(Copy, Clone, Debug)]
struct Segment {
    transform: Transform,
    length: f32,
    is_edge: bool,
}

#[derive(Debug)]
struct WindowsMut<'a, T> {
    inner: &'a mut [T],
    next: usize,
}

impl<'a, T> WindowsMut<'a, T> {
    fn forwards(inner: &'a mut [T]) -> Self {
        Self { inner, next: 1 }
    }

    fn backwards(inner: &'a mut [T]) -> Self {
        Self {
            next: inner.len().saturating_sub(1),
            inner,
        }
    }

    // Note that we cannot make this into an `Iterator`, since the
    // `Iterator::next` API would require all returned elements to
    // remain valid for the entire `Iterator` lifetime, however
    // since we return mutable windows those unique references
    // would overlap.
    fn next(&mut self) -> Option<(&mut T, &mut T)> {
        let (a, b) = self.inner.split_at_mut_checked(self.next)?;
        let a = a.last_mut()?;
        let b = b.first_mut()?;

        self.next += 1;
        Some((a, b))
    }
}

#[cfg(test)]
mod tests {
    use game_common::components::Transform;
    use glam::{Quat, Vec3};

    use crate::ik::SphericalConstraint;

    use super::{Joint, solve};

    macro_rules! assert_eq_tolerance {
        ($lhs:expr, $rhs:expr) => {{
            const TOLERANCE: f32 = 0.5;

            if $lhs.distance($rhs) > TOLERANCE {
                panic!(
                    "assertion failed (distance(left, right) < {:?})\nleft: {:?}\nright: {:?}\ndistance = {:?}",
                    TOLERANCE, $lhs, $rhs, $lhs.distance($rhs),
                );
            }
        }};
    }

    #[test]
    fn solve_basic_2d() {
        let transforms = solve(
            &[
                Joint {
                    transform: Transform {
                        translation: Vec3::new(0.0, 0.0, 0.0),
                        ..Default::default()
                    },
                    length: 5.0,
                    constraint: None,
                },
                Joint {
                    transform: Transform {
                        translation: Vec3::new(0.0, 5.0, 0.0),
                        ..Default::default()
                    },
                    length: 5.0,
                    constraint: None,
                },
                Joint {
                    transform: Transform {
                        translation: Vec3::new(0.0, 10.0, 0.0),
                        ..Default::default()
                    },
                    length: 5.0,
                    constraint: None,
                },
            ],
            Vec3::new(10.0, 0.0, 0.0),
        )
        .unwrap();

        assert_eq!(transforms.len(), 3);
        assert_eq_tolerance!(transforms[0].translation, Vec3::new(0.0, 0.0, 0.0));
        assert_eq_tolerance!(transforms[1].translation, Vec3::new(5.0, 0.0, 0.0));
        assert_eq_tolerance!(transforms[2].translation, Vec3::new(10.0, 0.0, 0.0));
    }

    #[test]
    fn constraint_apply() {
        let constraint = SphericalConstraint::default();
        let target = constraint.apply(
            Transform {
                translation: Vec3::new(5.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                ..Default::default()
            },
            Vec3::new(3.0, 3.0, 3.0),
        );

        dbg!(&target);
    }
}
