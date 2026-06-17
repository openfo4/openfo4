use std::ops::Mul;

use bytemuck::{Pod, Zeroable};
use glam::FloatExt;

#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::from_rgba([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Self = Self::from_rgba([0.0, 0.0, 0.0, 1.0]);

    pub const RED: Self = Self::from_rgba([1.0, 0.0, 0.0, 1.0]);
    pub const GREEN: Self = Self::from_rgba([0.0, 1.0, 0.0, 1.0]);
    pub const BLUE: Self = Self::from_rgba([0.0, 0.0, 1.0, 1.0]);

    pub const YELLOW: Self = Self::from_rgba([1.0, 1.0, 0.0, 1.0]);
    pub const AQUA: Self = Self::from_rgba([0.0, 1.0, 1.0, 1.0]);
    pub const FUCHSIA: Self = Self::from_rgba([1.0, 0.0, 1.0, 1.0]);

    pub const TRANSPARENT: Self = Self::from_rgba([0.0; 4]);

    #[inline]
    pub const fn as_rgb(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    #[inline]
    pub const fn as_rgba(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline]
    pub const fn from_rgb(rgb: [f32; 3]) -> Self {
        let [r, g, b] = rgb;
        Self { r, g, b, a: 1.0 }
    }

    #[inline]
    pub const fn from_rgba(rgba: [f32; 4]) -> Self {
        let [r, g, b, a] = rgba;
        Self { r, g, b, a }
    }

    pub fn lerp(self, other: Self, s: f32) -> Self {
        let a = self;
        let b = other;

        Self {
            r: f32::lerp(a.r, b.r, s),
            g: f32::lerp(a.g, b.g, s),
            b: f32::lerp(a.b, b.b, s),
            a: f32::lerp(a.a, b.a, s),
        }
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let a = self;
        let b = rhs;

        Self {
            r: a.r * b.r,
            g: a.g * b.g,
            b: a.b * b.b,
            a: a.a * b.a,
        }
    }
}

impl Mul<f32> for Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}
#[derive(Copy, Clone, Debug)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    /// Inner cutoff angle
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct AmbientLight {
    pub top: Color,
    pub bottom: Color,
    pub left: Color,
    pub right: Color,
    pub front: Color,
    pub back: Color,
    pub brightness: f32,
}
