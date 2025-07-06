use glam::{IVec2, UVec2, Vec2};

use crate::components::NiTransform;

pub const CELL_SIZE: UVec2 = UVec2::splat(4096);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellId(pub IVec2);

impl CellId {
    pub fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    pub fn min(&self) -> Vec2 {
        self.0.as_vec2() * CELL_SIZE.as_vec2()
    }

    pub fn max(&self) -> Vec2 {
        self.min() + CELL_SIZE.as_vec2()
    }

    pub fn square(&self, distance: u32) -> Vec<CellId> {
        let distance = distance as i32;
        let mut cells = Vec::new();
        for x in -distance..=distance {
            for y in -distance..=distance {
                cells.push(Self(IVec2::new(self.0.x + x, self.0.y + y)));
            }
        }
        cells
    }
}

impl From<NiTransform> for CellId {
    fn from(value: NiTransform) -> Self {
        Self::from(value.translation().truncate())
    }
}

impl From<Vec2> for CellId {
    fn from(coords: Vec2) -> Self {
        let x = if coords.x.is_sign_negative() {
            if coords.x % CELL_SIZE.x as f32 == 0.0 {
                (coords.x / CELL_SIZE.x as f32) as i32
            } else {
                (coords.x / CELL_SIZE.x as f32) as i32 - 1
            }
        } else {
            (coords.x / CELL_SIZE.x as f32) as i32
        };

        let y = if coords.y.is_sign_negative() {
            if coords.y % CELL_SIZE.y as f32 == 0.0 {
                (coords.y / CELL_SIZE.y as f32) as i32
            } else {
                (coords.y / CELL_SIZE.y as f32) as i32 - 1
            }
        } else {
            (coords.y / CELL_SIZE.y as f32) as i32
        };

        Self(IVec2 { x, y })
    }
}
