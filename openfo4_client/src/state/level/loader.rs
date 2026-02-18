use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use crossbeam::queue::SegQueue;
use event_listener::EventListener;
use futures_lite::FutureExt;
use hashbrown::HashMap;
use hecs::Entity;
use openfo4_asset::AssetLoader;
use openfo4_core::record::RecordId;
use parking_lot::RwLock;

use crate::cell::CellId;
use crate::loader::FileLoader;
use crate::shared::SharedState;
use crate::state::esp::EspLoader;
use crate::state::level::partial::PartialEntityFlags;
use crate::state::level::record::SpawnEntity;
use crate::state::level::{Event, LoadContext};
use crate::systems::scripting::loader::ScriptLoader;

#[derive(Debug)]
pub struct EntityLoader {
    esp_loader: Arc<EspLoader>,
    file_loader: Arc<FileLoader>,
    shared: Arc<SharedState>,
    asset_loader: AssetLoader,
    script_loader: Arc<ScriptLoader>,
    pub(super) entities: RwLock<EntitySet>,
    events: Arc<SegQueue<Event>>,
    tick: event_listener::Event,
}

#[derive(Clone, Debug)]
pub(super) struct EntitySet {
    pub entities: HashMap<RecordId, Entity>,
    // Most entities are not tracked, hence we store the refcounts
    // separately to save on memory/cache.
    pub ref_counts: HashMap<Entity, Arc<EntityRefInner>>,
}

impl EntityLoader {
    pub(super) fn new(
        esp_loader: Arc<EspLoader>,
        file_loader: Arc<FileLoader>,
        shared: Arc<SharedState>,
        asset_loader: AssetLoader,
        script_loader: Arc<ScriptLoader>,
        events: Arc<SegQueue<Event>>,
    ) -> Self {
        Self {
            esp_loader,
            file_loader,
            shared,
            asset_loader,
            script_loader,
            events,
            entities: RwLock::new(EntitySet {
                entities: HashMap::new(),
                ref_counts: HashMap::new(),
            }),
            tick: event_listener::Event::new(),
        }
    }

    /// Returns the entity of the given [`RecordId`].
    ///
    /// When the entity is not spawned yet, it will be spawned before this function returns. The
    /// returned [`EntityRef`] will keep the entity alive until it it dropped, even if the entity
    /// is not in the active streaming area.
    pub async fn load(&self, id: RecordId) -> Result<EntityRef, ()> {
        let mut entities = self.entities.read();

        loop {
            let Some(entity) = entities.entities.get(&id).copied() else {
                drop(entities);
                return self.load_single_entity(id).await;
            };

            if let Some(ref_count) = entities.ref_counts.get(&entity) {
                if ref_count.ref_count.fetch_add(1, Ordering::Relaxed) != 0 {
                    return Ok(EntityRef {
                        ref_count: ref_count.clone(),
                        queue: self.events.clone(),
                        entity,
                    });
                }

                // If ref_count == 0, entity deletion has already started.
                // We must request a new ref from the game loop.
                // Note that we must wait until the last entity ref sent
                // the deletion event before we can request it again.
                while !ref_count.deleted.load(Ordering::Acquire) {}
            }

            self.events.push(Event::CreateRef(entity));

            drop(entities);

            // Since we marked the SpawnEntity event with make_ref = true,
            // we must send a DropLastRef event to cleanup any references.
            // This is required if the future is aborted while we wait for the
            // game loop.
            struct WaitForRef<'a> {
                queue: &'a SegQueue<Event>,
                event: EventListener,
                entity: Entity,
            }

            impl<'a> Future for WaitForRef<'a> {
                type Output = ();

                fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    self.event.poll(cx)
                }
            }

            impl<'a> Drop for WaitForRef<'a> {
                fn drop(&mut self) {
                    self.queue.push(Event::DropLastRefByEntity(self.entity));
                }
            }

            WaitForRef {
                queue: &self.events,
                event: self.tick.listen(),
                entity,
            }
            .await;

            entities = self.entities.read();

            // The entity may no longer exist if the DropLastRef was executed
            // in the previous tick, but our CreateRef was executed in the next
            // tick.
            // In this case CreateRef is ignored and contains no refcount for our
            // entity.
            let Some(ref_count) = entities.ref_counts.get(&entity) else {
                continue;
            };

            return Ok(EntityRef {
                ref_count: ref_count.clone(),
                queue: self.events.clone(),
                entity,
            });
        }
    }

    async fn load_single_entity(&self, id: RecordId) -> Result<EntityRef, ()> {
        let record = self.esp_loader.lookup(None, id).await.unwrap();

        let ctx = LoadContext {
            // FIXME: What do we use here?
            cell: CellId::new(0, 0),
            esp_loader: &self.esp_loader,
            file_loader: &self.file_loader,
            script_loader: &self.script_loader,
            loader: &self.asset_loader,
            shared: &self.shared,
        };

        let flags = PartialEntityFlags::empty();

        let entity = SpawnEntity::load(&ctx, &record, flags).await.unwrap();

        self.events.push(Event::SpawnEntity {
            entity,
            make_ref: true,
            record: record.id,
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
        }

        impl<'a> Future for WaitForRef<'a> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                self.event.poll(cx)
            }
        }

        impl<'a> Drop for WaitForRef<'a> {
            fn drop(&mut self) {
                self.queue.push(Event::DropLastRefByRecord(self.record));
            }
        }

        let fut = WaitForRef {
            queue: &self.events,
            event: self.tick.listen(),
            record: record.id,
        };
        fut.await;

        let entities = self.entities.read();
        let entity = *entities.entities.get(&record.id).unwrap();
        let ref_count = entities.ref_counts.get(&entity).unwrap();

        Ok(EntityRef {
            ref_count: ref_count.clone(),
            queue: self.events.clone(),
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
