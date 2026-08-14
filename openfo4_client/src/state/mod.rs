pub mod actions;
pub mod anim_starter;
pub mod animation;
pub mod animation_test;
pub mod data;
pub mod equipment;
pub mod esp;
pub mod globals;
pub mod hardcoded;
pub mod level;
pub mod npc;
pub mod terrain;

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant};

use esp::EspLoader;
use game_common::components::{Color, Transform};
use game_core::counter::Interval;
use game_core::time::Time;
use game_core_pipeline::material::StandardMaterial;
use game_gizmos::Gizmos;
use game_input::keyboard::KeyCode;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use game_window::events::WindowEvent;
use glam::{IVec2, Vec3};
use level::Level;
use openfo4_asset::AssetLoader;
use openfo4_core::ecs::{CommandBuffer, Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_physics::PhysicsPipeline;
use openfo4_physics::components::{CollisionLayer, RigidBody};
use openfo4_physics::controller::CharacterController;
use parking_lot::Mutex;

use crate::assets::animation::AnimationSet;
use crate::assets::init_asset_loader;
use crate::cell::CellId;
use crate::components::{Camera, NiTransform, Player};
use crate::config::Config;
use crate::loader::FileLoader;
use crate::render::RenderObject;
use crate::render::physics_debug::despawn_expired;
use crate::shared::SharedState;
use crate::state::actions::{Action, Actions};
use crate::state::anim_starter::{AnimationEventQueue, AnimationSystem};
use crate::state::animation::update_actor_animation_graphs;
use crate::systems::activation::update_player_looking_at_target;
use crate::systems::ai::AiSystem;
use crate::systems::animation_sync::{bounce_animation_events, update_animation_parameters};
use crate::systems::camera::CameraMode;
use crate::systems::controller::{apply_player_movements, update_facing_direction};
use crate::systems::door::{handle_automatic_doors, handle_door_activations};
use crate::systems::global::{Difficulty, GameRules, GlobalProperties, PlayTime};
use crate::systems::hierarchy::TransformHierarchyCache;
use crate::systems::inventory::InventorySystem;
use crate::systems::movement::apply_movement;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::{Executor, ScriptEntities};
use crate::systems::time::{init_time, update_time};
use crate::systems::trigger_box::update_trigger_box_contacts;
use crate::systems::weapon::{
    handle_weapon_draw_events, handle_weapon_fire_events, handle_weapon_fire_requests,
    handle_weapon_hit_damage, handle_weapon_reload_events, handle_weapon_reload_requests,
};
use crate::systems::weather::WeatherSystem;
use crate::systems::{self, GunFireRequest, GunReloadRequest, WeaponDrawRequest, combat};

pub const UPS: u32 = 60;

/// The main simulation structure of a game world.
pub struct GameState {
    pool: Arc<TaskPool>,
    pub world: World,
    shared: Arc<SharedState>,
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
    globals: Arc<GlobalProperties>,
    script_entities: ScriptEntities,
    pub ai: AiSystem,
    real_time_elapsed: Duration,
    transform_hierarchy_cache: TransformHierarchyCache,
}

impl GameState {
    pub async fn new(
        shared: Arc<SharedState>,
        pool: Arc<TaskPool>,
        file_loader: Arc<FileLoader>,
        loader: Arc<EspLoader>,
        config: Config,
    ) -> Self {
        let asset_loader = init_asset_loader(&file_loader, &shared.entities);

        let actions = Actions::load(&loader).await;

        let animation = AnimationSystem::new(&loader);

        let global_variables = globals::load_global_variables(&loader).await;
        let form_lists = globals::load_form_lists(&loader).await;
        let factions = globals::load_factions(&loader).await;
        let globals = Arc::new(GlobalProperties {
            variables: Mutex::new(global_variables),
            form_lists: Mutex::new(form_lists),
            game_rules: GameRules {
                difficulty: Difficulty::Survival,
                fast_travel_enabled: true,
            },
            factions: Mutex::new(factions),
            mods: Default::default(),
            playtime: PlayTime::default(),
        });

        let mut executor = Executor::new(
            loader.clone(),
            file_loader.clone(),
            pool.clone(),
            config.scripting,
            globals.clone(),
        )
        .await;

        let level = Level::new(
            loader.clone(),
            asset_loader.clone(),
            pool.clone(),
            file_loader.clone(),
            shared.clone(),
            &mut executor,
            config.streaming,
        );

        let mut world = World::new();
        init_time(&mut world);

        let quests = QuestManager::new(pool.clone(), &loader, &mut executor, &level.loader());

        Self {
            weather: WeatherSystem::new(loader.clone(), pool.clone(), &asset_loader, &shared),
            inventory: InventorySystem::new(pool.clone(), asset_loader.clone(), loader.clone()),
            pool,
            world,
            shared,
            level,
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
            globals,
            script_entities: ScriptEntities::default(),
            ai: AiSystem::default(),
            real_time_elapsed: Duration::ZERO,
            transform_hierarchy_cache: TransformHierarchyCache::default(),
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
        self.globals.playtime.increase(self.interval.timestep());

        self.level.update(
            &mut self.world,
            &mut self.event_bus,
            &mut self.object_ref_set,
            &mut self.script_entities,
        );
        self.ai.run_events(&mut self.world, &mut self.event_bus);

        animation_test::animation_test(
            &mut self.event_bus,
            &mut self.world,
            &self.asset_loader,
            &self.esp_loader,
            &self.executor.loader(),
            &mut self.script_entities,
        );

        update_time(&mut self.world, Duration::from_secs_f32(1.0 / UPS as f32));
        self.weather
            .update(&mut self.world, self.interval.timestep());

        despawn_expired(&mut self.world);

        update_facing_direction(&mut self.world);
        apply_player_movements(&mut self.world);
        update_trigger_box_contacts(&mut self.world, &mut self.event_bus);

        apply_movement(&mut self.world, &mut self.physics, &self.actions);

        // systems::ni_transform::compute_transform_from_ni_transform(&mut self.world);
        // systems::ni_transform::compute_cell_ids_from_ni_transform(&mut self.world);
        // systems::hierarchy::compute_global_transform(&mut self.world);

        self.animation
            .update(&mut self.world, &self.esp_loader, &self.globals);

        self.executor.update(
            &mut self.world,
            &mut self.event_bus,
            &mut self.script_entities,
            &mut self.quests,
        );
        self.quests
            .update(&mut self.world, &mut self.event_bus, &mut self.executor);

        systems::nif_animation::update_nif_animations(
            &mut self.world,
            &self.shared.entities,
            self.real_time_elapsed,
            self.interval.timestep(),
        );

        systems::lights::update_flickering_lights(&mut self.world, time.delta());

        self.inventory.handle_inventory_events(
            &mut self.world,
            &mut self.event_bus,
            &mut self.executor,
        );
        update_actor_animation_graphs(&mut self.world, &self.asset_loader, &self.pool);
        update_animation_parameters(&mut self.world);

        openfo4_animation::systems::drive_animations::<AnimationSet>(
            &mut self.world,
            Duration::from_secs(1) / UPS,
            self.anim_event.take(),
            &mut self.event_bus,
        );
        bounce_animation_events(&mut self.event_bus);

        handle_weapon_fire_requests(&mut self.world, &self.actions, &self.event_bus);
        handle_weapon_reload_requests(&mut self.world, &self.actions, &self.event_bus);
        handle_weapon_fire_events(&mut self.world, &self.physics, &mut self.event_bus);
        handle_weapon_reload_events(&mut self.world, &mut self.event_bus);
        handle_weapon_hit_damage(&mut self.world, &mut self.event_bus);
        handle_weapon_draw_events(&mut self.world, &mut self.event_bus);

        combat::handle_zero_health_events(&mut self.world, &mut self.event_bus);

        handle_door_activations(&mut self.world, &mut self.event_bus, &self.object_ref_set);
        handle_automatic_doors(&mut self.world, &self.object_ref_set);
        update_player_looking_at_target(&mut self.world, &self.physics);

        self.ai.update(&mut self.world, &mut self.event_bus);

        systems::hierarchy::compute_global_transform(
            &mut self.world,
            &mut self.transform_hierarchy_cache,
        );
        self.physics.step(&mut self.world);

        systems::camera::update_camera_transform(&mut self.world);

        systems::nif_animation::start_nif_animation_sequences(&mut self.world, &self.event_bus);

        self.executor
            .pump_events(&mut self.world, &mut self.event_bus);

        if self.physics_debug {
            self.physics.render_debug(gizmos);
        }
    }

    pub fn world(&self) -> &World {
        &self.world
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
                    for (t, _) in self.world.query::<(&Transform, &Camera)>().iter() {
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
                        RigidBody {
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
                    for (actor, _) in self.world.query_mut::<(Entity, &Player)>() {
                        self.event_bus.send(GunFireRequest { actor });
                    }
                }

                if event.key_code == Some(KeyCode::Z) {
                    if let Some(action) = self.actions.0.get(&Action::FireAuto) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }

                if event.key_code == Some(KeyCode::R) {
                    for (actor, _) in self.world.query_mut::<(Entity, &Player)>() {
                        self.event_bus.send(GunReloadRequest { actor });
                    }
                }

                if event.key_code == Some(KeyCode::Q) {
                    for (actor, _) in self.world.query_mut::<(Entity, &Player)>() {
                        self.event_bus.send(WeaponDrawRequest { actor });
                    }
                }

                if event.key_code == Some(KeyCode::O) {
                    // self.anim_event = Some("g_weapForceEquip");
                    // self.anim_event = Some("MoveStart");

                    if let Some(action) = self.actions.0.get(&Action::Draw) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::P) {
                    // self.anim_event = Some("weapUnequip");
                    // self.anim_event = Some("CameraSneakStart");

                    if let Some(action) = self.actions.0.get(&Action::Sheath) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
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
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::J) {
                    // self.anim_event = Some("attackStartAuto");
                    // self.anim_event = Some("moveStop");
                    // self.anim_event = Some("CameraMoveStopSlave");

                    if let Some(action) = self.actions.0.get(&Action::MoveStop) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::K) {
                    // self.anim_event = Some("attackRelease");
                    // self.anim_event = Some("speedup");
                    if let Some(action) = self.actions.0.get(&Action::Jump) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::L) {
                    // self.anim_event = Some("PerkCannibalFeed");
                    // self.anim_event = Some("speeddown");

                    if let Some(action) = self.actions.0.get(&Action::PerkCannibal) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::G) {
                    // self.anim_event = Some("EssentialDownStart");

                    if let Some(action) = self.actions.0.get(&Action::BleedoutStart) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::H) {
                    // self.anim_event = Some("EssentialDownExit");

                    if let Some(action) = self.actions.0.get(&Action::BleedoutStop) {
                        for mut events in self.world.query_mut::<&mut AnimationEventQueue>() {
                            events.0.push_back(*action);
                        }
                    }
                }
                if event.key_code == Some(KeyCode::E) {
                    self.anim_event = Some("dump");
                }

                if event.key_code == Some(KeyCode::B) {
                    let mut buffer = CommandBuffer::new();

                    for (e, _, _, controller) in self.world.query_mut::<(
                        Entity,
                        &mut RigidBody,
                        &Player,
                        Option<&CharacterController>,
                    )>() {
                        if controller.is_none() {
                            buffer.insert_one(e, CharacterController::new());
                        } else {
                            buffer.remove_one::<CharacterController>(e);
                        }

                        break;
                    }

                    buffer.run_on(&mut self.world);
                }
            }
            _ => (),
        }
    }
}
