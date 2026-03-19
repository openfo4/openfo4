pub mod actions;
pub mod anim_starter;
pub mod animation;
pub mod animation_test;
pub mod equipment;
pub mod esp;
pub mod hardcoded;
pub mod item;
pub mod level;
pub mod npc;
pub mod terrain;

mod data;
mod globals;

use std::f32::consts::PI;
use std::path::PathBuf;
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
use hecs::World;
use level::Level;
use openfo4_asset::AssetLoader;
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;
use openfo4_hkx::HkxFile;
use openfo4_nif::Nif;
use openfo4_nif::bgsm::Bgsm;
use openfo4_physics::PhysicsPipeline;
use openfo4_physics::components::{CollisionLayer, RigidBody};
use openfo4_physics::controller::CharacterController;

use crate::assets::FileLoadContext;
use crate::assets::animation::{AnimationSet, FileAnimationClip};
use crate::assets::data_file::{DataFile, DataFileContext};
use crate::assets::nif::NifLoaderContext;
use crate::cell::CellId;
use crate::components::{Camera, NiTransform, Player, WorldSpace};
use crate::loader::FileLoader;
use crate::render::RenderObject;
use crate::render::physics_debug::despawn_expired;
use crate::shared::SharedState;
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::{AnimationEventQueue, AnimationSystem};
use crate::state::animation::update_actor_animation_graphs;
use crate::state::esp::EspLoaderBuilder;
use crate::state::hardcoded::HARDCODED_MODULE_ID;
use crate::systems::activation::update_player_looking_at_target;
use crate::systems::ai::AiSystem;
use crate::systems::animation_sync::{bounce_animation_events, update_animation_parameters};
use crate::systems::camera::CameraMode;
use crate::systems::controller::{apply_player_movements, update_facing_direction};
use crate::systems::door::{handle_automatic_doors, handle_door_activations};
use crate::systems::global::{Difficulty, GameRules, GlobalProperties};
use crate::systems::inventory::InventorySystem;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::{Executor, ScriptEntities};
use crate::systems::time::{init_time, update_time};
use crate::systems::trigger_box::update_trigger_box_contacts;
use crate::systems::weapon::{
    handle_weapon_draw_events, handle_weapon_fire_events, handle_weapon_hit_damage,
    handle_weapon_reload_events, handle_weapon_reload_requests,
};
use crate::systems::weather::WeatherSystem;
use crate::systems::{self, GunReloadRequest, WeaponDrawRequest};

pub const UPS: u32 = 60;

// Load cell
// Instantiate entity

pub struct GameState {
    pool: Arc<TaskPool>,
    pub world: World,
    shared: Arc<SharedState>,
    loader: Arc<FileLoader>,
    level: Level,
    physics: PhysicsPipeline,
    anim_event: Option<&'static str>,
    asset_loader: AssetLoader,
    pub event_bus: EventBus,
    animation: AnimationSystem,
    actions: Actions,
    physics_debug: bool,
    interval: Interval,
    esp_loader: Arc<EspLoader>,
    object_ref_set: ObjectReferenceSet,
    weather: WeatherSystem,
    inventory: InventorySystem,
    executor: Executor,
    quests: QuestManager,
    globals: GlobalProperties,
    script_entities: ScriptEntities,
    pub ai: AiSystem,
    real_time_elapsed: Duration,
}

impl GameState {
    pub fn new(shared: Arc<SharedState>, pool: Arc<TaskPool>, data_dirs: &[PathBuf]) -> Self {
        let mut loader = EspLoaderBuilder::new();
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

        let mut asset_loader = AssetLoader::new();
        asset_loader.register::<crate::assets::nif::NifScene>(NifLoaderContext::new(
            shared.entities.clone(),
        ));
        asset_loader.register::<crate::assets::gltf::GltfScene>(NifLoaderContext::new(
            shared.entities.clone(),
        ));
        asset_loader.register::<crate::assets::texture::FileImageHandle>(FileLoadContext {
            entities: shared.entities.clone(),
        });
        asset_loader.register::<crate::assets::material::BgsmStandardMaterial>(FileLoadContext {
            entities: shared.entities.clone(),
        });
        asset_loader.register::<crate::assets::material::BgsmMaterial>(FileLoadContext {
            entities: shared.entities.clone(),
        });
        asset_loader.register::<DataFile<Nif>>(DataFileContext {
            loader: file_loader.clone(),
        });
        asset_loader.register::<DataFile<HkxFile>>(DataFileContext {
            loader: file_loader.clone(),
        });
        asset_loader.register::<DataFile<Bgsm>>(DataFileContext {
            loader: file_loader.clone(),
        });
        asset_loader.register::<DataFile<openfo4_dds::Image>>(DataFileContext {
            loader: file_loader.clone(),
        });
        asset_loader.register::<DataFile<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>(
            DataFileContext {
                loader: file_loader.clone(),
            },
        );
        asset_loader.register::<FileAnimationClip>(());

        let actions = futures_lite::future::block_on(actions::Actions::load(&loader));
        dbg!(&actions);

        let animation = AnimationSystem::new(&loader);

        let mut executor = futures_lite::future::block_on(Executor::new(
            loader.clone(),
            file_loader.clone(),
            pool.clone(),
        ));

        let level = Level::new(
            loader.clone(),
            asset_loader.clone(),
            pool.clone(),
            file_loader.clone(),
            shared.clone(),
            &mut executor,
        );

        let mut world = World::new();
        init_time(&mut world);

        let quests = QuestManager::new(pool.clone(), &loader, &mut executor, &level.loader());

        let global_variables =
            futures_lite::future::block_on(globals::load_global_variables(&loader));
        let form_lists = futures_lite::future::block_on(globals::load_form_lists(&loader));

        Self {
            weather: WeatherSystem::new(loader.clone(), pool.clone(), &asset_loader, &shared),
            inventory: InventorySystem::new(pool.clone(), asset_loader.clone(), loader.clone()),
            pool,
            world,
            shared,
            level,
            loader: file_loader.clone(),
            physics: PhysicsPipeline::new(),
            anim_event: None,
            asset_loader,
            event_bus: EventBus::new(),
            animation,
            actions,
            physics_debug: false,
            interval: Interval::new(Duration::from_secs(1) / UPS),
            esp_loader: loader,
            object_ref_set: ObjectReferenceSet::default(),
            executor,
            quests,
            globals: GlobalProperties {
                variables: global_variables,
                form_lists,
                game_rules: GameRules {
                    difficulty: Difficulty::Survival,
                    fast_travel_enabled: true,
                },
            },
            script_entities: ScriptEntities::default(),
            ai: AiSystem::default(),
            real_time_elapsed: Duration::ZERO,
        }
    }

