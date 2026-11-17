use std::sync::Arc;

use event_listener::EventListener;
use glam::Vec3;
use openfo4_common::collections::slab::Slab;
use openfo4_common::components::Transform;
use openfo4_common::utils::AbortOnDrop;
use openfo4_core::ecs::Entity;
use openfo4_core::record::RecordId;

use crate::cell::CellId;
use crate::components::{NiTransform, WorldSpace};
use crate::state::esp::GlobalRecord;
use crate::state::level::data_ext::RecordDataExt;
use crate::state::level::partial::PartialEntityFlags;
use crate::state::level::record::SpawnEntity;
use crate::state::level::{
    EntityId, EntityState, Event, LoadContext, LoaderState, PartialEntityFlagCounts,
};

#[derive(Debug)]
pub struct EntityLoader {
    pub(super) state: Arc<LoaderState>,
}

impl EntityLoader {
    pub(super) fn new(state: Arc<LoaderState>) -> Self {
        Self { state }
    }

    #[tracing::instrument(name = "EntityLoader::spawn", skip_all)]
    pub async fn spawn(&self, base: RecordId, info: SpawnEntityInfo) -> Result<EntityRef, ()> {
        let record = self.state.esp_loader.lookup(None, base).await.unwrap();

        let cell = CellId::from(NiTransform::from(info.transform));
        let ctx = LoadContext::new(cell, &self.state);

        let mut entity = SpawnEntity::load_base(&ctx, &record, info.flags)
            .await
            .unwrap();
        entity.transform = NiTransform::from(info.transform);
        entity.worldspace = Some(info.worldspace);
        entity.disabled = info.disabled;

        let mut loading = Slab::new();
        let event = event_listener::Event::new();
        let listener = event.listen();
        let index = loading.insert((info.flags, event));

        let id;
        {
            let mut entities = self.state.entities.lock();
            id = entities.entities.insert(EntityState {
                flags: PartialEntityFlagCounts::default() + info.flags,
                entity: None,
                ref_count: 1,
                loading,
                record: None,
            });

            self.state.events.push(Event::Spawn { id, entity, index });

            drop(entities);
        }

        self.wait_until_loaded(id, info.flags, listener).await;

        let mut entities = self.state.entities.lock();
        let state = entities.entities.get_mut(id).unwrap();
        let entity = state.entity.unwrap();

        Ok(EntityRef(Arc::new(EntityRefInner {
            id,
            entity,
            flags: info.flags,
            state: Arc::clone(&self.state),
        })))
    }

    /// Returns the entity of the given [`RecordId`].
    ///
    /// When the entity is not spawned yet, it will be spawned before this function returns. The
    /// returned [`EntityRef`] will keep the entity alive until it it dropped, even if the entity
    /// is not in the active streaming area.
    ///
    /// In addition note that the game loop must make progress in case this future suspends. If
    /// that is not the case, the future might never resolve.
    #[tracing::instrument(name = "EntityLoader::load", skip_all)]
    pub async fn load(&self, id: RecordId, flags: PartialEntityFlags) -> Result<EntityRef, ()> {
        let worldspace = self.state.worldspace(id).unwrap();

        self.load_custom(id, flags, async move || {
            let record = self.state.esp_loader.lookup(None, id).await.unwrap();
            let position = record.local.data.position().unwrap();
            let translation = Vec3::new(
                position.position_x,
                position.position_y,
                position.position_z,
            );
            let cell = CellId::from(NiTransform::from_translation(translation));

            CustomRecordInfo {
                record,
                worldspace,
                cell,
            }
        })
        .await
    }

