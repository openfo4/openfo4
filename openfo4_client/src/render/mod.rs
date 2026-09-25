pub mod physics_debug;
mod skinning;

use std::sync::Arc;

use game_common::components::AmbientLight as AmbientLightComp;
use game_common::components::DirectionalLight as DirectionalLightComp;
use game_common::components::PointLight as PointLightComp;
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
use hashbrown::HashMap;
use hashbrown::HashSet;
use openfo4_core::ecs::{Entity, World};

use crate::components::{Camera as CameraComp, WorldSpace};
use crate::shared::SharedState;

pub struct RenderState {
    entities: Entities,
    pub renderer: Renderer,
    cameras: HashMap<Entity, CameraHandle>,
    target: RenderTarget,
    pub target_size: UVec2,
    task_pool: TaskPool,
    shared: Arc<SharedState>,
    // FIXME: worldspace cleanup,
    // for now we never destroy scenes when all entities are dropped.
    worldspaces: HashMap<WorldSpace, SceneHandle>,

    ambient_lights: HashMap<Entity, LightHandle>,
    directional_lights: HashMap<Entity, LightHandle>,
    point_lights: HashMap<Entity, LightHandle>,
    objects: HashMap<Entity, ObjectHandle>,

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
        Self {
            entities,
            renderer,
            cameras: HashMap::new(),
            target,
            target_size: UVec2::ZERO,
            task_pool: TaskPool::new(1),
            objects: HashMap::new(),
            shared,
            gizmos,
            worldspaces: HashMap::default(),
            ambient_lights: HashMap::new(),
            directional_lights: HashMap::new(),
            point_lights: HashMap::new(),
        }
    }

    pub fn render(&mut self) {
        self.gizmos.swap_buffers();
        self.renderer.render(&self.task_pool);
    }

    pub fn sync(&mut self, world: &World) {
        let _span = trace_span!("RenderState::sync").entered();

        self.cameras.clear();

        for (entity, transform, cam, worldspace) in world
            .query::<(Entity, &Transform, &CameraComp, &WorldSpace)>()
            .iter()
        {
            let scene = self
                .worldspaces
                .entry(*worldspace)
                .or_insert_with(|| self.entities.create_scene());

            let mut camera = Camera {
                transform: *transform,
                projection: Projection::default(),
                target: self.target,
                scene: scene.clone(),
            };
            camera.projection.fov = cam.fov;
            camera.update_aspect_ratio(self.target_size);

            let handle = self.entities.create_camera(camera.clone());

            self.cameras.insert(entity, handle);
            self.gizmos.update_camera(camera);
        }

        for entity in world.component_removed::<GlobalTransform>() {
            self.directional_lights.remove(&entity);
            self.point_lights.remove(&entity);
            self.objects.remove(&entity);
        }

        for entity in world.component_removed::<WorldSpace>() {
            self.ambient_lights.remove(&entity);
            self.directional_lights.remove(&entity);
            self.point_lights.remove(&entity);
            self.objects.remove(&entity);
        }

        for entity in world.component_removed::<AmbientLightComp>() {
            self.ambient_lights.remove(&entity);
        }

        for entity in world.component_removed::<DirectionalLightComp>() {
            self.directional_lights.remove(&entity);
        }

        for entity in world.component_removed::<PointLightComp>() {
            self.point_lights.remove(&entity);
        }

        for entity in world.component_removed::<Object>() {
            self.objects.remove(&entity);
        }

        // Ambient lights
        {
            let view = world.view::<(&WorldSpace, &AmbientLightComp)>();

            for entity in world
                .component_added::<WorldSpace>()
                .chain(world.component_changed::<WorldSpace>())
                .chain(world.component_added::<AmbientLightComp>())
                .chain(world.component_changed::<AmbientLightComp>())
            {
                let Some((worldspace, light)) = view.get(entity) else {
                    continue;
                };

                let scene = self
                    .worldspaces
                    .entry(*worldspace)
                    .or_insert_with(|| self.entities.create_scene());

                let handle = self.entities.create_light(Light::Ambient(AmbientLight {
                    scene: scene.clone(),
                    brightness: light.brightness,
                    top: light.top,
                    bottom: light.bottom,
                    left: light.left,
                    right: light.right,
                    front: light.front,
                    back: light.back,
                }));

                self.ambient_lights.insert(entity, handle);
            }
        }

        // Directional lights
        {
            let view = world.view::<(&GlobalTransform, &WorldSpace, &DirectionalLightComp)>();

            for entity in world
                .component_added::<GlobalTransform>()
                .chain(world.component_changed::<GlobalTransform>())
                .chain(world.component_added::<WorldSpace>())
                .chain(world.component_changed::<WorldSpace>())
                .chain(world.component_added::<DirectionalLightComp>())
                .chain(world.component_changed::<DirectionalLightComp>())
            {
                let Some((transform, worldspace, light)) = view.get(entity) else {
                    continue;
                };

                let scene = self
                    .worldspaces
                    .entry(*worldspace)
                    .or_insert_with(|| self.entities.create_scene());

                let handle = self
                    .entities
                    .create_light(Light::Directional(DirectionalLight {
                        transform: transform.0,
                        scene: scene.clone(),
                        color: light.color,
                        illuminance: light.illuminance,
                    }));

                self.directional_lights.insert(entity, handle);
            }
        }

        // Point lights
        {
            let view = world.view::<(&GlobalTransform, &WorldSpace, &PointLightComp)>();

            for entity in world
                .component_added::<GlobalTransform>()
                .chain(world.component_changed::<GlobalTransform>())
                .chain(world.component_added::<WorldSpace>())
                .chain(world.component_changed::<WorldSpace>())
                .chain(world.component_added::<PointLightComp>())
                .chain(world.component_changed::<PointLightComp>())
            {
                let Some((transform, worldspace, light)) = view.get(entity) else {
                    continue;
                };

                let scene = self
                    .worldspaces
                    .entry(*worldspace)
                    .or_insert_with(|| self.entities.create_scene());

                let handle = self.entities.create_light(Light::Point(PointLight {
                    color: light.color,
                    intensity: light.intensity,
                    radius: light.radius,
                    transform: transform.0,
                    scene: scene.clone(),
                }));

                self.point_lights.insert(entity, handle);
            }
        }

        // Objects
        {
            let view = world.view::<(&GlobalTransform, &WorldSpace, &RenderObject)>();

            let mut recreated = HashSet::new();

            for entity in world
                .component_added::<GlobalTransform>()
                .chain(world.component_added::<WorldSpace>())
                .chain(world.component_changed::<WorldSpace>())
                .chain(world.component_added::<RenderObject>())
                .chain(world.component_changed::<RenderObject>())
            {
                let Some((transform, worldspace, object)) = view.get(entity) else {
                    continue;
                };

                let scene = self
                    .worldspaces
                    .entry(*worldspace)
                    .or_insert_with(|| self.entities.create_scene());

                let handle = self.entities.create_object(Object {
                    transform: transform.0,
                    mesh: object.mesh.clone(),
                    material: object.material.clone(),
                    scene: scene.clone(),
                });
                self.objects.insert(entity, handle);
                recreated.insert(entity);
            }

            for entity in world.component_changed::<GlobalTransform>() {
                // If the entity was already changed by other means, it will already
                // have its transform updated.
                if recreated.contains(&entity) {
                    continue;
                }

                let Some((transform, _, _)) = view.get(entity) else {
                    continue;
                };

                let Some(handle) = self.objects.get(&entity) else {
                    continue;
                };

                self.entities.set_transform(handle, transform.0);
            }
        }

        skinning::mesh_skinning(world, &self.objects, &self.shared);

        // Note that the renderer can run in parallel to our thread.
        // If the renderer is slow it might still process the events
        // from our last submission.
        // For this case we notify it to stop after a single submission
        // to prevent it from already reading the next submission which
        // may still be incomplete.
        self.entities.barrier();
    }
}

#[derive(Clone, Debug)]
pub struct RenderObject {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
}
