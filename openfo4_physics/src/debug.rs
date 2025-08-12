use game_common::components::Color;
use game_gizmos::Gizmos;
use rapier3d::math::Point;
use rapier3d::pipeline::{DebugColor, DebugRenderBackend, DebugRenderObject};

#[derive(Copy, Clone, Debug)]
pub struct GizmoRenderBackend<'a>(pub &'a Gizmos);

impl DebugRenderBackend for GizmoRenderBackend<'_> {
    fn draw_line(
        &mut self,
        _object: DebugRenderObject<'_>,
        a: Point<f32>,
        b: Point<f32>,
        color: DebugColor,
    ) {
        self.0.line(a.into(), b.into(), Color(color));
    }
}