    pub(super) async fn load_custom<F, Fut>(
        &self,
        id: RecordId,
        flags: PartialEntityFlags,
        get_record: F,
    ) -> Result<EntityRef, ()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = CustomRecordInfo>,
    {
        let mut do_load = true;
        let mut index = 0;
        let eid;
        let mut listener = None;
        {
            let entities = &mut *self.state.entities.lock();

            eid = *entities.records.entry(id).or_insert_with(|| {
                entities.entities.insert(EntityState {
                    flags: PartialEntityFlagCounts::default(),
                    entity: None,
                    ref_count: 0,
                    loading: Slab::new(),
                    record: Some(id),
                })
            });

            let state = entities.entities.get_mut(eid).unwrap();

            // Our requested flags are already loaded.
            if let Some(entity) = state.entity
                && state.flags.flags().is_subset(&flags)
            {
                state.ref_count += 1;
                state.flags += flags;

                return Ok(EntityRef(Arc::new(EntityRefInner {
                    id: eid,
                    entity,
                    flags,
                    state: Arc::clone(&self.state),
                })));
            }

            for (_, (cur_flags, event)) in &state.loading {
                // Another future is already loading the flags that we need.
                if cur_flags.is_subset(&flags) {
                    state.ref_count += 1;
                    state.flags += flags;

                    listener = Some(event.listen());

                    do_load = false;
                    break;
                }
            }

            if do_load {
                state.ref_count += 1;
                state.flags += flags;

                let event = event_listener::Event::new();
                listener = Some(event.listen());
                index = state.loading.insert((flags, event));
            }
        }

        if do_load {
            let info = get_record().await;

            let ctx = LoadContext::new(info.cell, &self.state);
            let mut entity = SpawnEntity::load(&ctx, &info.record, flags)
                .await
                .unwrap_or_else(|| SpawnEntity::null(info.record.local.data.position().unwrap()));
            entity.worldspace = Some(info.worldspace);
            entity.form = Some(id);
            self.state.events.push(Event::Spawn {
                id: eid,
                entity,
                index,
            });
        }

        let listener = listener.unwrap();

        let entity = self.wait_until_loaded(eid, flags, listener).await;
        Ok(entity)
    }

    /// Wait until the given [`EntityId`] has been spawned by the [`Level`] after an [`Spawn`]
    /// event was sent.
    ///
    /// [`Level`]: super::Level
    /// [`Spawn`]: Event::Spawn
    fn wait_until_loaded(
        &self,
        id: EntityId,
        flags: PartialEntityFlags,
        listener: EventListener,
    ) -> impl Future<Output = EntityRef> {
        AbortOnDrop::new(
            async move {
                listener.await;

                let mut entities = self.state.entities.lock();
                let state = entities.entities.get_mut(id).unwrap();

                let entity = state.entity.unwrap();
                EntityRef(Arc::new(EntityRefInner {
                    id,
                    entity,
                    flags,
                    state: Arc::clone(&self.state),
                }))
            },
            move || {
                // We have already send a `Spawn` and the entity may already have been spawned
                // so we must abort the same way as Dropping an EntityRef.
                discard_entity_ref(id, flags, &self.state);
            },
        )
    }
}

/// A reference to an [`Entity`].
///
/// While this `EntityRef` exists, the entity will not be streamed out.
#[derive(Clone, Debug)]
pub struct EntityRef(Arc<EntityRefInner>);

impl EntityRef {
    pub fn entity(&self) -> Entity {
        self.0.entity
    }
}

#[derive(Debug)]
pub struct EntityRefInner {
    id: EntityId,
    entity: Entity,
    flags: PartialEntityFlags,
    state: Arc<LoaderState>,
}

impl Drop for EntityRefInner {
    fn drop(&mut self) {
        discard_entity_ref(self.id, self.flags, &self.state);
    }
}

#[derive(Clone, Debug)]
pub struct SpawnEntityInfo {
    pub transform: Transform,
    pub worldspace: WorldSpace,
    /// Flag the new entity as disabled on creation.
    pub disabled: bool,
    /// Flag the new entity as persistent.
    pub persistent: bool,
    pub flags: PartialEntityFlags,
}

fn discard_entity_ref(id: EntityId, flags: PartialEntityFlags, lstate: &LoaderState) {
    let mut entities = lstate.entities.lock();
    let state = entities.entities.get_mut(id).unwrap();

    state.ref_count -= 1;
    if state.ref_count == 0 {
        lstate.events.push(Event::Despawn { id });

        if let Some(record) = state.record {
            entities.records.remove(&record);
        }

        return;
    }

    let prev_flags = state.flags.flags();
    state.flags -= flags;
    if state.flags.flags() == prev_flags {
        return;
    }

    // TODO: "Downgrade" the entity by dropping some data.
}

#[derive(Clone, Debug)]
pub(super) struct CustomRecordInfo {
    pub record: GlobalRecord,
    pub worldspace: WorldSpace,
    pub cell: CellId,
}
