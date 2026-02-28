use game_egui::egui::{Context, Window};

#[derive(Clone, Debug, Default)]
pub struct DebugState {
    pub draw_colliders: bool,
    pub draw_translations: bool,
    pub draw_cell_boundaries: bool,
    pub draw_skeletons: bool,
    pub draw_light_markers: bool,
    pub draw_hitscan_marker: bool,
    pub draw_doors: bool,
    pub draw_trigger_boxes: bool,
    pub draw_navmesh: bool,
    pub draw_gpu_memory_visualizer: bool,
}

pub fn draw_debug_ui(ctx: &Context, state: &mut DebugState) {
    Window::new("Debug").show(ctx, |ui| {
        ui.checkbox(
            &mut state.draw_gpu_memory_visualizer,
            "Show GPU Memory Visualizer",
        );
        ui.checkbox(&mut state.draw_colliders, "Draw Colliders");
        ui.checkbox(&mut state.draw_translations, "Draw Entity Translations");
        ui.checkbox(&mut state.draw_cell_boundaries, "Draw Cell Boundaries");
        ui.checkbox(&mut state.draw_skeletons, "Draw Skeletons");
        ui.checkbox(&mut state.draw_light_markers, "Draw Light Markers");
        ui.checkbox(&mut state.draw_hitscan_marker, "Draw Hitscan Markers");
        ui.checkbox(&mut state.draw_doors, "Draw Door Markers");
        ui.checkbox(&mut state.draw_trigger_boxes, "Draw Trigger Boxes");
        ui.checkbox(&mut state.draw_navmesh, "Draw Navmesh");
    });
}
