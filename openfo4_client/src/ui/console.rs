use std::fmt::{self, Display, Formatter};

use futures_lite::future::block_on;
use openfo4_common::components::Transform;
use openfo4_common::string::Lowercase;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::record::RecordId;
use openfo4_egui::egui::{Context, Key, Window};
use openfo4_esp::{RecordData, RecordKind};

use crate::cell::CellId;
use crate::components::{NiTransform, Player};
use crate::state::GameState;
use crate::systems::inventory::AddItemEventAsync;
use crate::systems::time::GameTime;

#[derive(Clone, Debug, Default)]
pub struct ConsoleState {
    open: bool,
    text: String,
    response: String,
}

pub fn draw_console(ctx: &Context, state: &mut ConsoleState, game: &mut GameState) {
    Window::new("Console").show(ctx, |ui| {
        ui.text_edit_singleline(&mut state.text);

        ui.label(&state.response);

        if ui.input(|state| state.key_down(Key::Enter)) {
            if let Some(resp) = execute_command(&state.text, game) {
                state.response = resp;
            }
        }
    });
}

fn execute_command(cmd: &str, game: &mut GameState) -> Option<String> {
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
                for mut time in game.world.query::<&mut GameTime>().iter() {
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

            let Some(player) = get_player(&game.world) else {
                return None;
            };

            game.event_bus.send(AddItemEventAsync {
                actor: player,
                item,
                count: 1,
                attachments: Vec::new(),
                hidden: false,
                equip: false,
            });

            None
        }
        "CenterOnCell" | "coc" => {
            let Some(cell) = tokens.get(1).copied() else {
                return Some("missing cell name".to_owned());
            };

            let cell = match cell.parse::<RecordId>() {
                Ok(id) => IdOrName::Id(id),
                Err(_) => IdOrName::Name(cell),
            };

            match block_on(center_on_cell(game, cell)) {
                Ok(()) => None,
                Err(err) => Some(format!("cannot center on cell {cell}: {err}")),
            }
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum IdOrName<'a> {
    Id(RecordId),
    Name(&'a str),
}

impl Display for IdOrName<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => Display::fmt(id, f),
            Self::Name(name) => Display::fmt(name, f),
        }
    }
}

async fn center_on_cell(
    game: &mut GameState,
    cell: IdOrName<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cell = match cell {
        IdOrName::Id(id) => game.esp_loader.lookup(Some(RecordKind::Cell), id).await?,
        IdOrName::Name(name) => {
            // Match the name case-insensitive.
            let name = Lowercase::new(name);

            let cells = game.esp_loader.lookup_all(Some(RecordKind::Cell)).await?;

            cells
                .into_iter()
                .find(|record| match &record.local.data {
                    RecordData::Cell(cell) => cell
                        .id
                        .as_ref()
                        .and_then(|s| s.as_str())
                        .is_some_and(|s| Lowercase::new(s) == name),
                    _ => false,
                })
                .ok_or_else(|| format!("unable to find cell with name {name}"))?
        }
    };

    let RecordData::Cell(cell) = cell.local.data else {
        return Err(format!("record {} is not a cell", cell.id).into());
    };

    let grid = cell.grid.unwrap();
    let cell = CellId::new(grid.x, grid.y);
    let transform = NiTransform::from_translation(cell.center().extend(8096.0));
    let transform = Transform::from(transform);

    for (_, mut t) in game.world.query_mut::<(&Player, &mut Transform)>() {
        *t = transform;
    }

    Ok(())
}
