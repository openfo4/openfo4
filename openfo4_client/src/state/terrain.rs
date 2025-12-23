use std::ops::Deref;

use game_tracing::trace_span;
use glam::{FloatExt, UVec2, Vec2, Vec3A, Vec3Swizzles, Vec4};
use image::{ImageBuffer, Pixel, Rgba};

const UV_SCALE: Vec2 = Vec2::splat(8.0);

#[derive(Clone, Debug)]
pub struct TerrainLayer {
    pub position: UVec2,
    pub size: UVec2,
    pub layer: i16,
    pub alpha: [[f32; 17]; 17],
    pub diffuse_texture: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub normal_texture: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub smooth_spec_texture: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub smoothness: f32,
    pub specular_strength: f32,
}

#[derive(Clone, Debug)]
pub struct Terrain {
    pub diffuse_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub normal_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
    pub smooth_spec_texture: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

pub fn combine_texture_layers(layers: &[TerrainLayer], size: UVec2) -> Terrain {
    let _span = trace_span!("combine_texture_layers").entered();

    let mut diffuse_texture = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(size.x, size.y);
    let mut normal_texture = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(size.x, size.y);
    let mut smooth_spec_texture = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(size.x, size.y);

    for layer in layers {
        let Some(texture) = &layer.diffuse_texture else {
            continue;
        };

        if texture.width() == 0 || texture.height() == 0 {
            continue;
        }

        for y in layer.position.y..layer.position.y + layer.size.y {
            for x in layer.position.x..layer.position.x + layer.size.x {
                let u = x as f32 / layer.size.x as f32;
                let v = y as f32 / layer.size.y as f32;

                let alpha = layer.alpha.sample_linear(u, v);
                let mut color = texture.sample_linear(u * UV_SCALE.x, v * UV_SCALE.y);

                color.0[3] = ((color.0[3] as f32 / 255.0) * alpha * 255.0) as u8;

                let dst = diffuse_texture.get_pixel_mut(x, y);
                dst.blend(&color);
            }
        }
    }

    for layer in layers {
        let Some(texture) = &layer.normal_texture else {
            continue;
        };

        if texture.width() == 0 || texture.height() == 0 {
            continue;
        }

        for y in layer.position.y..layer.position.y + layer.size.y {
            for x in layer.position.x..layer.position.x + layer.size.x {
                let u = x as f32 / layer.size.x as f32;
                let v = y as f32 / layer.size.y as f32;

                let alpha = layer.alpha.sample_linear(u, v);
                let src_normal = texture.sample_linear(u * UV_SCALE.x, v * UV_SCALE.y);
                let dst_normal = *normal_texture.get_pixel(x, y);

                let mut src =
                    Vec3A::new(src_normal.0[0] as f32, src_normal.0[1] as f32, 0.0) / 255.0;
                let mut dst =
                    Vec3A::new(dst_normal.0[0] as f32, dst_normal.0[1] as f32, 0.0) / 255.0;

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

                *normal_texture.get_pixel_mut(x, y) = out;
            }
        }
    }

    for layer in layers {
        let Some(texture) = &layer.smooth_spec_texture else {
            continue;
        };

        if texture.width() == 0 || texture.height() == 0 {
            continue;
        }

        for y in layer.position.y..layer.position.y + layer.size.y {
            for x in layer.position.x..layer.position.x + layer.size.x {
                let u = x as f32 / layer.size.x as f32;
                let v = y as f32 / layer.size.y as f32;

                let alpha = layer.alpha.sample_linear(u, v);
                let mut gloss_spec = texture.sample_linear(u * UV_SCALE.x, v * UV_SCALE.y);

                // Scale the glossiness/specular texture with the material factors.
                // Specular strength in red channel, glossiness in green channel.
                gloss_spec.0[0] = (gloss_spec.0[0] as f32 * layer.specular_strength) as u8;
                gloss_spec.0[1] = (gloss_spec.0[1] as f32 * layer.smoothness) as u8;

                let dst = smooth_spec_texture.get_pixel_mut(x, y);
                *dst = lerp_pixel(*dst, gloss_spec, alpha);
            }
        }
    }

    Terrain {
        diffuse_texture,
        normal_texture,
        smooth_spec_texture,
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
