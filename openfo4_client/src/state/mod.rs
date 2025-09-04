pub mod actions;
pub mod anim_starter;
pub mod animation;
pub mod animation_test;
pub mod asset;
pub mod esp;
pub mod level;
pub mod skinning;
pub mod spawner;
pub mod terrain;

use std::f32::consts::PI;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::{Duration, Instant};

use esp::EspLoader;
use game_common::components::{Color, DirectionalLight, Transform};
use game_core::counter::Interval;
use game_core::time::Time;
use game_core_pipeline::material::StandardMaterial;
use game_gizmos::Gizmos;
use game_input::keyboard::KeyCode;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_window::events::WindowEvent;
use glam::{IVec2, Vec3};
use hecs::{With, World};
use level::{Level, Streamer};
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_esp::EspFile;
use openfo4_physics::PhysicsPipeline;
use openfo4_physics::components::{RigidBody, RigidBodyKind};
use openfo4_physics::controller::CharacterController;

use crate::assets::nif_v2::NifLoaderContext;
use crate::assets::{AssetServer, FileLoadContext};
use crate::cell::CellId;
use crate::components::{Camera, NiTransform, Player};
use crate::events::Event;
use crate::loader::FileLoader;
use crate::render::RenderObject;
use crate::shared::SharedState;
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::{AnimationEventQueue, AnimationSystem};
use crate::systems;
use crate::systems::animation_sync::update_animation_parameters;
use crate::systems::camera::CameraMode;
use crate::systems::controller::apply_player_movements;

pub const UPS: u32 = 60;

// Load cell
// Instantiate entity

pub struct GameState {
    pool: Arc<TaskPool>,
    world: World,
    shared: Arc<SharedState>,
    loader: Arc<FileLoader>,
    level: Level,
    physics: PhysicsPipeline,
    events: Vec<Event>,
    anim_event: Option<&'static str>,
    assets: AssetServer,
    asset_loader: AssetLoader,
    event_bus: EventBus,
    animation: AnimationSystem,
    actions: Actions,
    physics_debug: bool,
    interval: Interval,
    esp_loader: Arc<EspLoader>,
}

impl GameState {
    pub fn new(shared: Arc<SharedState>, pool: Arc<TaskPool>) -> Self {
        let mut file =
            EspFile::new(BufReader::new(File::open("../../Fallout4.esm").unwrap())).unwrap();

        let sources = [
            "../../archive/raw/Fallout4 - Animations.ba2",
            "../../archive/raw/Fallout4 - Interface.ba2",
            "../../archive/raw/Fallout4 - Materials.ba2",
            "../../archive/raw/Fallout4 - Meshes.ba2",
            "../../archive/raw/Fallout4 - MeshesExtra.ba2",
            "../../archive/raw/Fallout4 - Misc.ba2",
            "../../archive/raw/Fallout4 - Nvflex.ba2",
            "../../archive/raw/Fallout4 - Shaders.ba2",
            "../../archive/raw/Fallout4 - Sounds.ba2",
            "../../archive/raw/Fallout4 - Startup.ba2",
            "../../archive/raw/Fallout4 - Textures1.ba2",
            "../../archive/raw/Fallout4 - Textures2.ba2",
            "../../archive/raw/Fallout4 - Textures3.ba2",
            "../../archive/raw/Fallout4 - Textures4.ba2",
            "../../archive/raw/Fallout4 - Textures5.ba2",
            "../../archive/raw/Fallout4 - Textures6.ba2",
            "../../archive/raw/Fallout4 - Textures7.ba2",
            "../../archive/raw/Fallout4 - Textures8.ba2",
            "../../archive/raw/Fallout4 - Textures9.ba2",
        ];

        let mut loader = EspLoader::new();
        let file_loader = Arc::new(FileLoader::new(sources));
        loader.push(file);
        let loader = Arc::new(loader);
        let mut assets = AssetServer::new(file_loader.clone(), pool.clone());

        let asset_loader = AssetLoader::new();
        asset_loader.register::<crate::assets::nif_v2::NifScene>(NifLoaderContext::new(
            assets.clone(),
            shared.entities.clone(),
        ));
        asset_loader.register::<crate::assets::texture::FileImageHandle>(FileLoadContext {
            server: assets.clone(),
            entities: shared.entities.clone(),
        });
        asset_loader.register::<crate::assets::material::BgsmMaterial>(FileLoadContext {
            server: assets.clone(),
            entities: shared.entities.clone(),
        });
        asset_loader.register::<crate::assets::hkx::FileHkxHandle>(FileLoadContext {
            server: assets.clone(),
            entities: shared.entities.clone(),
        });

        let actions = futures_lite::future::block_on(actions::Actions::load(&loader));
        dbg!(&actions);

        let animation = AnimationSystem::new(&loader);

        let level = Level::new(
            loader.clone(),
            asset_loader.clone(),
            pool.clone(),
            file_loader.clone(),
            shared.clone(),
        );

        Self {
            pool,
            world: World::new(),
            shared,
            level,
            loader: file_loader.clone(),
            physics: PhysicsPipeline::new(),
            events: Vec::new(),
            anim_event: None,
            assets,
            asset_loader,
            event_bus: EventBus::new(),
            animation,
            actions,
            physics_debug: false,
            interval: Interval::new(Duration::from_secs(1) / UPS),
            esp_loader: loader,
        }
    }

