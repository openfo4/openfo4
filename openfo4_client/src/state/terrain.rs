use std::ops::Deref;
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use game_common::components::Transform;
use game_core_pipeline::entities::{Entities, MeshHandle};
use game_core_pipeline::material::StandardMaterial;
use game_render::backend::TextureFormat;
use game_render::texture::Image;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use glam::{FloatExt, UVec2, Vec2, Vec3A, Vec3Swizzles, Vec4};
use hashbrown::HashMap;
use image::{ImageBuffer, Pixel, Rgba};
use openfo4_asset::Handle;
use openfo4_core::ecs::{Entity, World};

use crate::assets::data_file::DataFile;
use crate::cell::{CELL_SIZE, CellId};
use crate::components::{NiTransform, Player, WorldSpace};
use crate::render::RenderObject;

const UV_SCALE: Vec2 = Vec2::splat(16.0);

pub struct TerrainManager {
    pool: Arc<TaskPool>,
    regions: HashMap<CellId, TerrainRegion>,
    entities: Entities,
    ready_queue: Arc<SegQueue<(CellId, RenderObject)>>,
}

impl TerrainManager {
    pub fn new(pool: Arc<TaskPool>, entities: Entities) -> Self {
        Self {
            pool,
            entities,
            regions: HashMap::new(),
            ready_queue: Arc::default(),
        }
    }

    pub fn update(&mut self, world: &mut World) {
        let _span = trace_span!("TerrainManager::update").entered();

        let Some((transform, worldspace, _)) = world
            .query_mut::<(&Transform, &WorldSpace, &Player)>()
            .into_iter()
            .next()
        else {
            return;
        };

        let player_cell = CellId::from(NiTransform::from(*transform));

        for (cell, region) in &mut self.regions {
            // FIXME: Need some finetuning on this.
            let res = match cell.distance_max(player_cell) {
                // ~200MB
                0 | 1 => 4096,
                // ~12MB
                2 => 1024,
                // ~3MB
                3 => 512,
                // ~750KB
                4 => 256,
                // ~200KB
                5 => 128,
                // ~50KB
                _ => 64,
            };

            if region.res != res {
                region.res = res;

                let mesh = region.mesh.clone();
                let layers = Arc::clone(&region.layers);
                let entities = self.entities.clone();
                let cell = *cell;
                let ready_queue = Arc::clone(&self.ready_queue);

                self.pool.spawn(async move {
                    let size = UVec2::splat(res);
                    let terrain = combine_texture_layers(&layers, size);

                    let diffuse = entities.create_image(Image::new(
                        size,
                        TextureFormat::Rgba8UnormSrgb,
                        terrain.diffuse_texture.into_vec(),
                    ));
                    let normal = entities.create_image(Image::new(
                        size,
                        TextureFormat::Rgba8Unorm,
                        terrain.normal_texture.into_vec(),
                    ));
                    let smooth_spec = entities.create_image(Image::new(
                        size,
                        TextureFormat::Rgba8UnormSrgb,
                        terrain.smooth_spec_texture.into_vec(),
                    ));

                    let material = entities.create_material(StandardMaterial {
                        base_color_texture: Some(diffuse),
                        normal_texture: Some(normal),
                        // Normal maps are still in DirectX format.
                        flip_normal_y: true,
                        two_component_normal_encoding: true,
                        specular_glossiness_texture: Some(smooth_spec),
                        // The glossiness is reconstructed from 1.0 - roughness.
                        // We want a glossiness factor of 1.0 since we have premultiplied
                        // all glossiness factors into the glossiness texture.
                        roughness: 0.0,
                        specular_strength: 1.0,
                        ..Default::default()
                    });

                    ready_queue.push((cell, RenderObject { mesh, material }));
                });
            }
        }

        while let Some((cell, object)) = self.ready_queue.pop() {
            let region = self.regions.get_mut(&cell).unwrap();

            let entity = region.entity.expect("terrain entity not set");

            world.insert_one(entity, object).unwrap();
        }
    }

    pub fn insert(&mut self, region: TerrainRegion) {
        self.regions.insert(region.cell, region);
    }
}

#[derive(Clone, Debug)]
pub struct TerrainRegion {
    pub cell: CellId,
    pub mesh: MeshHandle,
    pub layers: Arc<Vec<TerrainLayer>>,
    pub res: u32,
    pub entity: Option<Entity>,
}

