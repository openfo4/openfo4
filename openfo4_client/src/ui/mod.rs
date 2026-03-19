use game_egui::egui::Context;
use hecs::World;
use openfo4_core::events::EventBus;

use crate::ui::console::ConsoleState;
use crate::ui::debug::DebugState;
use crate::ui::hud::HudState;
use crate::ui::inventory::InventoryState;

pub mod console;
pub mod debug;
pub mod hud;
pub mod inventory;

#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub debug: DebugState,
    pub inventory: InventoryState,
    pub hud: HudState,
    console: ConsoleState,
}

pub fn draw_ui(ctx: &Context, state: &mut UiState, world: &World, events: &mut EventBus) {
    debug::draw_debug_ui(ctx, &mut state.debug);
    hud::draw_hud(ctx, &mut state.hud, world, events);
    inventory::draw_inventory(ctx, &mut state.inventory, world, events);
    console::draw_console(ctx, &mut state.console, world, events);
}
