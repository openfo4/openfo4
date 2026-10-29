use std::collections::HashMap;
use std::time::Duration;

use glam::{EulerRot, Quat, Vec3};
use hashbrown::HashSet;
use openfo4_animation::skeleton::{BoneMarker, SkeletonInstance};
use openfo4_asset::Handle;
use openfo4_common::components::{Color, PointLight, Transform};
use openfo4_core::Frozen;
use openfo4_core::ecs::{Entity, World};
use openfo4_core::events::EventBus;
use openfo4_core::hierarchy::Children;
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::RecordId;
use openfo4_core::transform::ConvertCoordinateSystem;
use openfo4_esp::records::{DoorFlags, LightDataFlags, LockLevel, PositionRotation};
use openfo4_esp::{RecordData, RecordHeaderFlags};
use openfo4_physics::components::{Collider, RigidBody};

use crate::assets::nif::{Instantiate, LoadNifFlags, NIF_TO_METERS, NifKey, NifScene};
use crate::components::{
    ActorMarker, BaseObject, Disabled, NiTransform, ObjectBounds, ObjectLockData, ObjectLockLevel,
    ObjectProperties, PlacedItem, PlacedReference, Root, RootParent, TriggerBox, WorldSpace,
};
use crate::render::RenderObject;
use crate::state::esp::GlobalRecord;
use crate::state::hardcoded::PUNCTUAL_LIGHT_INTENSITY_FACTOR;
use crate::state::level::LoadContext;
use crate::state::level::data_ext::RecordDataExt;
use crate::state::level::items::{LoadItemProps, resolve_item};
use crate::state::level::navmesh::load_navmesh;
use crate::state::level::partial::{LodLevel, PartialEntityFlags};
use crate::state::level::terrain::load_land;
use crate::state::npc::{Actor, ActorInstanceProps, ObjectKeywords};
use crate::state::terrain::{TerrainManager, TerrainRegion};
use crate::systems::door::{Door, DoorState, PortalDoor};
use crate::systems::equipment::{Equipment, equip_item};
use crate::systems::inventory::{Inventory, ItemStack};
use crate::systems::lights::{LightFlicker, LightFlickerMode};
use crate::systems::linked_refs::LinkedRefs;
use crate::systems::pathing::NavMesh;
use crate::systems::scripting::ScriptEntities;
use crate::systems::scripting::loader::ScriptObject;

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

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
    pub object_props: Option<ObjectProperties>,
    pub disabled: bool,
    pub base: Option<RecordId>,
}

impl SpawnEntity {
    pub fn instantiate(
        self,
        world: &mut World,
        events: &mut EventBus,
        terrain_manager: &mut TerrainManager,
        script_entities: &ScriptEntities,
        entity: Entity,
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
            object_props,
            disabled,
            base,
        } = self;

        debug_assert!(base.is_some() || form.is_some());

        let is_actor = actor.is_some();

        // FIXME: Modify this such that both scene and actor are valid.
        if let Some(scene) = scene {
            // Some scenes contain skinned meshes.
            // To make sure we skin these correctly we need to attach a skeleton
            // to the entity and "equip" the scene in the same way we do with actors.

            let mut flags = LoadNifFlags::all();
            if scene.skeleton.is_some() {
                flags.remove(LoadNifFlags::RENDER_OBJECTS);
            }

            let mut bones = HashMap::new();
            scene.instantiate_with_feedback(
                |node, entity| {
                    if let Some(bone) = node.bone {
                        bones.insert(bone, entity);
                    }
                },
                transform.into(),
                world,
                entity,
                flags,
            );

            if let Some(skeleton) = &scene.skeleton {
                for entity in bones.values() {
                    world.insert_one(*entity, BoneMarker).unwrap();
                }

                let skeleton = SkeletonInstance::new(skeleton.clone(), bones);

                let mut equipment = Equipment::default();
                equip_item(entity, &scene, &skeleton, world, &mut equipment, None);

                world.insert(entity, (skeleton, equipment)).unwrap();
            }
        } else if let Some(actor) = actor {
            actor.instantiate(world, events, entity);
        }

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
            // We are creating this `entity` and have exclusive access to
            // the `World`. It is not possible for another thread to observe
            // the `entity` until we return.
            script_entities.set(entity, script.id()).unwrap();

