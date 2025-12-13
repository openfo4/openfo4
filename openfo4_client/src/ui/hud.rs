use game_egui::egui::{Color32, Context, CornerRadius, LayerId, Painter, Pos2, Rect};

const HP_HEIGHT: f32 = 10.0;
const HP_WIDTH: f32 = 200.0;

const STROKE_SIZE: f32 = 2.0;

const CROSSHAIR_WIDTH: f32 = 40.0;
const CROSSHAIR_HEIGHT: f32 = 40.0;

pub fn draw_hud(ctx: &Context) {
    let painter = ctx.layer_painter(LayerId::background());
    let viewport = ctx.content_rect();

    draw_crosshair(&painter, viewport);
    draw_health(&painter, viewport);
}

fn draw_crosshair(painter: &Painter, viewport: Rect) {
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

fn draw_health(painter: &Painter, viewport: Rect) {
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

    draw_outline(&painter, outer_area);

    let health_factor = 0.7;
    let rads_factor = 0.3;

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