    pub fn init_camera(&mut self) {
        // let offset = Vec2::new(2.0, 7.0) * 4096.0 * NIF_TO_METERS;
        let offset = CellId(IVec2::new(2, 7)).min();
        let offset = NiTransform::from_translation(offset.extend(0.0));
        self.world.spawn((
            Transform::from(offset).looking_at(Vec3::ZERO, Vec3::Y),
            Camera {
                fov: PI / 4.0,
                mode: CameraMode::Free,
            },
        ));
    }

    pub fn update(&mut self, time: &Time, gizmos: &Gizmos) {
        let _span = trace_span!("GameState::update").entered();

        futures_lite::future::block_on(self.interval.wait(Instant::now()));
        self.real_time_elapsed += self.interval.timestep();

        self.level.update(
            &mut self.world,
            &mut self.event_bus,
            &mut self.object_ref_set,
            &mut self.script_entities,
        );
        self.ai.update(&mut self.world);

        animation_test::animation_test(
            &mut self.event_bus,
            &mut self.world,
            &self.asset_loader,
            &self.esp_loader,
        );

        update_time(&mut self.world, Duration::from_secs_f32(1.0 / UPS as f32));
        self.weather.update(&mut self.world);

        despawn_expired(&mut self.world);

        update_facing_direction(&mut self.world);
        apply_player_movements(&mut self.world, &mut self.physics);
        update_animation_parameters(&mut self.world);
        update_trigger_box_contacts(&mut self.world, &mut self.event_bus);

        // systems::ni_transform::compute_transform_from_ni_transform(&mut self.world);
        // systems::ni_transform::compute_cell_ids_from_ni_transform(&mut self.world);
        // systems::hierarchy::compute_global_transform(&mut self.world);

        self.animation.update(&mut self.world);

        self.executor.update(
            &mut self.world,
            &mut self.event_bus,
            &mut self.globals,
            &mut self.script_entities,
            &mut self.quests,
            &self.pool,
            &self.esp_loader,
            Duration::from_secs(1) / UPS,
        );
        self.quests
            .update(&mut self.world, &mut self.event_bus, &mut self.executor);

        openfo4_animation::systems::drive_animations::<AnimationSet>(
            &mut self.world,
            Duration::from_secs(1) / UPS,
            self.anim_event.take(),
            &mut self.event_bus,
        );
        bounce_animation_events(&mut self.event_bus);

        systems::nif_animation::update_nif_animations(
            &mut self.world,
            &mut self.event_bus,
            &self.shared.entities,
            self.real_time_elapsed,
        );

        systems::hierarchy::compute_global_transform(&mut self.world);
        self.physics.step(&mut self.world);

        systems::hierarchy::compute_global_transform(&mut self.world);

        systems::camera::update_camera_transform(&mut self.world);

        systems::lights::update_flickering_lights(&mut self.world, time.delta());

        self.inventory.handle_inventory_events(
            &mut self.world,
            &mut self.event_bus,
            &mut self.executor,
        );
        update_actor_animation_graphs(&mut self.world, &self.asset_loader, &self.pool);

        handle_weapon_reload_requests(&mut self.world, &self.actions, &self.event_bus);
        handle_weapon_fire_events(&mut self.world, &self.physics, &mut self.event_bus);
        handle_weapon_reload_events(&mut self.world, &mut self.event_bus);
        handle_weapon_hit_damage(&mut self.world, &mut self.event_bus);
        handle_weapon_draw_events(&mut self.world, &mut self.event_bus);

        handle_door_activations(&mut self.world, &mut self.event_bus, &self.object_ref_set);
        handle_automatic_doors(&mut self.world, &self.object_ref_set);
        update_player_looking_at_target(&mut self.world, &self.physics);

        if self.physics_debug {
            self.physics.render_debug(gizmos);
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn events_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    pub fn send_event(&mut self, event: WindowEvent) {
        systems::camera::handle_free_camera(&mut self.world, &event);
        systems::controller::handle_inputs(
            &mut self.world,
            &self.actions,
            &event,
            &mut self.event_bus,
        );
        systems::activation::handle_activation_inputs(
            &mut self.world,
            &event,
            &mut self.event_bus,
            &self.physics,
        );

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
                            s.indicies().unwrap().clone().into_u32(),
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
                            layer: CollisionLayer::STATIC,
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
                    for (actor, _) in self.world.query_mut::<&Player>() {
                        self.event_bus.send(GunReloadRequest { actor });
                    }
                }

                if event.key_code == Some(KeyCode::Q) {
                    for (actor, _) in self.world.query_mut::<&Player>() {
                        self.event_bus.send(WeaponDrawRequest { actor });
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
