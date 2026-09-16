mod assets;
mod cell;
mod components;
mod config;
mod events;
mod loader;
mod render;
mod shared;
mod state;
mod systems;
mod ui;

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use game_core::time::Time;
use game_crash_handler::main;
use game_egui::EguiContext;
use game_gizmos::Gizmos;
use game_render::RenderTarget;
use game_render::Renderer;
use game_tasks::TaskPool;
use game_ui::debug_widgets::gpu_timings::GpuTimings;
use game_ui::debug_widgets::memory_visualizer::MemoryVisualizer;
use game_ui::runtime::DocumentId;
use game_ui::widgets::Widget;
use game_ui::{UiState, WindowProperties};
use game_ui_render::UiRenderer;
use game_window::events::WindowEvent;
use game_window::windows::{WindowBuilder, WindowId};
use game_window::{WindowManager, WindowManagerContext};
use glam::IVec2;
use glam::UVec2;
use glam::Vec3;
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;
use render::RenderState;
use render::physics_debug::{
    draw_cell_boundaries, draw_colliers, draw_skeletons, draw_translations,
};
use shared::SharedState;
use state::GameState;

use crate::cell::CellId;
use crate::components::NiTransform;
use crate::components::WorldSpace;
use crate::config::Config;
use crate::loader::FileLoader;
use crate::render::physics_debug::{
    draw_doors, draw_hitscan_markers, draw_light_markers, draw_navmesh,
    draw_planned_ai_agent_paths, draw_trigger_boxes,
};
use crate::state::data;
use crate::state::esp::EspLoaderBuilder;
use crate::state::hardcoded;
use crate::state::hardcoded::ACTOR_PLAYER;
use crate::state::hardcoded::HARDCODED_MODULE_ID;
use crate::systems::inventory::AddItemEventAsync;
use crate::systems::inventory::EquipItemEvent;

#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Disable the crash handler shim.
    // Note: This flag is handled by the crash handler shim
    // and should not be used by us.
    // It only exists so that the flag is included in the help
    // message.
    #[arg(long = "no-crash-handler")]
    _no_crash_handler: bool,
    /// Path to the config file.
    #[arg(long, default_value = "config.toml")]
    config: PathBuf,
    /// Path to a data directory. This argument can be repeated. The data directories will be
    /// loaded in the exact order specified.
    #[arg(long)]
    data: Option<Vec<PathBuf>>,
}

#[main]
fn main() -> ExitCode {
    let args = Args::parse();

    game_core::logger::init();

    let config = match Config::from_file(&args.config) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(
                "failed to load config from {}: {}",
                args.config.to_string_lossy(),
                err
            );
            return ExitCode::FAILURE;
        }
    };

    let data_dirs = args.data.unwrap_or_else(|| vec![PathBuf::from(".")]);

    let num_threads = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
    let pool = Arc::new(TaskPool::new(num_threads.get()));

    let mut wm = WindowManager::new();
    let window = wm.windows_mut().spawn(WindowBuilder::new());

    let mut renderer = Renderer::new().unwrap();
    let entities = game_core_pipeline::init(&mut renderer);
    let gizmos = Gizmos::new(&mut renderer);

    let shared = Arc::new(SharedState::new(entities.clone()));

    let ui_renderer = Arc::new(UiRenderer::new(&mut renderer));
    let egui_ctx = EguiContext::new(ui_renderer);

    let ui = UiState::new(&mut renderer);
    let mut game =
        futures_lite::future::block_on(create_game(shared.clone(), pool, &data_dirs, config));
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
        time: Time::new(),
        egui_ctx,
        ui_state: ui::UiState::default(),
    };

    wm.run(app);
    ExitCode::SUCCESS
}

async fn create_game(
    shared: Arc<SharedState>,
    pool: Arc<TaskPool>,
    data_dirs: &[PathBuf],
    config: Config,
) -> GameState {
    let mut loader = EspLoaderBuilder::new();
    loader.set_hardcoded(hardcoded::load_hardcoded_forms());

    let mut sources = Vec::new();

    for path in data_dirs {
        let contents = match data::scan_data_directory(path) {
            Ok(contents) => contents,
            Err(err) => {
                tracing::error!(
                    "failed to load from data directory {}: {}; skipping",
                    path.to_string_lossy(),
                    err
                );
                continue;
            }
        };

        for path in contents.plugins {
            loader.push(path);
        }

        for path in contents.archives {
            sources.push(path);
        }

        for path in contents.directories {
            sources.push(path);
        }
    }

    let file_loader = Arc::new(FileLoader::new(sources));
    let loader = Arc::new(loader.finalize());

    let mut game = GameState::new(shared, pool, file_loader, loader, config).await;
    spawn_player(&mut game).await;

    game
}

