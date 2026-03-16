use std::time::Duration;

use game_common::components::{Color, PointLight, Transform};
use glam::{EulerRot, Quat, Vec3};
use hashbrown::HashSet;
use hecs::{Entity, World};
use openfo4_asset::Handle;
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::object_ref_set::ObjectReferenceSet;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_core::{Frozen, WorldExt};
use openfo4_esp::records::{DoorFlags, LightDataFlags};
use openfo4_esp::{RecordData, RecordHeaderFlags};
use openfo4_physics::components::{Collider, RigidBody};

use crate::assets::nif::{Instantiate, LoadNifFlags, NIF_TO_METERS, NifKey, NifScene};
use crate::components::{
    ActorMarker, NiTransform, ObjectBounds, Persistent, PlacedItem, Root, RootParent, TriggerBox,
    WorldSpace,
};
use crate::render::RenderObject;
use crate::state::esp::GlobalRecord;
use crate::state::level::LoadContext;
use crate::state::level::data_ext::RecordDataExt;
use crate::state::level::items::resolve_item;
use crate::state::level::navmesh::load_navmesh;
use crate::state::level::partial::PartialEntityFlags;
use crate::state::level::terrain::load_land;
use crate::state::npc::{Actor, ActorInstanceProps, ObjectKeywords};
use crate::state::terrain::{TerrainManager, TerrainRegion};
use crate::systems::door::{Door, PortalDoor};
use crate::systems::inventory::{Inventory, ItemStack};
use crate::systems::lights::{LightFlicker, LightFlickerMode};
use crate::systems::linked_refs::LinkedRefs;
use crate::systems::pathing::NavMesh;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::loader::ScriptObject;

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
    pub form: Option<RecordId>,
    pub door: Option<Door>,
    pub inventory: Option<Inventory>,
    pub placed_item: Option<PlacedItem>,
    pub terrain: Option<TerrainRegion>,
    pub script: Option<ScriptObject>,
    pub keywords: ObjectKeywords,
    pub trigger_box: Option<TriggerBox>,
    pub navmesh: Option<NavMesh>,
    pub object_bounds: Option<ObjectBounds>,
}

impl SpawnEntity {
    pub fn instantiate(
        self,
        world: &mut World,
        events: &mut EventBus,
        object_ref_set: &mut ObjectReferenceSet,
        terrain_manager: &mut TerrainManager,
        script_entities: &mut ScriptEntities,
    ) -> Entity {
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
            form,
            door,
            inventory,
            placed_item,
            terrain,
            script,
            keywords,
            trigger_box,
            navmesh,
            object_bounds,
        } = self;

        // If the object reference already exists we will overwrite it.
        if let Some(form) = form
            && let Some(entity) = object_ref_set.get_by_form(form)
        {
            world.despawn_recursive(entity);
        }

        let is_actor = actor.is_some();

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

        if let Some(inventory) = inventory {
            world.insert_one(entity, inventory).unwrap();
        }

        if let Some(placed_item) = placed_item {
            world.insert_one(entity, placed_item).unwrap();
        }

        if let Some(tb) = trigger_box {
            world.insert_one(entity, tb).unwrap();
        }

        if let Some(navmesh) = navmesh {
            world.insert_one(entity, navmesh).unwrap();
        }

        if let Some(object_bounds) = object_bounds {
            world.insert_one(entity, object_bounds).unwrap();
        }

        if let Some(script) = script {
            script_entities.0.insert(script.id(), entity);

            // Insert into entity to keep the script alive.
            world.insert_one(entity, script).unwrap();
        }

        if persistent {
            world.insert_one(entity, Persistent).unwrap();
        }

        world.insert_one(entity, Frozen::default()).unwrap();

        world.insert_one(entity, Root).unwrap();

        world.insert_one(entity, LinkedRefs::default()).unwrap();
        world.insert_one(entity, keywords).unwrap();

        if is_actor {
            world.insert_one(entity, ActorMarker).unwrap();
        }