#[derive(Clone, Debug)]
pub struct TerrainLayer {
    /// Position between 0 and CELL_SIZE.
    pub position: UVec2,
    /// Size such that position+size <= CELL_SIZE.
    pub size: UVec2,
    pub layer: i16,
    pub alpha: [[f32; 17]; 17],
    pub diffuse_texture: Option<Handle<DataFile<ImageBuffer<Rgba<u8>, Vec<u8>>>>>,
    pub normal_texture: Option<Handle<DataFile<ImageBuffer<Rgba<u8>, Vec<u8>>>>>,
    pub smooth_spec_texture: Option<Handle<DataFile<ImageBuffer<Rgba<u8>, Vec<u8>>>>>,
    pub smoothness: f32,
    pub specular_strength: f32,
}

#[derive(Clone, Debug)]
pub struct Terrain {
    pub diffuse_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub normal_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub smooth_spec_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

fn combine_texture_layers(layers: &[TerrainLayer], size: UVec2) -> Terrain {
    let _span = trace_span!("combine_texture_layers").entered();

    let mut diffuse_texture = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(size.x, size.y);
    let mut normal_texture = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(size.x, size.y);
    let mut smooth_spec_texture = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(size.x, size.y);

    merge_layers(&mut diffuse_texture, layers, |dst, layer, u, v| {
        let Some(texture) = &layer.diffuse_texture else {
            return;
        };

        if texture.width() == 0 || texture.height() == 0 {
            return;
        }

        let alpha = layer.alpha.sample_linear(u, v);
        let mut color = texture.sample_linear(u * UV_SCALE.x, v * UV_SCALE.y);
        color.0[3] = ((color.0[3] as f32 / 255.0) * alpha * 255.0) as u8;

        dst.blend(&color);
    });

    merge_layers(&mut normal_texture, layers, |pixel, layer, u, v| {
        let Some(texture) = &layer.normal_texture else {
            return;
        };
        if texture.width() == 0 || texture.height() == 0 {
            return;
        }

        let alpha = layer.alpha.sample_linear(u, v);

        let src_normal = texture.sample_linear(u * UV_SCALE.x, v * UV_SCALE.y);
        let dst_normal = *pixel;

        let mut src = Vec3A::new(src_normal.0[0] as f32, src_normal.0[1] as f32, 0.0) / 255.0;
        let mut dst = Vec3A::new(dst_normal.0[0] as f32, dst_normal.0[1] as f32, 0.0) / 255.0;

        // Map values from 0..1 to -1..1.
        src = src * 2.0 - 1.0;
        dst = dst * 2.0 - 1.0;

        // Reconstruct Z channel.
        // Z = sqrt(1 - x^2 - y^2)
        src.z = (1.0 - src.xy().dot(src.xy())).sqrt();
        dst.z = (1.0 - dst.xy().dot(dst.xy())).sqrt();

        // Blend normals.
        let mut out = dst.lerp(src, alpha);

        // Map back from -1..1 to 0..1.
        out = out / 2.0 + 0.5;

        out = out * 255.0;
        let out = Rgba([out.x as u8, out.y as u8, 0, 0]);

        *pixel = out;
    });

    merge_layers(&mut smooth_spec_texture, layers, |dst, layer, u, v| {
        let Some(texture) = &layer.smooth_spec_texture else {
            return;
        };

        if texture.width() == 0 || texture.height() == 0 {
            return;
        }

        let alpha = layer.alpha.sample_linear(u, v);
        let mut gloss_spec = texture.sample_linear(u * UV_SCALE.x, v * UV_SCALE.y);

        // Scale the glossiness/specular texture with the material factors.
        // Specular strength in red channel, glossiness in green channel.
        gloss_spec.0[0] = (gloss_spec.0[0] as f32 * layer.specular_strength) as u8;
        gloss_spec.0[1] = (gloss_spec.0[1] as f32 * layer.smoothness) as u8;

        *dst = lerp_pixel(*dst, gloss_spec, alpha);
    });

    Terrain {
        diffuse_texture,
        normal_texture,
        smooth_spec_texture,
    }
}

fn merge_layers<F>(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, layers: &[TerrainLayer], mut f: F)
where
    F: FnMut(&mut Rgba<u8>, &TerrainLayer, f32, f32),
{
    let size = UVec2::new(img.width(), img.height());

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let pos_x = x as f32 / size.x as f32;
        let pos_y = y as f32 / size.y as f32;

        for layer in layers {
            let start_x = layer.position.x as f32 / CELL_SIZE.x as f32;
            let start_y = layer.position.y as f32 / CELL_SIZE.y as f32;
            let end_x = (layer.position.x + layer.size.x) as f32 / CELL_SIZE.x as f32;
            let end_y = (layer.position.y + layer.size.y) as f32 / CELL_SIZE.y as f32;

            if pos_x < start_x || pos_x >= end_x || pos_y < start_y || pos_y >= end_y {
                continue;
            }

            let u = (pos_x - start_x) / (end_x - start_x);
            let v = (pos_y - start_y) / (end_y - start_y);

            f(pixel, layer, u, v);
        }
    }
}

