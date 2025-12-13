use std::fmt::Write;

use game_egui::egui::{Context, Window};
use hecs::World;

use crate::components::Player;
use crate::systems::inventory::Inventory;

#[derive(Clone, Debug, Default)]
pub struct InventoryState {}

pub fn draw_inventory(ctx: &Context, state: &mut InventoryState, world: &World) {
    let mut query = world.query::<(&Player, &Inventory)>();
    let Some((_, (_, inventory))) = query.iter().next() else {
        return;
    };

    Window::new("Inventory").show(ctx, |ui| {
        for (_, item) in inventory.items() {
            if item.hidden {
                continue;
            }

            let mut text = item.name.clone();
            if item.count != 1 {
                write!(text, " ({})", item.count).unwrap();
            }

            ui.label(text);
        }
    });
}
