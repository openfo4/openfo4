use glam::Vec4;
use hashbrown::HashMap;
use openfo4_render::api::CommandQueue;
use openfo4_render::backend::BufferUsage;
use openfo4_render::buffer::slab::{CompactSlabBuffer, SlabIndex};

use crate::entities::LightId;
use crate::lights::{PointLight, PointLightUniform};

#[derive(Debug)]
pub struct LightState {
    point_light_ids: HashMap<LightId, SlabIndex>,

    pub point_lights: CompactSlabBuffer<PointLightUniform>,
    pub point_lights_sphere: CompactSlabBuffer<Vec4>,
}

impl LightState {
    pub fn new(queue: &CommandQueue<'_>) -> Self {
        Self {
            point_light_ids: HashMap::new(),
            point_lights: CompactSlabBuffer::new(BufferUsage::STORAGE),
            point_lights_sphere: CompactSlabBuffer::new(BufferUsage::STORAGE),
        }
    }

    pub fn create_point_light(&mut self, id: LightId, light: PointLight) {
        let index = self.point_lights.insert(&PointLightUniform::new(&light));
        let sphere = light.transform.translation.extend(light.radius);
        let index2 = self.point_lights_sphere.insert(&sphere);
        debug_assert_eq!(index, index2);

        self.point_light_ids.insert(id, index);
    }

    pub fn destroy_point_light(&mut self, id: LightId) {
        let Some(index) = self.point_light_ids.remove(&id) else {
            return;
        };

        self.point_lights.remove(index);
        self.point_lights_sphere.remove(index);
    }

    pub fn num_point_lights(&self) -> u32 {
        self.point_light_ids.len() as u32
    }
}