trait SampleImage {
    type Pixel;

    fn sample_linear(&self, x: f32, y: f32) -> Self::Pixel;
}

impl<C> SampleImage for ImageBuffer<Rgba<u8>, C>
where
    C: Deref<Target = [<Rgba<u8> as Pixel>::Subpixel]>,
{
    type Pixel = Rgba<u8>;

    fn sample_linear(&self, mut x: f32, mut y: f32) -> Self::Pixel {
        x = x.rem_euclid(1.0) * self.width() as f32;
        y = y.rem_euclid(1.0) * self.height() as f32;

        let xf = x.floor() as u32;
        let yf = y.floor() as u32;
        let xc = u32::min(x.ceil() as u32, self.width());
        let yc = u32::min(y.ceil() as u32, self.height());

        let p11 = *self.get_pixel(xf, yf);
        let p12 = *self.get_pixel(xf, yc);
        let p21 = *self.get_pixel(xc, yf);
        let p22 = *self.get_pixel(xc, yc);

        let p1 = lerp_pixel(p11, p21, x.fract());
        let p2 = lerp_pixel(p12, p22, x.fract());
        let p = lerp_pixel(p1, p2, y.fract());

        p
    }
}

impl<const X: usize, const Y: usize> SampleImage for [[f32; X]; Y] {
    type Pixel = f32;

    fn sample_linear(&self, mut x: f32, mut y: f32) -> Self::Pixel {
        x = x.rem_euclid(1.0) * X as f32;
        y = y.rem_euclid(1.0) * Y as f32;

        let xf = x.floor() as usize;
        let yf = y.floor() as usize;
        let xc = usize::min(x.ceil() as usize, X - 1);
        let yc = usize::min(y.ceil() as usize, Y - 1);

        let p11 = self[xf][yf];
        let p12 = self[xf][yc];
        let p21 = self[xc][yf];
        let p22 = self[xc][yc];

        let p1 = f32::lerp(p11, p21, x.fract());
        let p2 = f32::lerp(p12, p22, x.fract());
        let p = f32::lerp(p1, p2, y.fract());

        p
    }
}

fn lerp_pixel(a: Rgba<u8>, b: Rgba<u8>, s: f32) -> Rgba<u8> {
    let lhs = Vec4::new(a.0[0] as f32, a.0[1] as f32, a.0[2] as f32, a.0[3] as f32) / 255.0;
    let rhs = Vec4::new(b.0[0] as f32, b.0[1] as f32, b.0[2] as f32, b.0[3] as f32) / 255.0;
    let out = lhs.lerp(rhs, s) * 255.0;
    Rgba([out[0] as u8, out[1] as u8, out[2] as u8, out[3] as u8])
}

#[cfg(test)]
mod tests {
    use game_common::assert_approx_eq;
    use image::ImageBuffer;

    use crate::cell::CELL_SIZE;
    use crate::state::terrain::{SampleImage, TerrainLayer, merge_layers};

    #[test]
    fn test_merge_layers() {
        let mut img = ImageBuffer::new(1024, 1024);

        merge_layers(
            &mut img,
            &[TerrainLayer {
                position: CELL_SIZE / 2,
                size: CELL_SIZE / 2,
                alpha: [[0.0; 17]; 17],
                diffuse_texture: None,
                normal_texture: None,
                smooth_spec_texture: None,
                layer: 0,
                smoothness: 0.0,
                specular_strength: 0.0,
            }],
            |_, _, u, v| {
                assert!(u >= 0.0 && u <= 1.0);
                assert!(v >= 0.0 && v <= 1.0);
            },
        );
    }

    #[test]
    fn test_sample_linear() {
        const EPSILON: f32 = 0.01;

        let mut data = [[0.0; 100]; 100];
        for x in 0..100 {
            for y in 0..100 {
                data[x][y] = (x + y) as f32;
            }
        }

        assert_approx_eq!(data.sample_linear(0.0, 0.0), 0.0, EPSILON);
        assert_approx_eq!(
            data.sample_linear(1.0 - f32::EPSILON, 1.0 - f32::EPSILON),
            198.0,
            EPSILON,
        );
        assert_approx_eq!(data.sample_linear(0.1, 0.5), 60.0, EPSILON);
        assert_approx_eq!(data.sample_linear(0.5, 0.5), 100.0, EPSILON);
        assert_approx_eq!(data.sample_linear(0.9, 0.2), 110.0, EPSILON);
        assert_approx_eq!(data.sample_linear(0.35339, 0.29234), 64.573, EPSILON);
    }
}
