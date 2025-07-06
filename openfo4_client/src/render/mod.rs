pub mod physics_debug;

use std::sync::{Arc, OnceLock};

use game_common::components::{Color, GlobalTransform, Transform};
use game_core_pipeline::camera::{Camera, Projection};
use game_core_pipeline::entities::{
    CameraHandle, Entities, LightHandle, MaterialHandle, MeshHandle, Object, ObjectHandle,
    SceneHandle,
};
use game_core_pipeline::lights::{DirectionalLight, Light};
use game_gizmos::Gizmos;
use game_render::Renderer;
use game_render::camera::RenderTarget;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::{UVec2, Vec3};
use hashbrown::HashMap;
use hecs::{Entity, World};

use crate::cell::CellId;
use crate::components::{Camera as CameraComp, NiTransform};
use crate::shared::{ImageAsset, MaterialAsset, MeshAsset, SharedState};
use crate::state::asset::Instance;

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
    lights: Vec<LightHandle>,
    pub gizmos: Gizmos,
}

impl RenderState {
    pub fn new(
        mut renderer: Renderer,
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
            lights: Vec::new(),
            gizmos,
        }
    }

    pub fn render(&mut self) {
        self.gizmos.swap_buffers();
        self.renderer.render(&self.task_pool);
    }

    pub fn sync(&mut self, world: &World) {
        let _span = trace_span!("RenderState::sync").entered();

        self.cameras.clear();
        self.lights.clear();

        for (entity, (transform, cam)) in &mut world.query::<(&Transform, &CameraComp)>() {
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

            // self.lights
            //     .push(self.entities.create_light(Light::Point(PointLight {
            //         transform: *transform,
            //         scene: self.scene.clone(),
            //         color: Color::WHITE,
            //         intensity: 1000.0,
            //         radius: 1000.0,
            //     })));
        }

        static ONCE: OnceLock<()> = OnceLock::new();
        ONCE.get_or_init(|| {
            let l = self
                .entities
                .create_light(Light::Directional(DirectionalLight {
                    scene: self.scene.clone(),
                    transform: Transform::from_translation(Vec3::splat(200.0))
                        .looking_at(Vec3::ZERO, Vec3::Y),
                    color: Color::WHITE,
                    // illuminance: 120_000.0,
                    illuminance: 25_000.0,
                }));
            core::mem::forget(l);
        });

        let mut objects_to_remove = self.objects.clone();
        for (entity, (transform, object)) in &mut world.query::<(&GlobalTransform, &RenderObject)>()
        {
            objects_to_remove.remove(&entity);

            if let Some(prev_transform) = self.prev_transforms.get(&entity) {
                if prev_transform == transform {
                    continue;
                }
            }
            self.prev_transforms.insert(entity, *transform);

            self.objects.remove(&entity);

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
    }
}

#[derive(Clone, Debug)]
pub struct RenderObject {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
}
