use std::fmt::Write;

use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_egui::egui::{
    Color32, Context, CornerRadius, Frame, Key, LayerId, Painter, Pos2, Rect, RichText, Window,
};

use crate::components::{ActorMarker, DeadActor, LookingAtTarget, PlacedItem, Player, RootParent};
use crate::state::hardcoded::fallout4::ACTOR_VALUE_HEALTH;
use crate::systems::actor_value::ActorValueProperties;
use crate::systems::inventory::Inventory;
use crate::systems::{PickupItemEvent, TransferItemEvent};

const HP_HEIGHT: f32 = 10.0;
const HP_WIDTH: f32 = 200.0;

const STROKE_SIZE: f32 = 2.0;

const CROSSHAIR_WIDTH: f32 = 40.0;
const CROSSHAIR_HEIGHT: f32 = 40.0;

#[derive(Clone, Debug, Default)]
pub struct HudState {
    quickloot: QuicklootState,
}

#[derive(Clone, Debug, Default)]
struct QuicklootState {
    target: Option<Entity>,
    index: usize,
}

pub fn draw_hud(ctx: &Context, state: &mut HudState, world: &World, events: &mut EventBus) {
    let painter = ctx.layer_painter(LayerId::background());
    let viewport = ctx.content_rect();

    draw_crosshair(&painter, viewport, world);
    draw_health(&painter, viewport, world);
    draw_quickloot_menu(ctx, viewport, world, events, &mut state.quickloot);
    draw_pickup_menu(ctx, world, events);
    draw_actor_interaction(ctx, world);
}

fn draw_crosshair(painter: &Painter, viewport: Rect, world: &World) {
    draw_interaction_crosshair(painter, viewport);
}

fn draw_interaction_crosshair(painter: &Painter, viewport: Rect) {
    let area = Rect {
        min: Pos2 {
            x: (viewport.size().x - CROSSHAIR_WIDTH) / 2.0,
            y: (viewport.size().y - CROSSHAIR_HEIGHT) / 2.0,
        },
        max: Pos2 {
            x: (viewport.size().x + CROSSHAIR_WIDTH) / 2.0,
            y: (viewport.size().y + CROSSHAIR_WIDTH) / 2.0,
        },
    };

    for rect in [
        // Top Left
        Rect {
            min: area.min,
            max: Pos2 {
                x: area.min.x + STROKE_SIZE,
                y: area.min.y + area.size().y / 3.0,
            },
        },
        Rect {
            min: area.min,
            max: Pos2 {
                x: area.min.x + area.size().x / 3.0,
                y: area.min.y + STROKE_SIZE,
            },
        },
        // Top Right
        Rect {
            min: Pos2 {
                x: area.max.x - area.size().x / 3.0,
                y: area.min.y,
            },
            max: Pos2 {
                x: area.max.x,
                y: area.min.y + STROKE_SIZE,
            },
        },
        Rect {
            min: Pos2 {
                x: area.max.x - STROKE_SIZE,
                y: area.min.y,
            },
            max: Pos2 {
                x: area.max.x,
                y: area.min.y + area.size().y / 3.0,
            },
        },
        // Bottom Left
        Rect {
            min: Pos2 {
                x: area.min.x,
                y: area.max.y - area.size().y / 3.0,
            },
            max: Pos2 {
                x: area.min.x + STROKE_SIZE,
                y: area.max.y,
            },
        },
        Rect {
            min: Pos2 {
                x: area.min.x,
                y: area.max.y - STROKE_SIZE,
            },
            max: Pos2 {
                x: area.min.x + area.size().x / 3.0,
                y: area.max.y,
            },
        },
        // Bottom Right
        Rect {
            min: Pos2 {
                x: area.max.x - area.size().x / 3.0,
                y: area.max.y - STROKE_SIZE,
            },
            max: area.max,
        },
        Rect {
            min: Pos2 {
                x: area.max.x - STROKE_SIZE,
                y: area.max.y - area.size().y / 3.0,
            },
            max: area.max,
        },
    ] {
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::GREEN);
    }
}

fn draw_health(painter: &Painter, viewport: Rect, world: &World) {
    let outer_area = Rect {
        min: Pos2 {
            x: viewport.min.x + 100.0,
            y: viewport.max.y - 100.0,
        },
        max: Pos2 {
            x: viewport.min.x + 100.0 + HP_WIDTH,
            y: viewport.max.y - 100.0 + HP_HEIGHT,
        },
    };
    let inner_area = Rect {
        min: Pos2 {
            x: outer_area.min.x + STROKE_SIZE,
            y: outer_area.min.y + STROKE_SIZE,
        },
        max: Pos2 {
            x: outer_area.max.x - STROKE_SIZE,
            y: outer_area.max.y - STROKE_SIZE,
        },
    };

    let mut iter = world.query::<(&Player, &ActorValueProperties)>();
    let Some((_, props)) = iter.iter().next() else {
        return;
    };

    let health = props.get(ACTOR_VALUE_HEALTH).unwrap();
    let health_factor = health.damaged / health.value;

    let rads_factor = 0.0;

    draw_outline(&painter, outer_area);

    painter.rect_filled(
        Rect {
            min: inner_area.min,
            max: Pos2 {
                x: inner_area.min.x + inner_area.size().x * health_factor,
                y: inner_area.max.y,
            },
        },
        CornerRadius::ZERO,
        Color32::GREEN,
    );

    painter.rect_filled(
        Rect {
            min: Pos2 {
                x: inner_area.max.x - inner_area.size().x * rads_factor,
                y: inner_area.min.y,
            },
            max: inner_area.max,
        },
        CornerRadius::ZERO,
        Color32::RED,
    );
}

