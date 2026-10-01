use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use crossbeam::queue::SegQueue;
use event_listener::EventListener;
use futures_lite::FutureExt;
use game_common::collections::bimap::BiMap;
use game_common::components::Transform;
use hashbrown::HashMap;
use openfo4_core::ecs::Entity;
use openfo4_core::record::RecordId;
use parking_lot::RwLock;

use crate::cell::CellId;
use crate::components::{NiTransform, WorldSpace};
use crate::state::level::partial::{LodLevel, PartialEntityFlags};
use crate::state::level::record::SpawnEntity;
use crate::state::level::{Event, LoadContext, LoaderState};

#[derive(Debug)]
pub struct EntityLoader {
    state: Arc<LoaderState>,
    pub(super) entities: RwLock<EntitySet>,
    pub(super) tick: event_listener::Event,
}

#[derive(Clone, Debug)]
pub(super) struct EntitySet {
    pub entities: BiMap<RecordId, Entity>,
    // Most entities are not tracked, hence we store the refcounts
    // separately to save on memory/cache.
    pub ref_counts: HashMap<Entity, Arc<EntityRefInner>>,
}

impl EntityLoader {
    pub(super) fn new(state: Arc<LoaderState>) -> Self {
        Self {
            state,
            entities: RwLock::new(EntitySet {
                entities: BiMap::new(),
                ref_counts: HashMap::new(),
            }),
            tick: event_listener::Event::new(),
        }
    }

