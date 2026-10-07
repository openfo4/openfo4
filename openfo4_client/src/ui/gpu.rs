use game_egui::egui::{
    Align2, Area, Color32, ColorImage, Context, Label, Painter, Pos2, Rect, RichText, Sense,
    TextureOptions, Vec2,
};
use game_render::Renderer;
use game_render::statistics::{AllocationKind, MemoryBlock};

const BLOCK_HEIGHT: u32 = 32;
const BLOCK_WIDTH: u32 = 512;

const COLOR_UNUSED: Color32 = Color32::LIGHT_GRAY;
const COLOR_BUFFER: Color32 = Color32::RED;
const COLOR_TEXTURE: Color32 = Color32::BLUE;

pub fn draw_gpu_memory_visualizer(ctx: &Context, renderer: &Renderer) {
    let mem = renderer.statistics().memory.read();

    Area::new("gpu_memory_visualizer".into())
        .anchor(Align2::LEFT_TOP, Vec2::ZERO)
        .sense(Sense::empty())
        .interactable(false)
        .movable(false)
        .show(ctx, |ui| {
            for (_, block) in mem.blocks.iter() {
                let mut text = format!(
                    "Size: {} Used: {} Usage Ratio: {:.2} Allocations: {}",
                    bytes_to_human_readable(block.size),
                    bytes_to_human_readable(block.used),
                    block.used as f64 / block.size as f64,
                    block.allocs.len()
                );

                if block.device_local {
                    text.push_str(" DEVICE_LOCAL");
                }

                if block.host_visible {
                    text.push_str(" HOST_VISIBLE");
                }

                if block.dedicated {
                    text.push_str(" DEDICATED");
                }

                let text = RichText::new(text)
                    .background_color(Color32::BLACK)
                    .color(Color32::WHITE);
                ui.add(Label::new(text).selectable(false).sense(Sense::empty()));

                let (_, painter) = ui.allocate_painter(
                    Vec2::new(BLOCK_WIDTH as f32, BLOCK_HEIGHT as f32),
                    Sense::empty(),
                );

                let img = draw_block(&painter, block);
                let img = ctx.load_texture("", img, TextureOptions::NEAREST);

                painter.image(
                    img.id(),
                    painter.clip_rect(),
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        });
}

fn draw_block(painter: &Painter, block: &MemoryBlock) -> ColorImage {
    let block_px = BLOCK_HEIGHT as usize * BLOCK_WIDTH as usize;
    let bytes_per_px = block.size.div_ceil(block_px);

    let num_pixels = BLOCK_HEIGHT * BLOCK_WIDTH;
    let mut pixels = (0..num_pixels).map(|_| COLOR_UNUSED).collect::<Vec<_>>();

    painter.rect_filled(painter.clip_rect(), 0.0, COLOR_UNUSED);

    for alloc in block.allocs.values() {
        let start_px = alloc.offset / bytes_per_px;
        let end_px = (alloc.offset + alloc.size).div_ceil(bytes_per_px);

        for px in start_px as u32..end_px as u32 {
            let x = px / BLOCK_HEIGHT;
            let y = px % BLOCK_HEIGHT;

            let color = match alloc.kind {
                AllocationKind::Buffer => COLOR_BUFFER,
                AllocationKind::Texture => COLOR_TEXTURE,
            };

            let index = y * BLOCK_WIDTH + x;
            pixels[index as usize] = color;
        }
    }

    ColorImage {
        size: [BLOCK_WIDTH as usize, BLOCK_HEIGHT as usize],
        source_size: Vec2::new(BLOCK_WIDTH as f32, BLOCK_HEIGHT as f32),
        pixels,
    }
}

fn bytes_to_human_readable(mut bytes: usize) -> String {
    for unit in ["", "KiB"] {
        if bytes < 1024 {
            return format!("{} {}", bytes, unit);
        }

        bytes /= 1024;
    }

    format!("{} MiB", bytes)
}