async fn spawn_player(game: &mut GameState) {
    let start_x = std::env::var("OF4_SX")
        .unwrap_or("-13".to_owned())
        .parse()
        .unwrap();
    let start_y = std::env::var("OF4_SY")
        .unwrap_or("13".to_owned())
        .parse()
        .unwrap();

    let offset = CellId(IVec2::new(start_x, start_y)).min();
    let transform =
        NiTransform::from_translation(offset.extend(0.0) + Vec3::new(1024.0, 1024.0, 8096.0));
    let worldspace = WorldSpace {
        record: RecordId::new(HARDCODED_MODULE_ID, FormId(0x3c)),
        is_interior: false,
    };
    let player = game
        .spawn_actor(ACTOR_PLAYER, worldspace, transform.into())
        .await;

    game.event_bus.send(AddItemEventAsync {
        actor: player,
        item: RecordId::new(HARDCODED_MODULE_ID, FormId(0x9983b)),
        // OMODs
        attachments: vec![
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x12eced)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x9983d)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x99836)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x1877fa)),
            RecordId::new(HARDCODED_MODULE_ID, FormId(0x4f21d)),
        ],
        hidden: false,
        count: 1,
        equip: true,
    });
    game.event_bus.send(AddItemEventAsync {
        actor: player,
        item: RecordId::new(HARDCODED_MODULE_ID, FormId(0x000C1897)),
        attachments: Vec::new(),
        hidden: false,
        count: 100,
        equip: true,
    });
    game.event_bus.send(EquipItemEvent {
        actor: player,
        item: RecordId::new(HARDCODED_MODULE_ID, FormId(0x9983b)),
    });
}

struct App {
    game: GameState,
    state: RenderState,
    ui: UiState,
    doc: Option<DocumentId>,
    window: WindowId,
    size: UVec2,
    time: Time,
    egui_ctx: EguiContext,
    ui_state: ui::UiState,
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
            _ => (),
        }

        self.egui_ctx.handle_event(&event);

        if self.egui_ctx.context().wants_pointer_input()
            && matches!(
                event,
                WindowEvent::MouseWheel(_)
                    | WindowEvent::MouseButtonInput(_)
                    | WindowEvent::MouseMotion(_)
            )
        {
            return;
        }

        if self.egui_ctx.context().wants_keyboard_input()
            && matches!(event, WindowEvent::KeyboardInput(_))
        {
            return;
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

            if self.ui_state.debug.draw_gpu_memory_visualizer {
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

        self.egui_ctx
            .draw(|ctx| ui::draw_ui(ctx, &mut self.ui_state, &mut self.game));

        self.time.update();

        self.game.update(&self.time, &self.state.gizmos);
        self.state.sync(self.game.world());
        self.ui.update();

        if self.ui_state.debug.draw_colliders {
            draw_colliers(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_translations {
            draw_translations(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_cell_boundaries {
            draw_cell_boundaries(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_skeletons {
            draw_skeletons(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_light_markers {
            draw_light_markers(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_hitscan_marker {
            draw_hitscan_markers(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_doors {
            draw_doors(self.game.world(), &self.state.gizmos);
        }

        if self.ui_state.debug.draw_trigger_boxes {
            draw_trigger_boxes(&self.game.world, &self.state.gizmos);
        }

        if self.ui_state.debug.draw_navmesh {
            draw_navmesh(&self.game.world, &self.game.ai, &self.state.gizmos);
        }

        if self.ui_state.debug.draw_planned_ai_agent_paths {
            draw_planned_ai_agent_paths(&self.game.world, &self.game.ai, &self.state.gizmos);
        }

        self.state.render();

        // FIXME: Don't leak this into main.
        self.game.event_bus.clear();
        self.game.world.clear_trackers();
    }
}