    pub fn init_camera(&mut self) {
        let distance = std::env::var("OF4_DIST")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0);

        // let offset = Vec2::new(2.0, 7.0) * 4096.0 * NIF_TO_METERS;
        let offset = CellId(IVec2::new(2, 7)).min();
        let offset = NiTransform::from_translation(offset.extend(0.0));
        self.world.spawn((
            Transform::from(offset).looking_at(Vec3::ZERO, Vec3::Y),
            Camera {
                fov: PI / 4.0,
                mode: CameraMode::Free,
            },
            Streamer { distance },
        ));
    }

    pub fn toggle_sun_light(&mut self) {
        for (entity, _) in self.world.query_mut::<(&Transform, &DirectionalLight)>() {
            self.world.despawn(entity).unwrap();
            return;
        }

        self.world.spawn((
            DirectionalLight {
                color: Color::WHITE,
                illuminance: 25_000.0,
            },
            Transform::from_translation(Vec3::splat(200.0)).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }

    pub fn update(&mut self, time: &Time, gizmos: &Gizmos) {
        let _span = trace_span!("GameState::update").entered();

        futures_lite::future::block_on(self.interval.wait(Instant::now()));

        self.level.update(&mut self.world, &mut self.events);

        animation_test::animation_test(
            &self.loader,
            &self.shared,
            &mut self.world,
            &self.assets,
            &self.asset_loader,
            &self.esp_loader,
        );

        apply_player_movements(&mut self.world, &mut self.physics);
        update_animation_parameters(&mut self.world);

        // systems::ni_transform::compute_transform_from_ni_transform(&mut self.world);
        // systems::ni_transform::compute_cell_ids_from_ni_transform(&mut self.world);
        // systems::hierarchy::compute_global_transform(&mut self.world);

        self.animation.update(&mut self.world);

        openfo4_animation::systems::drive_animations(
            &mut self.world,
            Duration::from_secs(1) / UPS,
            self.anim_event.take(),
        );

        // systems::hierarchy::compute_global_transform(&mut self.world);
        self.physics.step(&mut self.world);

        skinning::mesh_skinning(&mut self.world, &self.shared);

        systems::hierarchy::compute_global_transform(&mut self.world);

        systems::camera::update_camera_transform(&mut self.world);

        if self.physics_debug {
            self.physics.render_debug(gizmos);
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn send_event(&mut self, event: WindowEvent) {
        systems::camera::handle_free_camera(&mut self.world, &event);
        systems::controller::handle_inputs(&mut self.world, &self.actions, &event);

        match event {
            WindowEvent::KeyboardInput(event) => {
                if event.key_code == Some(KeyCode::L) && event.state.is_pressed() {
                    let s: game_render::mesh::Mesh = game_render::shape::Box {
                        min_x: -30.0,
                        max_x: 30.0,
                        min_y: -30.0,
                        max_y: 30.0,
                        min_z: -30.0,
                        max_z: 30.0,
                    }
                    .into();
                    let mesh = openfo4_physics::components::ColliderShape::TriMesh(
                        openfo4_physics::components::TriMesh::new_convex(
                            s.positions().to_vec(),
                            s.indicies().unwrap().into_u32(),
                        ),
                    );

                    let mut transform = Transform::default();
                    for (_, (t, _)) in self.world.query::<(&Transform, &Camera)>().iter() {
                        transform = *t;
                        break;
                    }

                    let m = self.shared.entities.create_mesh(s);
                    let material = self.shared.entities.create_material(StandardMaterial {
                        base_color: Color::BLACK,
                        ..Default::default()
                    });
                    let obj = RenderObject { mesh: m, material };

                    self.world.spawn((
                        transform,
                        openfo4_physics::components::Collider {
                            friction: 1.0,
                            restitution: 0.0,
                            shape: mesh,
                        },
                        openfo4_physics::components::RigidBody {
                            kind: openfo4_physics::components::RigidBodyKind::Dynamic,
                            linvel: Vec3::ZERO,
                            angvel: Vec3::ZERO,
                        },
                        obj,
                    ));
                }

                if !event.state.is_pressed() {
                    return;
                }

                if event.key_code == Some(KeyCode::F8) {
                    self.physics_debug ^= true;
                }

                if event.key_code == Some(KeyCode::F9) {
                    self.toggle_sun_light();
                }

                if event.key_code == Some(KeyCode::T) {
                    if let Some(action) = self.actions.0.get(&Action::FireSingle) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }

                if event.key_code == Some(KeyCode::Z) {
                    if let Some(action) = self.actions.0.get(&Action::FireAuto) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }

                if event.key_code == Some(KeyCode::R) {
                    // self.anim_event = Some("g_weapForceEquip");
                    // self.anim_event = Some("MoveStart");

                    if let Some(action) = self.actions.0.get(&Action::Reload) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }

                if event.key_code == Some(KeyCode::O) {
                    // self.anim_event = Some("g_weapForceEquip");
                    // self.anim_event = Some("MoveStart");

                    if let Some(action) = self.actions.0.get(&Action::GunRelaxed) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::P) {
                    // self.anim_event = Some("weapUnequip");
                    // self.anim_event = Some("CameraSneakStart");

                    if let Some(action) = self.actions.0.get(&Action::GunReady) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::I) {
                    // self.anim_event = Some("attackStart");
                    // self.anim_event = Some("CameraSneakStop");
                }
                if event.key_code == Some(KeyCode::U) {
                    // self.anim_event = Some("attackEnd");
                    // self.anim_event = Some("moveStartForward");
                    // self.anim_event = Some("CameraMoveStartSlave");

                    if let Some(action) = self.actions.0.get(&Action::MoveStart) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::J) {
                    // self.anim_event = Some("attackStartAuto");
                    // self.anim_event = Some("moveStop");
                    // self.anim_event = Some("CameraMoveStopSlave");

                    if let Some(action) = self.actions.0.get(&Action::MoveStop) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::K) {
                    // self.anim_event = Some("attackRelease");
                    // self.anim_event = Some("speedup");
                    if let Some(action) = self.actions.0.get(&Action::Jump) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::L) {
                    // self.anim_event = Some("PerkCannibalFeed");
                    // self.anim_event = Some("speeddown");

                    if let Some(action) = self.actions.0.get(&Action::PerkCannibal) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::G) {
                    // self.anim_event = Some("EssentialDownStart");

                    if let Some(action) = self.actions.0.get(&Action::BleedoutStart) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::H) {
                    // self.anim_event = Some("EssentialDownExit");

                    if let Some(action) = self.actions.0.get(&Action::BleedoutStop) {
                        for (_, events) in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::E) {
                    self.anim_event = Some("dump");
                }

                if event.key_code == Some(KeyCode::B) {
                    for (e, (body, _, controller)) in
                        self.world
                            .query_mut::<(&mut RigidBody, &Player, Option<&CharacterController>)>()
                    {
                        if controller.is_none() {
                            self.world
                                .insert_one(e, CharacterController::new())
                                .unwrap();
                        } else {
                            self.world.remove_one::<CharacterController>(e).unwrap();
                        }

                        break;
                    }
                }
            }
            _ => (),
        }
    }
}