        if let Some(worldspace) = worldspace {
            let mut queue = vec![entity];
            while let Some(e) = queue.pop() {
                world.insert_one(e, worldspace).unwrap();
                world.insert_one(e, RootParent(entity)).unwrap();

                if let Some(door) = door {
                    world.insert_one(e, door).unwrap();
                }

                if let Ok(children) = world.get::<&Children>(e) {
                    queue.extend(&children.0);
                }
            }
        }

        if let Some(form) = form {
            object_ref_set.insert(form, entity);
        }

        if let Some(mut terrain) = terrain {
            terrain.entity = Some(entity);
            terrain_manager.insert(terrain);
        }

        entity
    }

    pub(super) async fn load(
        ctx: &LoadContext<'_>,
        record: &GlobalRecord,
        flags: PartialEntityFlags,
    ) -> Option<Self> {
        let position;
        let scale;
        let base_object;

        let mut light_mod = LightModification::default();
        let mut door = None;
        let mut trigger_box = None;

        match &record.local.data {
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

                if let Some(door_teleport) = object.door_teleport {
                    let target = ctx
                        .esp_loader
                        .resolve_form(door_teleport.destination, record.id)
                        .unwrap();

                    door = Some(Door::Portal(PortalDoor {
                        target,
                        target_position: NiTransform::from_ni_transform(Transform {
                            translation: Vec3::from_array(door_teleport.position),
                            rotation: Quat::from_euler(
                                EulerRot::XYZ,
                                -position.rotation_x,
                                -position.rotation_y,
                                -position.rotation_z,
                            ),
                            scale: Vec3::ONE,
                        }),
                    }));
                }

                if let Some(primitive) = object.primitive {
                    trigger_box = Some(TriggerBox {
                        half_extents: Vec3::from_array(primitive.bound_half_extents).to_opengl()
                            * NIF_TO_METERS,
                        marker_color: Color::from_rgb(primitive.color),
                        entities_inside: HashSet::new(),
                    });
                }
            }
            RecordData::PlacedNpc(npc) => {
                position = npc.position;
                base_object = npc.base_npc;
                scale = 1.0;
            }
            RecordData::Land(land) => {
                let entity = load_land(ctx, record.id, &land).await;
                return Some(entity);
            }
            RecordData::NavMesh(mesh) => {
                let entity = load_navmesh(ctx, &mesh).await;
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

        let base_object = ctx.esp_loader.resolve_form(base_object, record.id).unwrap();
        let base = match ctx.esp_loader.lookup(None, base_object).await {
            Ok(base) => base,
            Err(err) => {
                // TODO
                return None;
            }
        };

        if base.local.flags.contains(RecordHeaderFlags::IS_MARKER) {
            return None;
        }

        let mut model = None;
        let mut light = None;
        let mut light_flicker = None;
        let mut actor = None;
        let mut inventory = None;
        let mut object_bounds = None;

        let mut keywords = HashSet::default();
        for keyword in base.local.data.keywords() {
            let keyword = ctx.esp_loader.resolve_form(*keyword, base.id).unwrap();
            keywords.insert(keyword);
        }

        let item = load_item(ctx, &base).await;

        match base.local.data {
            RecordData::STAT(data) => {
                model = data.modl.model;
                object_bounds = Some(data.object_bounds);
            }
            RecordData::StaticCollection(data) => {
                model = data.model.model;
                object_bounds = Some(data.object_bounds);
            }
            RecordData::MSTT(data) => {
                model = data.model.model;
                object_bounds = Some(data.obnd);
            }
            RecordData::Door(data) => {
                model = data.model.model;

                if data.flags.contains(DoorFlags::AUTOMATIC) {
                    door = door.map(|door| match door {
                        Door::Portal(door) => Door::AutoPortal(door),
                        door => door,
                    });
                }

                object_bounds = Some(data.object_bounds);
            }
            RecordData::Container(data) => {
                model = data.model.model;

                let mut inv = Inventory::default();

                for item in &data.items {
                    let item_id = ctx.esp_loader.resolve_form(item.item, base.id).unwrap();

                    let items = resolve_item(
                        ctx.esp_loader,
                        ctx.script_loader,
                        item_id,
                        item.count.into(),
                        &[],
                    )
                    .await;
                    for item in items {
                        inv.insert(item);
                    }
                }

                inventory = Some(inv);

                object_bounds = Some(data.object_bounds);
            }
            RecordData::Npc(_) => {
                if flags.contains(PartialEntityFlags::ACTOR) {
                    let props = ActorInstanceProps {
                        dead: record.local.flags.contains(RecordHeaderFlags::STARTS_DEAD),
                    };

                    if let Some(mut res) =
                        crate::state::npc::spawn_actor(ctx.loader, ctx.esp_loader, base.id, &props)
                            .await
                            .ok()
                    {
                        keywords = core::mem::take(&mut res.keywords);
                        actor = Some(res);
                    }
                }
            }
            RecordData::IdleMarker(_) => return None,
            RecordData::Activator(data) => {
                model = data.model.model;
                object_bounds = Some(data.object_bounds);
            }
            RecordData::MiscObject(data) => {
                model = data.model.model;
                object_bounds = Some(data.object_bounds);
            }
            RecordData::Furniture(data) => {
                model = data.model.model;
                object_bounds = Some(data.object_bounds);
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

                object_bounds = Some(data.object_bounds);
            }
            RecordData::Ingestible(data) => {
                model = data.model.model;
                object_bounds = Some(data.object_bounds);
            }
            _ => return None,
            x => todo!("{:?}", x),
        }

        let mut nif_flags = LoadNifFlags::empty();
        if flags.contains(PartialEntityFlags::GRAPHICS) {
            nif_flags |= LoadNifFlags::RENDER_OBJECTS;
        }
        if flags.contains(PartialEntityFlags::PHYSICS) {
            nif_flags |= LoadNifFlags::COLLIDERS;
        }

        let scene = match model {
            Some(model) if flags.contains(PartialEntityFlags::SCENE) => {
                let mut path = DataPathBuf::new(model.as_bytes());
                path.push_front("meshes");

                match ctx
                    .loader
                    .load::<NifScene, _>(&NifKey {
                        path,
                        flags: nif_flags,
                    })
                    .await
                {
                    Ok(scene) => Some(scene),
                    Err(err) => {
                        tracing::error!("{}", err);
                        None
                    }
                }
            }
            Some(_) | None => None,
        };

        let object_bounds = object_bounds.map(|b| ObjectBounds {
            min: (Vec3::new(b.min_x as f32, b.min_y as f32, b.min_z as f32) * scale).as_ivec3(),
            max: (Vec3::new(b.max_x as f32, b.max_y as f32, b.max_z as f32) * scale).as_ivec3(),
        });

        let script = if record.local.data.virtual_machine_adapter().is_some() {
            ctx.script_loader.load(record.id).await.map(|v| v.object)
        } else {
            None
        };

        Some(SpawnEntity {
            transform: NiTransform::from_ni_transform(transform),
            scene,
            light: flags
                .contains(PartialEntityFlags::GRAPHICS)
                .then_some(light)
                .flatten(),
            light_flicker: flags
                .contains(PartialEntityFlags::GRAPHICS)
                .then_some(light_flicker)
                .flatten(),
            actor,
            rigid_body: flags
                .contains(PartialEntityFlags::PHYSICS)
                .then_some(RigidBody::FIXED),
            door,
            inventory,
            placed_item: item.map(PlacedItem),
            keywords: ObjectKeywords::new(keywords),
            trigger_box,
            script,
            object_bounds,
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

async fn load_item(ctx: &LoadContext<'_>, record: &GlobalRecord) -> Option<ItemStack> {
    let is_item = match record.local.data {
        RecordData::Ammo(_) => true,
        RecordData::Armor(_) => true,
        RecordData::Book(_) => true,
        RecordData::Ingestible(_) => true,
        RecordData::Key(_) => true,
        RecordData::Note(_) => true,
        RecordData::MiscObject(_) => true,
        RecordData::Weapon(_) => true,
        _ => false,
    };

    if is_item {
        Some(
            resolve_item(ctx.esp_loader, ctx.script_loader, record.id, 1, &[])
                .await
                .remove(0),
        )
    } else {
        None
    }
}