fn draw_outline(painter: &Painter, area: Rect) {
    // Left
    painter.rect_filled(
        Rect {
            min: Pos2 {
                x: area.min.x,
                y: area.min.y,
            },
            max: Pos2 {
                x: area.min.x + STROKE_SIZE,
                y: area.max.y,
            },
        },
        CornerRadius::ZERO,
        Color32::GREEN,
    );

    // Bottom
    painter.rect_filled(
        Rect {
            min: Pos2 {
                x: area.min.x,
                y: area.max.y - STROKE_SIZE,
            },
            max: Pos2 {
                x: area.max.x,
                y: area.max.y,
            },
        },
        CornerRadius::ZERO,
        Color32::GREEN,
    );

    // Right
    painter.rect_filled(
        Rect {
            min: Pos2 {
                x: area.max.x - STROKE_SIZE,
                y: area.min.y,
            },
            max: Pos2 {
                x: area.max.x,
                y: area.max.y,
            },
        },
        CornerRadius::ZERO,
        Color32::GREEN,
    );
}

fn draw_quickloot_menu(
    ctx: &Context,
    viewport: Rect,
    world: &World,
    events: &mut EventBus,
    state: &mut QuicklootState,
) {
    let Some((player, target)) = world
        .query::<(Entity, &LookingAtTarget, &Player)>()
        .iter()
        .map(|(p, v, _)| (p, *v))
        .next()
    else {
        return;
    };

    let Ok(root_parent) = world.get::<&RootParent>(target.0) else {
        return;
    };
    let target = root_parent.0;

    let Ok(target_entity) = world.entity(target) else {
        return;
    };

    // Don't allow looting from actors that are alive.
    if target_entity.has::<ActorMarker>() && !target_entity.has::<DeadActor>() {
        return;
    }

    let Some(inventory) = target_entity.get::<&Inventory>() else {
        return;
    };

    if state.target != Some(target) {
        state.index = 0;
    }
    state.target = Some(target);

    let mut transfer_current_item = false;
    ctx.input(|input| {
        if input.raw_scroll_delta.y > 0.0 {
            state.index = state.index.saturating_add(1);
        } else if input.raw_scroll_delta.y < 0.0 {
            state.index = state.index.saturating_sub(1);
        }

        transfer_current_item = input.key_pressed(Key::F);
    });

    Window::new("Quickloot")
        .frame(Frame::NONE)
        .collapsible(false)
        .interactable(false)
        .resizable(false)
        .movable(false)
        .title_bar(false)
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.label("CONTAINER");

            // When index is out of bounds move it to the last item.
            state.index = usize::min(
                state.index,
                inventory
                    .items()
                    .filter(|(_, item)| !item.hidden)
                    .count()
                    .saturating_sub(1),
            );

            for (index, (key, item)) in inventory
                .items()
                .filter(|(_, item)| !item.hidden)
                .enumerate()
            {
                let mut name = item.name.clone();
                if item.count != 1 {
                    write!(name, " ({})", item.count).unwrap();
                }

                let mut text = RichText::new(name);
                if state.index == index {
                    text = text.background_color(Color32::GREEN).color(Color32::BLACK);
                } else {
                    text = text.color(Color32::GREEN);
                }

                if transfer_current_item && index == state.index {
                    events.send(TransferItemEvent {
                        source: target,
                        destination: player,
                        item: key,
                        count: item.count,
                    });
                }

                ui.label(text);
            }
        });
}

fn draw_pickup_menu(ctx: &Context, world: &World, events: &mut EventBus) {
    let Some((player, target)) = world
        .query::<(Entity, &LookingAtTarget, &Player)>()
        .iter()
        .map(|(p, v, _)| (p, *v))
        .next()
    else {
        return;
    };

    let Ok(root_parent) = world.get::<&RootParent>(target.0) else {
        return;
    };

    let target = root_parent.0;
    let Ok(item) = world.get::<&PlacedItem>(target) else {
        return;
    };

    let pickup_item = ctx.input(|input| input.key_down(Key::F));

    Window::new("Pickup")
        .frame(Frame::NONE)
        .collapsible(false)
        .interactable(false)
        .resizable(false)
        .movable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.label("PICKUP");

            ui.label(RichText::new(item.0.name.clone()).color(Color32::GREEN));

            if pickup_item {
                events.send(PickupItemEvent {
                    actor: player,
                    item: target,
                });
            }
        });
}

fn draw_actor_interaction(ctx: &Context, world: &World) {
    let Some((_, target)) = world
        .query::<(Entity, &LookingAtTarget, &Player)>()
        .iter()
        .map(|(p, v, _)| (p, *v))
        .next()
    else {
        return;
    };

    let Ok(root_parent) = world.get::<&RootParent>(target.0) else {
        return;
    };
    let target = root_parent.0;

    let Ok(target_entity) = world.entity(target) else {
        return;
    };

    if !target_entity.has::<ActorMarker>() || target_entity.has::<DeadActor>() {
        return;
    }

    Window::new("Talk").show(ctx, |ui| {
        ui.label("Talk");
    });
}
