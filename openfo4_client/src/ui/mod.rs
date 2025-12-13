use game_egui::egui::Context;

use crate::ui::debug::DebugState;

pub mod debug;
pub mod inventory;

#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub debug: DebugState,
}

pub fn draw_ui(ctx: &Context, state: &mut UiState) {
    debug::draw_debug_ui(ctx, &mut state.debug);
}
