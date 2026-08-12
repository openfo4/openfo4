use game_ai::navmesh::{Agent, Path};
use game_common::components::Transform;
use game_common::math::Ray;
use game_tracing::trace_span;
use glam::Vec3;

use crate::components::MovementState;
use crate::systems::FacingDirection;
use crate::systems::ai::{Behavior, Context, Status};
use crate::systems::movement::{MovementDirection, MovementSpeed};

const TARGET_REACHED_THRESHOLD: f32 = 0.05;

#[derive(Clone, Debug)]
pub struct MoveTo {
    target: Vec3,
    face: Face,
    state: PathState,
}

impl MoveTo {
    pub fn new(target: Vec3, face: Face) -> Self {
        Self {
            target,
            face,
            state: PathState::None,
        }
    }
}

impl Behavior for MoveTo {
    fn start(&mut self, ctx: &mut Context<'_>) {
        let Ok(position) = ctx.world.get::<&Transform>(ctx.entity) else {
            self.state = PathState::Failed;
            return;
        };

        let Some(navmesh) = ctx.navmeshes.get(&ctx.worldspace) else {
            self.state = PathState::Failed;
            return;
        };

        let mut start = position.translation;
        if let Some(toi) = navmesh.cast_ray(Ray::new(start, Vec3::Y), 100.0) {
            start += toi * Vec3::Y;
        }

        start += 0.001 * Vec3::Y;

        let Some(path) = navmesh.search(
            &Agent {
                position: start,
                width: 1.0,
            },
            self.target,
        ) else {
            self.state = PathState::Failed;
            return;
        };

        let index = ctx.paths.insert((ctx.entity, path.clone()));
        self.state = PathState::Moving(path, index);
    }

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        let _span = trace_span!("MoveTo::update").entered();

        match &mut self.state {
            PathState::None => unreachable!(),
            PathState::Moving(path, index) => loop {
                let Some(waypoint) = path.waypoints.first() else {
                    ctx.paths.remove(*index);
                    self.state = PathState::Reached;

                    ctx.world
                        .insert(
                            ctx.entity,
                            (MovementDirection::ZERO, MovementSpeed::default()),
                        )
                        .ok();

                    return Status::Success;
                };

                let Ok(transform) = ctx.world.get::<&Transform>(ctx.entity).map(|v| *v) else {
                    ctx.paths.remove(*index);
                    self.state = PathState::Failed;
                    return Status::Failure;
                };

                let delta = waypoint - transform.translation;
                if delta.length_squared() < TARGET_REACHED_THRESHOLD {
                    path.waypoints.remove(0);
                    continue;
                }

                let facing_dir = ctx
                    .world
                    .get::<&FacingDirection>(ctx.entity)
                    .map(|f| f.0)
                    .unwrap_or_default();
                let dir = facing_dir.conjugate() * delta.normalize_or_zero();

                ctx.world
                    .insert(
                        ctx.entity,
                        (
                            MovementDirection::new_normalized(dir),
                            MovementSpeed { speed: 225.0 / 4.0 },
                            MovementState {
                                is_moving: true,
                                is_sneaking: false,
                                is_sprinting: false,
                            },
                        ),
                    )
                    .unwrap();

                return Status::Running;
            },
            PathState::Failed => Status::Failure,
            PathState::Reached => Status::Success,
        }
    }

    fn finish(&mut self, ctx: &mut Context<'_>) {
        if let PathState::Moving(_, index) = &self.state {
            ctx.paths.remove(*index);
            self.state = PathState::None;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Face {
    Forward,
    At(Vec3),
}

#[derive(Clone, Debug)]
enum PathState {
    None,
    Moving(Path, usize),
    Reached,
    Failed,
}
