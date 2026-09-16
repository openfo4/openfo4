use game_egui::egui::Context;

use crate::state::GameState;
use crate::ui::console::ConsoleState;
use crate::ui::debug::DebugState;
use crate::ui::hud::HudState;
use crate::ui::inventory::InventoryState;

pub mod console;
pub mod debug;
pub mod hud;
pub mod inventory;
mod scripting;

#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub debug: DebugState,
    pub inventory: InventoryState,
    pub hud: HudState,
    console: ConsoleState,
    scripting: scripting::State,
}

pub fn draw_ui(ctx: &Context, state: &mut UiState, game: &mut GameState) {
    debug::draw_debug_ui(ctx, &mut state.debug);
    hud::draw_hud(ctx, &mut state.hud, &game.world, &mut game.event_bus);
    inventory::draw_inventory(ctx, &mut state.inventory, &game.world, &mut game.event_bus);
    console::draw_console(ctx, &mut state.console, &game.world, &mut game.event_bus);

    if state.debug.scripting_objects_open {
        scripting::draw_objects(ctx, game, &mut state.scripting);
    }
}