            // Insert into entity to keep the script alive.
            world.insert_one(entity, script).unwrap();
        }

        if let Some(props) = object_props {
            world.insert_one(entity, props).unwrap();
        }

        if disabled {
            world.insert_one(entity, Disabled).unwrap();
        }

        world.insert_one(entity, Frozen::FROZEN).unwrap();

        world.insert_one(entity, Root).unwrap();

        world.insert_one(entity, LinkedRefs::default()).unwrap();
        world.insert_one(entity, keywords).unwrap();

        if let Some(base) = base {
            world
                .insert_one(entity, BaseObject { record: base })
                .unwrap();
        }
        if let Some(form) = form {
            world.insert_one(entity, PlacedReference(form)).unwrap();
        }

        if is_actor {
            world.insert_one(entity, ActorMarker).unwrap();
        }

        if let Some(worldspace) = worldspace {
            let mut queue = vec![entity];
            while let Some(e) = queue.pop() {
                world.insert_one(e, worldspace).unwrap();
                world.insert_one(e, RootParent(entity)).unwrap();

                if let Ok(children) = world.get::<&Children>(e) {
                    queue.extend(&children.0);
                }
            }
        }

        if let Some(door) = door {
            world.insert_one(entity, door).unwrap();
        }

        if let Some(terrain) = terrain {
            terrain_manager.insert(terrain);
        }

        entity
    }

    pub fn null(position: PositionRotation) -> Self {
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
            scale: Vec3::splat(1.0),
        };

        Self {
            transform: NiTransform::from_ni_transform(transform),
            scene: None,
            collider: None,
            object: None,
            rigid_body: None,
            light: None,
            light_flicker: None,
            actor: None,
            worldspace: None,
            form: None,
            door: None,
            inventory: None,
            placed_item: None,
            terrain: None,
            script: None,
            keywords: ObjectKeywords::new(HashSet::new()),
            trigger_box: None,
            navmesh: None,
            object_bounds: None,
            object_props: None,
            disabled: false,
            base: None,
        }
    }

    pub(super) async fn load(
        ctx: &LoadContext<'_>,
        record: &GlobalRecord,
        mut flags: PartialEntityFlags,
    ) -> Option<Self> {
        // Some objects are always rendered with LOD0.
        // This effectively disables distance culling for all objects
        // where the base object is not a `Static`.
        if record.local.flags.contains(RecordHeaderFlags::IS_FULL_LOD) {
            flags.lod = LodLevel::Normal;
        }

        let position;
        let scale;
        let base_object;

        let mut light_mod = LightModification::default();
        let mut door = None;
        let mut trigger_box = None;
        let mut lock_data = None;

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

                if let Some(lock) = object.lock_data {
                    let level = match lock.level {
                        LockLevel::None => None,
                        LockLevel::Novice1 | LockLevel::Novice2 => Some(ObjectLockLevel::Noivce),
                        LockLevel::Advanced => Some(ObjectLockLevel::Advanced),
                        LockLevel::Expert => Some(ObjectLockLevel::Expert),
                        LockLevel::Master => Some(ObjectLockLevel::Master),
                        LockLevel::RequiresTerminal => Some(ObjectLockLevel::RequiresTerminal),
                        LockLevel::RequiresKey => Some(ObjectLockLevel::RequiresKey),
                        LockLevel::Barred => Some(ObjectLockLevel::Barred),
                        LockLevel::Chained => Some(ObjectLockLevel::Chained),
                        LockLevel::Inaccessible => Some(ObjectLockLevel::Inaccessible),
                    };

                    let key = if !lock.key.is_null() {
                        Some(ctx.esp_loader.resolve_form(lock.key, record.id).unwrap())
                    } else {
                        None
                    };

                    lock_data = Some(ObjectLockData { level, key });
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

        let mut light = None;
        let mut light_flicker = None;
        let mut actor = None;
        let mut inventory = None;

        let mut keywords = HashSet::default();
        for keyword in base.local.data.keywords() {
            let keyword = ctx.esp_loader.resolve_form(*keyword, base.id).unwrap();
            keywords.insert(keyword);
        }

        let item = load_item(ctx, &base).await;

        // Only Static records have LODs.
        // All other objects are not visible when LOD0 is not rendered.
        if flags.lod != LodLevel::Normal && !matches!(base.local.data, RecordData::STAT(_)) {
            return None;
        }

        let model = base.local.data.model(flags.lod);
        let object_bounds = base.local.data.object_bounds();

        match &base.local.data {
            RecordData::Door(data) => {
                if data.flags.contains(DoorFlags::AUTOMATIC) {
                    door = door.map(|door| match door {
                        Door::Portal(door) => Door::AutoPortal(door),
                        door => door,
                    });
                } else if door.is_none() {
                    door = Some(Door::Normal(DoorState::Closed));
                }
            }
            RecordData::Container(data) => {
                let mut inv = Inventory::default();

                for item in &data.items {
                    let item_id = ctx.esp_loader.resolve_form(item.item, base.id).unwrap();

                    let items = resolve_item(
                        ctx.esp_loader,
                        ctx.script_loader,
                        item_id,
                        item.count.into(),
                        &[],
                        &LoadItemProps {
                            level: PLAYER_LEVEL,
                            object_template_keywords: HashSet::new(),
                        },
                    )
                    .await;
                    for item in items {
                        inv.insert(item);
                    }
                }

                inventory = Some(inv);
            }
            RecordData::Npc(_) => {
                if flags.actor {
                    let props = ActorInstanceProps {
                        dead: record.local.flags.contains(RecordHeaderFlags::STARTS_DEAD),
                    };

                    if let Some(mut res) = crate::state::npc::spawn_actor(
                        ctx.loader,
                        ctx.esp_loader,
                        ctx.script_loader,
                        base.id,
                        &props,
                    )
                    .await
                    .ok()
                    {
                        keywords = core::mem::take(&mut res.keywords);
                        actor = Some(res);
                    }
                }
            }
            RecordData::IdleMarker(_) => return None,
            RecordData::Light(data) => {
                let intensity = data.fade_value + light_mod.fade;
                let radius = (data.data.radius as f32 + light_mod.radius) * NIF_TO_METERS;

                light = Some(PointLight {
                    color: Color::from_rgb([
                        data.data.color.r as f32 / 255.0,
                        data.data.color.g as f32 / 255.0,
                        data.data.color.b as f32 / 255.0,
                    ]),
                    intensity: intensity * PUNCTUAL_LIGHT_INTENSITY_FACTOR,
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
            _ => {}
        }

        let mut nif_flags = LoadNifFlags::empty();
        if flags.graphics {
            nif_flags |= LoadNifFlags::RENDER_OBJECTS;
            nif_flags |= LoadNifFlags::ANIMATIONS;
        }
        if flags.physics {
            nif_flags |= LoadNifFlags::COLLIDERS;
        }

        let scene = match model {
            Some(model) if flags.scene => {
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

        let script = if record.local.data.virtual_machine_adapter().is_some() && flags.script {
            ctx.script_loader.load(record.id).await.map(|v| v.object)
        } else {
            None
        };

        Some(SpawnEntity {
            transform: NiTransform::from_ni_transform(transform),
            scene,
            light: flags.graphics.then_some(light).flatten(),
            light_flicker: flags.graphics.then_some(light_flicker).flatten(),
            actor,
            rigid_body: flags.physics.then_some(RigidBody::FIXED),
            door,
            inventory,
            placed_item: item.map(PlacedItem),
            keywords: ObjectKeywords::new(keywords),
            trigger_box,
            script,
            object_bounds,
            object_props: Some(ObjectProperties { lock: lock_data }),
            form: Some(record.id),
            base: Some(base.id),
            ..Default::default()
        })
    }

    pub(super) async fn load_base(
        ctx: &LoadContext<'_>,
        record: &GlobalRecord,
        flags: PartialEntityFlags,
    ) -> Option<Self> {
        let scale: f32 = 1.0;

        let mut actor = None;
        let mut inventory = None;

        let mut keywords = HashSet::default();
        for keyword in record.local.data.keywords() {
            let keyword = ctx.esp_loader.resolve_form(*keyword, record.id).unwrap();
            keywords.insert(keyword);
        }

        let item = load_item(ctx, &record).await;

        if flags.lod != LodLevel::Normal
            && !record
                .local
                .flags
                .contains(RecordHeaderFlags::HAS_DISTANT_LOD)
            // FIXME: Lights don't have a distant lod flag and seem
            // to be visible at distance, sometimes. Sometimes they
            // are not visible.
            && !matches!(record.local.data, RecordData::Light(_))
        {
            return None;
        }

        let model = record.local.data.model(flags.lod);
        let object_bounds = record.local.data.object_bounds();

        match &record.local.data {
            RecordData::Door(_data) => {
                todo!();
            }
            RecordData::Container(data) => {
                let mut inv = Inventory::default();

                for item in &data.items {
                    let item_id = ctx.esp_loader.resolve_form(item.item, record.id).unwrap();

                    let items = resolve_item(
                        ctx.esp_loader,
                        ctx.script_loader,
                        item_id,
                        item.count.into(),
                        &[],
                        &LoadItemProps {
                            level: PLAYER_LEVEL,
                            object_template_keywords: HashSet::new(),
                        },
                    )
                    .await;
                    for item in items {
                        inv.insert(item);
                    }
                }

                inventory = Some(inv);
            }
            RecordData::Npc(_) => {
                if flags.actor {
                    let props = ActorInstanceProps {
                        dead: record.local.flags.contains(RecordHeaderFlags::STARTS_DEAD),
                    };

                    if let Some(mut res) = crate::state::npc::spawn_actor(
                        ctx.loader,
                        ctx.esp_loader,
                        ctx.script_loader,
                        record.id,
                        &props,
                    )
                    .await
                    .ok()
                    {
                        keywords = core::mem::take(&mut res.keywords);
                        actor = Some(res);
                    }
                }
            }
            RecordData::IdleMarker(_) => return None,
            RecordData::Light(_data) => {
                todo!();
            }
            _ => {}
        }

        let mut nif_flags = LoadNifFlags::empty();
        if flags.graphics {
            nif_flags |= LoadNifFlags::RENDER_OBJECTS;
            nif_flags |= LoadNifFlags::ANIMATIONS;
        }
        if flags.physics {
            nif_flags |= LoadNifFlags::COLLIDERS;
        }

        let scene = match model {
            Some(model) if flags.scene => {
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

        let script = if record.local.data.virtual_machine_adapter().is_some() && flags.script {
            ctx.script_loader.load_instance(record.id).await
        } else {
            None
        };

        Some(SpawnEntity {
            scene,
            actor,
            rigid_body: flags.physics.then_some(RigidBody::FIXED),
            inventory,
            placed_item: item.map(PlacedItem),
            keywords: ObjectKeywords::new(keywords),
            script,
            object_bounds,
            base: Some(record.id),
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
            resolve_item(
                ctx.esp_loader,
                ctx.script_loader,
                record.id,
                1,
                &[],
                &LoadItemProps {
                    level: PLAYER_LEVEL,
                    object_template_keywords: HashSet::new(),
                },
            )
            .await
            .remove(0),
        )
    } else {
        None
    }
}
