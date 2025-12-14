use std::fmt::{Debug, Write};

use game_egui::egui::{Context, RichText, Window};
use hecs::World;
use openfo4_core::events::EventBus;

use crate::components::Player;
use crate::systems::inventory::{EquipItemEvent, Inventory, UnequipItemEvent};

#[derive(Clone, Debug, Default)]
pub struct InventoryState {}

pub fn draw_inventory(
    ctx: &Context,
    state: &mut InventoryState,
    world: &World,
    events: &mut EventBus,
) {
    let mut query = world.query::<(&Player, &Inventory)>();
    let Some((player, (_, inventory))) = query.iter().next() else {
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

            if ui.button(RichText::new(text)).clicked() {
                if item.equipped {
                    events.send(UnequipItemEvent {
                        actor: player,
                        item: item.item,
                    });
                } else {
                    events.send(EquipItemEvent {
                        actor: player,
                        item: item.item,
                    });
                }
            }
        }
    });
}
