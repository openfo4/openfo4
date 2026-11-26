mod input;
mod render;
mod ui;

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use futures_lite::future::block_on;
use glam::IVec2;
use glam::Quat;
use glam::UVec2;
use glam::Vec3;
use openfo4_audio::AudioManager;
use openfo4_audio::backend::DefaultBackend;
use openfo4_config::ConfigSection;
use openfo4_core::record::RecordId;
use openfo4_crash_handler::main;
use openfo4_egui::EguiContext;
use openfo4_esp::EspFile;
use openfo4_esp::FormId;
use openfo4_esp::bstr::BString;
use openfo4_esp::bstr::ByteVec;
use openfo4_game::cell::CellId;
use openfo4_game::components::NiTransform;
use openfo4_game::components::WorldSpace;
use openfo4_game::config::Config;
use openfo4_game::loader::FileLoader;
use openfo4_game::shared::SharedState;
use openfo4_game::state::GameState;
use openfo4_game::state::data;
use openfo4_game::state::esp::EspLoaderBuilder;
use openfo4_game::state::hardcoded;
use openfo4_game::state::hardcoded::HARDCODED_MODULE_ID;
use openfo4_game::state::inputs::ActorInputEvent;
use openfo4_game::state::inputs::Direction;
use openfo4_game::systems::inventory::AddItemEventAsync;
use openfo4_game::systems::inventory::EquipItemEvent;
use openfo4_gizmos::Gizmos;
use openfo4_input::keyboard::KeyCode;
use openfo4_input::keyboard::KeyboardInput;
use openfo4_io::fs::File;
use openfo4_render::RenderTarget;
use openfo4_render::Renderer;
use openfo4_render::RendererConfig;
use openfo4_tasks::TaskPool;
use openfo4_ui_render::UiRenderer;
use openfo4_window::events::WindowEvent;
use openfo4_window::windows::WindowBuilder;
use openfo4_window::{WindowManager, WindowManagerContext};
use render::RenderState;
use render::physics_debug::{
    draw_cell_boundaries, draw_colliers, draw_skeletons, draw_translations,
};

use crate::input::EventMode;
use crate::input::KeyMap;
use crate::render::physics_debug::{
    draw_doors, draw_hitscan_markers, draw_light_markers, draw_navmesh,
    draw_planned_ai_agent_paths, draw_trigger_boxes,
};

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

    openfo4_core::logger::init();

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

    let Ok(mut wm) = WindowManager::new().inspect_err(|err| {
        tracing::error!("failed to create window manager: {}", err);
    }) else {
        return ExitCode::FAILURE;
    };

    let window = wm.windows_mut().spawn(WindowBuilder::new());

    let mut renderer = Renderer::new(RendererConfig {
        validation: config.graphics.validation,
        gpuav: config.graphics.gpuav,
    })
    .unwrap();
    let entities = openfo4_core_pipeline::init(&mut renderer);
    let gizmos = Gizmos::new(&mut renderer);

    let shared = Arc::new(SharedState::new(entities.clone()));

    let ui_renderer = Arc::new(UiRenderer::new(&mut renderer));
    let egui_ctx = EguiContext::new(ui_renderer.clone());

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
        size: UVec2::ZERO,
        egui_ctx,
        ui_state: ui::UiState::default(),
        ui_renderer,
        keymap: build_default_keymap(),
        key_events: Vec::new(),
        turn_events: Vec::new(),
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
    loader.set_hardcoded_parents(hardcoded::load_hardcoded_parents());

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
            let name = path.file_name().unwrap();
            let name = Vec::from_os_string(name.to_owned()).unwrap();

            let file = File::open(path).unwrap();
            let file = EspFile::new(file).await.unwrap();

            loader.push(BString::new(name), file);
        }

        for path in contents.archives {
            sources.push(path);
        }

        for path in contents.directories {
            sources.push(path);
        }
    }

    let file_loader = Arc::new(FileLoader::new(sources));
    let loader = Arc::new(loader.finalize().await);

    let audio = AudioManager::new(DefaultBackend::default());

    let mut game = GameState::new(shared, pool, file_loader, loader, config, audio).await;
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
    let player = game.spawn_player(worldspace, transform.into()).await;

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
    size: UVec2,
    egui_ctx: EguiContext,
    ui_state: ui::UiState,
    ui_renderer: Arc<UiRenderer>,
    keymap: KeyMap,
    key_events: Vec<KeyboardInput>,
    turn_events: Vec<ActorInputEvent>,
}

