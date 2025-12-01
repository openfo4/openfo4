use game_ui::runtime::Context;
use game_ui::widgets::{Container, Text, Widget};
use hecs::World;

use crate::components::Player;
use crate::systems::inventory::Inventory;

pub fn draw_inventory(ctx: &Context, world: &World) {
    let mut query = world.query::<(&Player, &Inventory)>();
    let Some((_, (_, inventory))) = query.iter().next() else {
        return;
    };

    let root = Container::default().mount(ctx);

    for (_, item) in inventory.items() {
        if item.hidden {
            continue;
        }

        Text::new(format!("{} ({}) {}", item.name, item.count, item.equipped)).mount(&root);
    }
}
