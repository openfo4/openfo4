use std::time::Instant;

use glam::Vec3;
use hashbrown::HashMap;
use openfo4_ai::navmesh::{Agent, Mesh, NavMesh, Path};
use openfo4_common::components::Transform;
use openfo4_common::math::Ray;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::{Event, EventBus, EventReader};
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;
use openfo4_tracing::trace_span;
use slab::Slab;

use crate::components::{Grounded, Player, WorldSpace};
use crate::interval::Interval;
use crate::state::hardcoded::HARDCODED_MODULE_ID;
use crate::systems::ai::combat::{CombatBehavior, DefaultCombatPackageConfig};
use crate::systems::ai::movement::MoveTo;
use crate::systems::pathing;

pub mod combat;
pub mod movement;

#[derive(Debug, Default)]
pub struct AiSystem {
    pub navmeshes: HashMap<WorldSpace, NavMesh>,
    navmesh_entities: HashMap<Entity, ()>,
    pub paths: Slab<(Entity, Path)>,
    behaviors: HashMap<Entity, CombatBehavior>,
    events: EventBus,
}

impl AiSystem {
    pub fn update(&mut self, world: &mut World, events: &mut EventBus) {
        let _span = trace_span!("AiSystem::update").entered();

        self.update_navmeshes(world);
        self.update_ai_actors(world, events);
    }

    pub fn run_events(&mut self, world: &mut World, events: &mut EventBus) {
        self.events.move_to(events);
    }

    fn update_navmeshes(&mut self, world: &mut World) {
        let _span = trace_span!("AiSystem::update_navmeshes").entered();

        let mut delete_entities = self.navmesh_entities.clone();
        for (entity, worldspace, mesh) in world
            .query::<(Entity, &WorldSpace, &pathing::NavMesh)>()
            .iter()
        {
            // New navmesh
            if !self.navmesh_entities.contains_key(&entity) {
                let navmesh = self.navmeshes.entry(*worldspace).or_default();
                navmesh.append(&Mesh {
                    triangles: mesh.vertices.clone(),
                    indices: mesh.indices.clone(),
                });

                self.navmesh_entities.insert(entity, ());
                continue;
            }

            delete_entities.remove(&entity);
        }
    }

    fn update_ai_actors(&mut self, world: &mut World, events: &EventBus) {
        let mut entities = Vec::new();
        for (e, _) in world.query_mut::<(Entity, &AiComponent)>() {
            entities.push(e);
        }

        let Some(player) = world
            .query::<(Entity, &Player)>()
            .iter()
            .map(|(e, _)| e)
            .next()
        else {
            return;
        };

        for e in entities {
            if world.get::<&Player>(e).is_ok() {
                continue;
            }

            let bh = self.behaviors.entry(e).or_insert_with(|| {
                CombatBehavior::new(DefaultCombatPackageConfig::default(), vec![player])
            });

            let mut ctx = Context {
                entity: e,
                worldspace: WorldSpace {
                    record: RecordId::new(HARDCODED_MODULE_ID, FormId(0x3c)),
                    is_interior: false,
                },
                navmeshes: &self.navmeshes,
                world,
                paths: &mut self.paths,
                rx_events: events,
                tx_events: &mut self.events,
            };

            bh.update(&mut ctx);
        }
    }

    pub fn test_player_path(&mut self, world: &World) {
        for (entity, _, transform, worldspace) in world
            .query::<(Entity, &Player, &Transform, &WorldSpace)>()
            .iter()
        {
            let navmesh = self.navmeshes.get(&*worldspace).unwrap();

            let mut target = *transform;
            target.translation.x += 10.0;

            let Some(path) = navmesh.search(
                &Agent {
                    position: transform.translation,
                    width: 1.0,
                },
                target.translation,
            ) else {
                tracing::error!("no path");
                return;
            };
        }
    }
}

pub trait AiPackage {
    fn update(&mut self);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Status {
    Success,
    Failure,
    Running,
}

pub struct AiComponent;

#[derive(Debug)]
struct BehaviorTree {}

trait Behavior {
    fn start(&mut self, ctx: &mut Context<'_>);

    fn update(&mut self, ctx: &mut Context<'_>) -> Status;

    fn finish(&mut self, ctx: &mut Context<'_>);
}

struct Context<'a> {
    entity: Entity,
    worldspace: WorldSpace,
    navmeshes: &'a HashMap<WorldSpace, NavMesh>,
    world: &'a mut World,
    paths: &'a mut Slab<(Entity, Path)>,
    rx_events: &'a EventBus,
    tx_events: &'a mut EventBus,
}

impl Context<'_> {
    fn read_events<T>(&self) -> EventReader<'_, T>
    where
        T: Event,
    {
        self.rx_events.read()
    }

    fn send_event<T>(&mut self, event: T)
    where
        T: Event,
    {
        self.tx_events.send(event);
    }
}

#[derive(Debug)]
struct RandomMovement {
    target: Option<MoveTo>,
    timer: Interval,
}

impl Behavior for RandomMovement {
    fn start(&mut self, ctx: &mut Context<'_>) {}

    fn update(&mut self, ctx: &mut Context<'_>) -> Status {
        match &mut self.target {
            Some(target) => {
                let s = target.update(ctx);
                dbg!(s);
                if s == Status::Running {
                    return Status::Running;
                }

                self.target = None;
            }
            None => {
                if self.timer.is_ready(Instant::now()) {
                    let Ok(position) = ctx.world.get::<&Transform>(ctx.entity).map(|v| *v) else {
                        return Status::Failure;
                    };

                    if ctx.world.get::<&Grounded>(ctx.entity).is_err() {
                        return Status::Running;
                    }

                    let sign_x = rand::random::<bool>();
                    let sign_y = rand::random::<bool>();

                    let mut target = position.translation;
                    if sign_x {
                        target.x += 1.0;
                    } else {
                        target.x -= 1.0;
                    }
                    if sign_y {
                        target.z += 1.0;
                    } else {
                        target.z -= 1.0;
                    }

                    dbg!("pre", target);

                    let navmesh = ctx.navmeshes.get(&ctx.worldspace).unwrap();
                    if let Some(toi) = navmesh.cast_ray(Ray::new(target, Vec3::Y), f32::MAX) {
                        dbg!("toi up", toi);
                        target += toi * Vec3::Y;
                    } else if let Some(toi) = navmesh.cast_ray(Ray::new(target, -Vec3::Y), f32::MAX)
                    {
                        dbg!("toi down", toi);
                        target -= toi * Vec3::Y;
                    }

                    // target += 0.01 * Vec3::Y;

                    dbg!("post", &target);

                    let mut mt = MoveTo::new(target, 225.0);
                    mt.start(ctx);
                    self.target = Some(mt);
                }
            }
        }

        Status::Running
    }

    fn finish(&mut self, ctx: &mut Context<'_>) {}
}

pub struct AiPackages {
    pub combat: CombatBehavior,
}

#[derive(Clone, Debug)]
pub struct Sandbox {}

trait Node {
    type Result;
    type Error;

    #[expect(unused)]
    fn init(&mut self, ctx: &mut Context<'_>) {}

    // fn update(&mut self, ctx: &mut Context<'_>) -> Status<Self::Result, Self::Error>;

    #[expect(unused)]
    fn deinit(&mut self, ctx: &mut Context<'_>) {}
}