impl openfo4_window::App for App {
    fn handle_event(&mut self, ctx: WindowManagerContext<'_>, event: WindowEvent) {
        match event.clone() {
            WindowEvent::WindowCreated(event) => {
                if let Some(window) = ctx.windows.state(event.window) {
                    self.state.renderer.create(event.window, window.clone());
                }
            }
            WindowEvent::WindowResized(event) => {
                self.state.renderer.resize(event.window, event.size());
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

        match event {
            WindowEvent::KeyboardInput(event) => {
                self.key_events.push(event);
            }
            WindowEvent::MouseMotion(event) => {
                let yaw = event.delta.x * 0.001;
                let pitch = event.delta.y * 0.001;

                let yaw = Quat::from_axis_angle(Vec3::Y, -yaw);
                let pitch = Quat::from_axis_angle(Vec3::X, -pitch);
                self.turn_events.push(ActorInputEvent::TurnYaw(yaw));
                self.turn_events.push(ActorInputEvent::TurnPitch(pitch));
            }
            _ => {}
        }
    }

    fn update(&mut self, _ctx: WindowManagerContext<'_>) {
        self.egui_ctx.draw(|ctx| {
            ui::draw_ui(
                ctx,
                &mut self.ui_state,
                &mut self.game,
                &self.state.renderer,
            )
        });

        let inputs = self.keymap.process(self.key_events.drain(..));
        if let Some(player) = self.game.get_player() {
            for input in inputs.chain(self.turn_events.drain(..)) {
                self.game.send_actor_input(player, input);
            }
        }

        block_on(self.game.update());
        self.state.sync(self.game.world());

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

        self.ui_renderer.barrier();

        self.state.render();

        // FIXME: Don't leak this into main.
        self.game.event_bus.clear();
        self.game.world.clear_trackers();
    }
}

fn build_default_keymap() -> KeyMap {
    let mut keymap = KeyMap::default();
    keymap.insert(
        KeyCode::W,
        EventMode::Continous,
        ActorInputEvent::Move {
            direction: Direction::FRONT,
            speed: 1.0,
        },
    );
    keymap.insert(
        KeyCode::S,
        EventMode::Continous,
        ActorInputEvent::Move {
            direction: Direction::BACK,
            speed: 1.0,
        },
    );
    keymap.insert(
        KeyCode::A,
        EventMode::Continous,
        ActorInputEvent::Move {
            direction: Direction::LEFT,
            speed: 1.0,
        },
    );
    keymap.insert(
        KeyCode::D,
        EventMode::Continous,
        ActorInputEvent::Move {
            direction: Direction::RIGHT,
            speed: 1.0,
        },
    );
    keymap.insert(
        KeyCode::Q,
        EventMode::OnKeyDown,
        ActorInputEvent::WeaponDraw,
    );
    keymap.insert(
        KeyCode::R,
        EventMode::OnKeyDown,
        ActorInputEvent::WeaponReload,
    );
    keymap.insert(KeyCode::F, EventMode::OnKeyDown, ActorInputEvent::Activate);
    keymap.insert(
        KeyCode::T,
        EventMode::OnKeyDown,
        ActorInputEvent::WeaponAttack,
    );
    keymap.insert(
        KeyCode::Key1,
        EventMode::OnKeyDown,
        ActorInputEvent::ToggleWalk,
    );
    keymap.insert(
        KeyCode::LControl,
        EventMode::OnKeyDown,
        ActorInputEvent::ToggleSneak,
    );
    keymap.insert(
        KeyCode::LShift,
        EventMode::OnKeyDown,
        ActorInputEvent::ToggleSprint,
    );
    keymap.insert(
        KeyCode::Tab,
        EventMode::OnKeyDown,
        ActorInputEvent::ToggleFreeCamera,
    );

    keymap
}
