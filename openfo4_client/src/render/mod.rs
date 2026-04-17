pub mod physics_debug;
mod skinning;

use std::sync::Arc;

use game_common::components::{GlobalTransform, Transform};
use game_core_pipeline::camera::{Camera, Projection};
use game_core_pipeline::entities::{
    CameraHandle, Entities, LightHandle, MaterialHandle, MeshHandle, Object, ObjectHandle,
    SceneHandle,
};
use game_core_pipeline::lights::{AmbientLight, DirectionalLight, Light, PointLight};
use game_gizmos::Gizmos;
use game_render::RenderTarget;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::UVec2;
use hashbrown::{HashMap, HashSet};
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::{ComponentChanged, EventBus};

use crate::components::{Camera as CameraComp, Player, WorldSpace};
use crate::shared::SharedState;

pub struct RenderState {
    entities: Entities,
    pub renderer: Renderer,
    cameras: HashMap<Entity, CameraHandle>,
    scene: SceneHandle,
    target: RenderTarget,
    pub target_size: UVec2,
    task_pool: TaskPool,
    pub objects: HashMap<Entity, ObjectHandle>,
    shared: Arc<SharedState>,
    prev_transforms: HashMap<Entity, GlobalTransform>,

    pub lights: HashMap<Entity, LightHandle>,
    pub gizmos: Gizmos,
}

impl RenderState {
    pub fn new(
        renderer: Renderer,
        target: RenderTarget,
        shared: Arc<SharedState>,
        entities: Entities,
        gizmos: Gizmos,
    ) -> Self {
        let scene = entities.create_scene();
        Self {
            entities,
            renderer,
            cameras: HashMap::new(),
            scene,
            target,
            target_size: UVec2::ZERO,
            task_pool: TaskPool::new(1),
            objects: HashMap::new(),
            shared,
            prev_transforms: HashMap::new(),
            lights: HashMap::new(),
            gizmos,
        }
    }

    pub fn render(&mut self) {
        self.gizmos.swap_buffers();
        self.renderer.render(&self.task_pool);
    }

    pub fn sync(&mut self, world: &World, events: &EventBus) {
        let _span = trace_span!("RenderState::sync").entered();

        let Some(player_worldspace) = world
            .query::<(&WorldSpace, &Player)>()
            .iter()
            .map(|v| *v.0)
            .next()
        else {
            return;
        };

        self.cameras.clear();
        self.lights.clear();

        for (entity, transform, cam) in world.query::<(Entity, &Transform, &CameraComp)>().iter() {
            let mut camera = Camera {
                transform: *transform,
                scene: self.scene.clone(),
                projection: Projection::default(),
                target: self.target,
            };
            camera.projection.fov = cam.fov;
            camera.update_aspect_ratio(self.target_size);

            let handle = self.entities.create_camera(camera.clone());

            self.cameras.insert(entity, handle);
            self.gizmos.update_camera(camera);
        }

        let mut lights_to_remove = self.lights.clone();

        for (entity, light, worldspace) in world
            .query::<(Entity, &game_common::components::AmbientLight, &WorldSpace)>()
            .iter()
        {
            if *worldspace != player_worldspace {
                continue;
            }

            lights_to_remove.remove(&entity);

            let handle = self.entities.create_light(Light::Ambient(AmbientLight {
                scene: self.scene.clone(),
                brightness: light.brightness,
                top: light.top,
                bottom: light.bottom,
                left: light.left,
                right: light.right,
                front: light.front,
                back: light.back,
            }));

            self.lights.insert(entity, handle);
        }

        for (entity, transform, light, worldspace) in world
            .query::<(
                Entity,
                &GlobalTransform,
                &game_common::components::DirectionalLight,
                &WorldSpace,
            )>()
            .iter()
        {
            if *worldspace != player_worldspace {
                continue;
            }

            lights_to_remove.remove(&entity);

            let handle = self
                .entities
                .create_light(Light::Directional(DirectionalLight {
                    transform: transform.0,
                    scene: self.scene.clone(),
                    color: light.color,
                    illuminance: light.illuminance,
                }));

            self.lights.insert(entity, handle);
        }

        for (entity, transform, light, worldspace) in world
            .query::<(
                Entity,
                &GlobalTransform,
                &game_common::components::PointLight,
                &WorldSpace,
            )>()
            .iter()
        {
            if *worldspace != player_worldspace {
                continue;
            }

            lights_to_remove.remove(&entity);

            let handle = self.entities.create_light(Light::Point(PointLight {
                color: light.color,
                intensity: light.intensity,
                radius: light.radius,
                transform: transform.0,
                scene: self.scene.clone(),
            }));

            self.lights.insert(entity, handle);
        }

        for (entity, _) in lights_to_remove {
            self.lights.remove(&entity);
        }

        let objects_changed: HashSet<_> = events
            .read::<ComponentChanged<RenderObject>>()
            .map(|v| v.get())
            .collect();

        let mut objects_to_remove = self.objects.clone();
        for (entity, transform, object, worldspace) in world
            .query::<(Entity, &GlobalTransform, &RenderObject, &WorldSpace)>()
            .iter()
        {
            if *worldspace != player_worldspace {
                continue;
            }

            objects_to_remove.remove(&entity);

            if let Some(prev_transform) = self.prev_transforms.get(&entity) {
                if *prev_transform == *transform && !objects_changed.contains(&entity) {
                    continue;
                }
            }
            self.prev_transforms.insert(entity, *transform);

            let handle = self.entities.create_object(Object {
                transform: transform.0,
                mesh: object.mesh.clone(),
                material: object.material.clone(),
                scene: self.scene.clone(),
            });
            self.objects.insert(entity, handle);
        }

        for (entity, id) in objects_to_remove {
            self.prev_transforms.remove(&entity);
            self.objects.remove(&entity);
        }

        skinning::mesh_skinning(world, &self.objects, &self.shared);
    }
}

#[derive(Clone, Debug)]
pub struct RenderObject {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
}
