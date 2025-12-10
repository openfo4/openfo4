use std::time::Duration;

use game_common::components::{Color, PointLight, Transform};
use glam::{EulerRot, Quat, Vec3};
use hecs::{Entity, World};
use openfo4_asset::Handle;
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_esp::records::LightDataFlags;
use openfo4_esp::{Record, RecordData, RecordHeaderFlags};
use openfo4_physics::components::{Collider, RigidBody};

use crate::assets::nif_v2::{Instantiate, LoadNifFlags, NIF_TO_METERS, NifScene};
use crate::components::{NiTransform, Persistent, Root, WorldSpace};
use crate::render::RenderObject;
use crate::state::level::LoadContext;
use crate::state::level::terrain::load_land;
use crate::state::npc::Actor;
use crate::systems::lights::{LightFlicker, LightFlickerMode};

#[derive(Debug, Default)]
pub struct SpawnEntity {
    pub transform: NiTransform,
    pub scene: Option<Handle<NifScene>>,
    pub collider: Option<Collider>,
    pub object: Option<RenderObject>,
    pub rigid_body: Option<RigidBody>,
    pub light: Option<PointLight>,
    pub light_flicker: Option<LightFlicker>,
    pub actor: Option<Actor>,
    pub persistent: bool,
    pub worldspace: Option<WorldSpace>,
}

impl SpawnEntity {
    pub fn instantiate(self, world: &mut World, events: &mut EventBus) -> Entity {
        // Destructure pattern so we get warned if we forget
        // to add new items here.
        let Self {
            transform,
            scene,
            collider,
            object,
            rigid_body,
            light,
            light_flicker,
            actor,
            persistent,
            worldspace,
        } = self;

        // FIXME: Modify this such that both scene and actor are valid.
        let entity = if let Some(scene) = scene {
            scene.instantiate(transform.into(), world, LoadNifFlags::all())
        } else if let Some(actor) = actor {
            actor.instantiate(world, events)
        } else {
            world.reserve_entity()
        };

        world
            .insert_one(entity, Transform::from(transform))
            .unwrap();

        if let Some(rigid_body) = rigid_body {
            world.insert_one(entity, rigid_body).unwrap();
        }

        if let Some(collider) = collider {
            world.insert_one(entity, collider).unwrap();
        }

        if let Some(object) = object {
            world.insert_one(entity, object).unwrap();
        }

        if let Some(light) = light {
            world.insert_one(entity, light).unwrap();
        }

        if let Some(light_flicker) = light_flicker {
            world.insert_one(entity, light_flicker).unwrap();
        }

        if persistent {
            world.insert_one(entity, Persistent).unwrap();
        }

        world.insert_one(entity, Root).unwrap();

        // FIXME: What about attached equipment items?
        if let Some(worldspace) = worldspace {
            let mut queue = vec![entity];
            while let Some(e) = queue.pop() {
                world.insert_one(e, worldspace).unwrap();
                if let Ok(children) = world.get::<&Children>(e) {
                    queue.extend(&children.0);
                }
            }
        }

        entity
    }

    pub(super) async fn load(ctx: &LoadContext<'_>, record: &Record) -> Option<Self> {
        let position;
        let scale;
        let base_object;

        let mut light_mod = LightModification::default();

        match &record.data {
            RecordData::PlacedObject(object) => {
                position = object.position;
                base_object = object.base_object;
                scale = object.scale.unwrap_or(1.0);

                if let Some(radius) = object.radius {
                    light_mod.radius = radius;
                }

                if let Some(light) = object.light_data {
                    light_mod.fov = light.fov_offset_from_base_object;
                    light_mod.fade = light.fade_offset_from_base_ojbect;
                }
            }
            RecordData::PlacedNpc(npc) => {
                position = npc.position;
                base_object = npc.base_npc;
                scale = 1.0;
            }
            RecordData::Land(land) => {
                let entity = load_land(ctx, &land).await;
                return Some(entity);
            }
            _ => return None,
        }

        let transform = Transform {
            translation: Vec3::new(
                position.position_x,
                position.position_y,
                position.position_z,
            ),
            // Rotation is negative for some reason?
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                -position.rotation_x,
                -position.rotation_y,
                -position.rotation_z,
            ),
            scale: Vec3::splat(scale),
        };

        let base = match ctx.esp_loader.lookup(None, base_object).await {
            Ok(base) => base,
            Err(err) => {
                // TODO
                return None;
            }
        };

        if base.flags.contains(RecordHeaderFlags::IS_MARKER) {
            return None;
        }

        let mut model = None;
        let mut light = None;
        let mut light_flicker = None;
        let mut actor = None;

        match base.data {
            RecordData::STAT(data) => {
                model = data.modl.model;
            }
            RecordData::StaticCollection(data) => {
                model = data.model.model;
            }
            RecordData::MSTT(data) => {
                model = data.model.model;
            }
            RecordData::DOOR(data) => {
                model = data.modl.model;
            }
            RecordData::Container(data) => {
                model = data.modl.model;
            }
            RecordData::Npc(_) => {
                actor = crate::state::npc::spawn_actor(ctx.loader, ctx.esp_loader, base.id)
                    .await
                    .ok();
            }
            RecordData::IdleMarker(_) => return None,
            RecordData::Activator(data) => {
                model = data.model.model;
            }
            RecordData::MiscObject(data) => {
                model = data.model.model;
            }
            RecordData::Furniture(data) => {
                model = data.model.model;
            }
            RecordData::Light(data) => {
                model = data.model.model;

                let intensity = (data.fade_value + light_mod.fade) * 10.0;
                let radius = (data.data.radius as f32 + light_mod.radius) * NIF_TO_METERS;

                light = Some(PointLight {
                    color: Color::from_rgb([
                        data.data.color.r as f32 / 255.0,
                        data.data.color.g as f32 / 255.0,
                        data.data.color.b as f32 / 255.0,
                    ]),
                    intensity,
                    radius,
                });

                let is_pulsing = data.data.flags.contains(LightDataFlags::PULSE);
                let is_flicking = data.data.flags.contains(LightDataFlags::FLICKER);

                if is_pulsing || is_flicking {
                    let mode = if is_pulsing {
                        LightFlickerMode::Pulse
                    } else {
                        debug_assert!(is_flicking);
                        LightFlickerMode::Flicker
                    };

                    let period = Duration::from_secs_f32(1.0 / data.data.flicker_period);

                    light_flicker = Some(LightFlicker {
                        mode,
                        period,
                        intensity,
                        intensity_amplitude: data.data.flicker_intensity_amplitude,
                        state: Default::default(),
                    });
                }
            }
            RecordData::Ingestible(data) => {
                model = data.model.model;
            }
            _ => return None,
            x => todo!("{:?}", x),
        }

        let scene = match model {
            Some(model) => {
                let path = format!("meshes/{}", model.as_str().unwrap());
                match ctx.loader.load::<NifScene, _>(&path).await {
                    Ok(scene) => Some(scene),
                    Err(err) => {
                        tracing::error!("{}", err);
                        None
                    }
                }
            }
            None => None,
        };

        Some(SpawnEntity {
            transform: NiTransform::from_ni_transform(transform),
            scene,
            light,
            light_flicker,
            actor,
            rigid_body: Some(RigidBody::FIXED),
            ..Default::default()
        })
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct LightModification {
    radius: f32,
    fov: f32,
    fade: f32,
}
