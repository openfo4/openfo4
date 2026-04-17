use game_egui::egui::{Context, Key, Window};
use hecs::{Entity, World};
use openfo4_core::events::EventBus;

use crate::components::Player;
use crate::systems::inventory::AddItemEventAsync;
use crate::systems::time::GameTime;

#[derive(Clone, Debug, Default)]
pub struct ConsoleState {
    open: bool,
    text: String,
    response: String,
}

pub fn draw_console(ctx: &Context, state: &mut ConsoleState, world: &World, events: &mut EventBus) {
    Window::new("Console").show(ctx, |ui| {
        ui.text_edit_singleline(&mut state.text);

        ui.label(&state.response);

        if ui.input(|state| state.key_down(Key::Enter)) {
            if let Some(resp) = execute_command(&state.text, world, events) {
                state.response = resp;
            }
        }
    });
}

fn execute_command(cmd: &str, world: &World, events: &mut EventBus) -> Option<String> {
    let tokens: Vec<&str> = cmd.split(" ").filter(|v| !v.is_empty()).collect();

    match tokens.get(0).copied()? {
        "set" => {
            let Some(global) = tokens.get(1).copied() else {
                return Some("missing parameter: global".to_owned());
            };

            let Some(value) = tokens.get(2).copied() else {
                return Some("missing parameter: value".to_owned());
            };

            let value = match value.parse::<f32>() {
                Ok(value) => value,
                Err(err) => return Some(format!("invalid value: {}", err)),
            };

            if global.eq_ignore_ascii_case("GameYear")
                || global.eq_ignore_ascii_case("GameMonth")
                || global.eq_ignore_ascii_case("GameDay")
                || global.eq_ignore_ascii_case("GameHour")
            {
                for time in world.query::<&mut GameTime>().iter() {
                    if global.eq_ignore_ascii_case("GameYear") {
                        todo!()
                    } else if global.eq_ignore_ascii_case("GameMonth") {
                        todo!()
                    } else if global.eq_ignore_ascii_case("GameDay") {
                        todo!()
                    } else {
                        debug_assert!(global.eq_ignore_ascii_case("GameHour"));
                        let hour = (value as u8).clamp(0, 23);
                        time.set_hour(hour);
                    }
                }

                return None;
            }

            Some(format!("unknown global: {}", global))
        }
        "player.additem" => {
            let Some(item) = tokens.get(1).copied() else {
                return Some("missing parameter: item".to_owned());
            };

            let item = match item.parse() {
                Ok(item) => item,
                Err(err) => {
                    return Some(format!("invalid item: {}", err));
                }
            };

            let Some(player) = get_player(world) else {
                return None;
            };

            events.send(AddItemEventAsync {
                actor: player,
                item,
                count: 1,
                attachments: Vec::new(),
                hidden: false,
                equip: false,
            });

            None
        }
        v => Some(format!("unknown command: {}", v)),
    }
}

fn get_player(world: &World) -> Option<Entity> {
    world
        .query::<(Entity, &Player)>()
        .iter()
        .next()
        .map(|(entity, _)| entity)
}
