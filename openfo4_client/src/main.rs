mod assets;
mod cell;
mod components;
mod events;
mod loader;
mod render;
mod shared;
mod state;
mod systems;

use std::num::NonZeroUsize;
use std::sync::Arc;

use game_common::components::{Color, Transform};
use game_core::time::Time;
use game_crash_handler::main;
use game_gizmos::Gizmos;
use game_input::keyboard::KeyCode;
use game_render::Renderer;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::{CameraId, Object, SceneId};
use game_render::light::DirectionalLight;
use game_render::mesh::{Indices, Mesh};
use game_render::pbr::PbrMaterial;
use game_tasks::TaskPool;
use game_ui::debug_widgets::gpu_timings::GpuTimings;
use game_ui::debug_widgets::memory_visualizer::MemoryVisualizer;
use game_ui::runtime::DocumentId;
use game_ui::widgets::Widget;
use game_ui::{UiState, WindowProperties};
use game_window::events::WindowEvent;
use game_window::windows::{Window, WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::{UVec2, Vec3};
use openfo4_nif::{Block, Nif};
use render::RenderState;
use render::physics_debug::{
    draw_cell_boundaries, draw_colliers, draw_skeletons, draw_translations,
};
use shared::SharedState;
use state::GameState;

#[main]
fn main() {
    game_core::logger::init();

    let num_threads = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
    let pool = Arc::new(TaskPool::new(num_threads.get()));

    let mut wm = WindowManager::new();
    let window = wm.windows_mut().spawn(WindowBuilder::new());

    let mut renderer = Renderer::new().unwrap();
    let entities = game_core_pipeline::init(&mut renderer);
    let gizmos = Gizmos::new(&mut renderer);

    let shared = Arc::new(SharedState::new(entities.clone()));

    let ui = UiState::new(&mut renderer);
    let mut game = GameState::new(shared.clone(), pool);
    game.init_camera();
    let state = RenderState::new(
        renderer,
        RenderTarget::Window(window),
        shared,
        entities,
        gizmos,
    );

    let app = App {
        game,
        state,
        ui,
        doc: None,
        window,
        size: UVec2::ZERO,
        debug_options: DebugOptions::default(),
        time: Time::new(),
    };

    wm.run(app);
}

struct App {
    game: GameState,
    state: RenderState,
    ui: UiState,
    doc: Option<DocumentId>,
    window: WindowId,
    size: UVec2,
    debug_options: DebugOptions,
    time: Time,
}

impl game_window::App for App {
    fn handle_event(&mut self, ctx: WindowManagerContext<'_>, event: WindowEvent) {
        match event.clone() {
            WindowEvent::WindowCreated(event) => {
                if let Some(window) = ctx.windows.state(event.window) {
                    self.state.renderer.create(event.window, window.clone());
                    self.ui.create(
                        event.window.into(),
                        WindowProperties {
                            size: window.inner_size(),
                            scale_factor: window.scale_factor(),
                            state: window,
                        },
                    );
                    self.doc = self.ui.runtime().create_document(event.window.into());
                }
            }
            WindowEvent::WindowResized(event) => {
                self.state.renderer.resize(event.window, event.size());
                self.ui.resize(event.window.into(), event.size());
                self.state.target_size = event.size();
                self.size = event.size();
            }
            WindowEvent::WindowDestroyed(event) => {
                self.state.renderer.destroy(event.window);
            }
            WindowEvent::KeyboardInput(event) => {
                if event.key_code == Some(KeyCode::F3) && event.state.is_pressed() {
                    self.debug_options.draw_colliders ^= true;
                }

                if event.key_code == Some(KeyCode::F4) && event.state.is_pressed() {
                    self.debug_options.draw_object_translations ^= true;
                }

                if event.key_code == Some(KeyCode::F5) && event.state.is_pressed() {
                    self.debug_options.show_memory_visualizer ^= true;
                }

                if event.key_code == Some(KeyCode::F6) && event.state.is_pressed() {
                    self.debug_options.draw_cell_boundaries ^= true;
                }

                if event.key_code == Some(KeyCode::F7) && event.state.is_pressed() {
                    self.debug_options.draw_skeletons ^= true;
                }
            }
            _ => (),
        }

        self.game.send_event(event);
    }

    fn update(&mut self, ctx: WindowManagerContext<'_>) {
        if let Some(doc) = &mut self.doc {
            self.ui.runtime().destroy_document(*doc);
            *doc = self
                .ui
                .runtime()
                .create_document(self.window.into())
                .unwrap();
            self.ui.resize(self.window.into(), self.size);

            let root = self.ui.runtime().root_context(*doc);
            root.clear_children();

            if self.debug_options.show_memory_visualizer {
                MemoryVisualizer {
                    size: UVec2::splat(1024),
                    stats: self.state.renderer.statistics().clone(),
                }
                .mount(&root);
                GpuTimings {
                    stats: self.state.renderer.statistics().clone(),
                }
                .mount(&root);
            }
        }

        if self.debug_options.draw_colliders {
            draw_colliers(self.game.world(), &self.state.gizmos);
        }

        if self.debug_options.draw_object_translations {
            draw_translations(self.game.world(), &self.state.gizmos);
        }

        if self.debug_options.draw_cell_boundaries {
            draw_cell_boundaries(self.game.world(), &self.state.gizmos);
        }

        if self.debug_options.draw_skeletons {
            draw_skeletons(self.game.world(), &self.state.gizmos);
        }

        self.time.update();

        self.game.update(&self.time, &self.state.gizmos);
        self.state.sync(self.game.world());
        self.ui.update();
        self.state.render();
    }
}

#[derive(Clone, Debug, Default)]
struct DebugOptions {
    pub draw_colliders: bool,
    pub draw_object_translations: bool,
    pub show_memory_visualizer: bool,
    pub draw_cell_boundaries: bool,
    pub draw_skeletons: bool,
}
