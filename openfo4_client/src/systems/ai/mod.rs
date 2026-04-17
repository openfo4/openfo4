use game_ai::navmesh::{Agent, Mesh, NavMesh, Path};
use game_common::components::Transform;
use game_tracing::trace_span;
use hashbrown::HashMap;
use hecs::{Entity, World};

use crate::components::{Player, WorldSpace};
use crate::systems::pathing;

pub mod combat;
pub mod movement;

#[derive(Debug, Default)]
pub struct AiSystem {
    pub navmeshes: HashMap<WorldSpace, NavMesh>,
    navmesh_entities: HashMap<Entity, ()>,
    pub paths: HashMap<Entity, Path>,
}

impl AiSystem {
    pub fn update(&mut self, world: &mut World) {
        let _span = trace_span!("AiSystem::update").entered();

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
                },
                target.translation,
            ) else {
                tracing::error!("no path");
                return;
            };

            self.paths.insert(entity, path);
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
}
