pub mod camera;
pub mod entities;
pub mod lights;
pub mod material;
pub mod sky;
pub mod terrain;

mod passes;

pub use passes::FINAL_RENDER_PASS;

use entities::Entities;
use material::StandardMaterial;
use openfo4_render::Renderer;

pub fn init(renderer: &mut Renderer) -> Entities {
    let (entities, rx, arx) = Entities::new();

    renderer.with_command_queue_and_graph(|graph, queue| {
        passes::init(
            graph,
            queue,
            rx,
            arx,
            entities.sky.clone(),
            entities.new_lut.clone(),
        );
    });

    entities
}
