use glam::Vec3;
use openfo4_common::components::Color;
use openfo4_gizmos::Gizmos;
use rapier3d::pipeline::{DebugColor, DebugRenderBackend, DebugRenderObject};

#[derive(Copy, Clone, Debug)]
pub struct GizmoRenderBackend<'a>(pub &'a Gizmos);

impl DebugRenderBackend for GizmoRenderBackend<'_> {
    fn draw_line(&mut self, _object: DebugRenderObject<'_>, a: Vec3, b: Vec3, color: DebugColor) {
        self.0.line(a, b, Color::from_rgba(color));
    }
}