    pub async fn spawn(&self, base: RecordId, info: SpawnEntityInfo) -> Result<EntityRef, ()> {
        let record = self.state.esp_loader.lookup(None, base).await.unwrap();

        // FIXME: What do we use here?
        let cell = CellId::new(0, 0);
        let ctx = LoadContext::new(cell, &self.state);

        let flags = PartialEntityFlags {
            graphics: false,
            physics: false,
            actor: false,
            scene: false,
            script: false,
            require_distant_lod: false,
            lod: LodLevel::Normal,
        };

        let mut entity = SpawnEntity::load_base(&ctx, &record, flags).await.unwrap();
        entity.transform = NiTransform::from(info.transform);
        entity.worldspace = Some(info.worldspace);
        entity.disabled = info.disabled;

        let event = event_listener::Event::with_tag();
        let listen = event.listen();

        self.state.events.push(Event::SpawnEntity {
            record: None,
            entity,
            make_ref: true,
            signal: Some(event),
        });

        let entity = listen.await;

        let entities = self.entities.read();
        let ref_count = entities.ref_counts.get(&entity).unwrap().clone();

        Ok(EntityRef {
            ref_count,
            queue: self.state.events.clone(),
            entity,
        })
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
    pub async fn load(&self, id: RecordId) -> Result<EntityRef, ()> {
        loop {
            let entity = {
                let entities = self.entities.read();
                entities.entities.get_left(&id).copied()
            };

            let Some(entity) = entity else {
                return self.load_single_entity(id).await;
            };

            {
                let entities = self.entities.read();

                if let Some(ref_count) = entities.ref_counts.get(&entity) {
                    if ref_count.ref_count.fetch_add(1, Ordering::Relaxed) != 0 {
                        return Ok(EntityRef {
                            ref_count: ref_count.clone(),
                            queue: self.state.events.clone(),
                            entity,
                        });
                    }

                    // If ref_count == 0, entity deletion has already started.
                    // We must request a new ref from the game loop.
                    // Note that we must wait until the last entity ref sent
                    // the deletion event before we can request it again.
                    while !ref_count.deleted.load(Ordering::Acquire) {}
                }

                self.state.events.push(Event::CreateRef(entity));

                drop(entities);
            }

            // Since we marked the SpawnEntity event with make_ref = true,
            // we must send a DropLastRef event to cleanup any references.
            // This is required if the future is aborted while we wait for the
            // game loop.
            struct WaitForRef<'a> {
                queue: &'a SegQueue<Event>,
                event: EventListener,
                entity: Entity,
                drop_ref: bool,
            }

            impl<'a> Future for WaitForRef<'a> {
                type Output = ();

                fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    self.event.poll(cx)
                }
            }

            impl<'a> Drop for WaitForRef<'a> {
                fn drop(&mut self) {
                    if self.drop_ref {
                        self.queue.push(Event::DropLastRefByEntity(self.entity));
                    }
                }
            }

            let mut fut = WaitForRef {
                queue: &self.state.events,
                event: self.tick.listen(),
                entity,
                drop_ref: true,
            };
            (&mut fut).await;
            fut.drop_ref = false;

            let entities = self.entities.read();

            // The entity may no longer exist if the DropLastRef was executed
            // in the previous tick, but our CreateRef was executed in the next
            // tick.
            // In this case CreateRef is ignored and contains no refcount for our
            // entity.
            let Some(ref_count) = entities.ref_counts.get(&entity) else {
                continue;
            };

            debug_assert_eq!(ref_count.ref_count.load(Ordering::SeqCst), 1);
            debug_assert_eq!(ref_count.deleted.load(Ordering::SeqCst), false);

            return Ok(EntityRef {
                ref_count: ref_count.clone(),
                queue: self.state.events.clone(),
                entity,
            });
        }
    }

    #[tracing::instrument(name = "EntityLoader::load_single_entity", skip_all)]
    async fn load_single_entity(&self, id: RecordId) -> Result<EntityRef, ()> {
        let record = self.state.esp_loader.lookup(None, id).await.unwrap();

        // FIXME: What do we use here?
        let cell = CellId::new(0, 0);
        let ctx = LoadContext::new(cell, &self.state);

        let flags = PartialEntityFlags {
            graphics: false,
            physics: false,
            actor: false,
            scene: false,
            script: false,
            require_distant_lod: false,
            lod: LodLevel::Normal,
        };

        let mut entity = SpawnEntity::load(&ctx, &record, flags).await.unwrap();
        entity.worldspace = Some(self.state.worldspace(id).expect("entity has no worldspace"));

        self.state.events.push(Event::SpawnEntity {
            entity,
            make_ref: true,
            record: Some(record.id),
            signal: None,
        });

        // Now we wait until the game loop synchronizes with our future.
        // Since we marked the SpawnEntity event with make_ref = true,
        // we must send a DropLastRef event to cleanup any references.
        // This is required if the future is aborted while we wait for the
        // game loop.
        struct WaitForRef<'a> {
            queue: &'a SegQueue<Event>,
            event: EventListener,
            record: RecordId,
            drop_ref: bool,
        }

        impl<'a> Future for WaitForRef<'a> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                self.event.poll(cx)
            }
        }

        impl<'a> Drop for WaitForRef<'a> {
            fn drop(&mut self) {
                if self.drop_ref {
                    self.queue.push(Event::DropLastRefByRecord(self.record));
                }
            }
        }

        let mut fut = WaitForRef {
            queue: &self.state.events,
            event: self.tick.listen(),
            record: record.id,
            drop_ref: true,
        };
        (&mut fut).await;
        fut.drop_ref = false;

        let entities = self.entities.read();
        let entity = *entities.entities.get_left(&record.id).unwrap();
        let ref_count = entities.ref_counts.get(&entity).unwrap();

        Ok(EntityRef {
            ref_count: ref_count.clone(),
            queue: self.state.events.clone(),
            entity,
        })
    }
}

/// A reference to an [`Entity`].
///
/// While this `EntityRef` exists, the entity will not be streamed out.
#[derive(Debug)]
pub struct EntityRef {
    ref_count: Arc<EntityRefInner>,
    queue: Arc<SegQueue<Event>>,
    entity: Entity,
}

impl EntityRef {
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

impl Clone for EntityRef {
    fn clone(&self) -> Self {
        self.ref_count.ref_count.fetch_add(1, Ordering::Relaxed);

        Self {
            ref_count: self.ref_count.clone(),
            entity: self.entity,
            queue: self.queue.clone(),
        }
    }
}

impl Drop for EntityRef {
    fn drop(&mut self) {
        if self.ref_count.ref_count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        self.ref_count.ref_count.load(Ordering::Acquire);

        self.queue.push(Event::DropLastRefByEntity(self.entity));

        self.ref_count.deleted.store(true, Ordering::Release);
    }
}

#[derive(Debug)]
pub(super) struct EntityRefInner {
    ref_count: AtomicUsize,
    deleted: AtomicBool,
}

impl EntityRefInner {
    pub(super) fn new_with_one_rc() -> Self {
        Self {
            ref_count: AtomicUsize::new(1),
            deleted: AtomicBool::new(false),
        }
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
}
