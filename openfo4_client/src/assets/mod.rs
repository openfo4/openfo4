mod bgsm;
mod hkx;
mod image;
mod nif;
mod scene;

pub mod material;
pub mod nif_v2;
pub mod texture;

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use game_core_pipeline::entities::Entities;
use game_tasks::TaskPool;
use parking_lot::{Mutex, RwLock};

use crate::loader::{FileLoader, Reader};

pub struct FileLoadContext {
    pub server: AssetServer,
    pub entities: Entities,
}

#[derive(Debug)]
pub struct Error(String);

impl<T> From<T> for Error
where
    T: std::error::Error,
{
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct AssetServer {
    loader: Arc<FileLoader>,
    pool: Arc<TaskPool>,
    assets: Arc<RwLock<HashMap<AssetKey<'static>, Arc<AssetState>>>>,
}

impl AssetServer {
    pub fn new(loader: Arc<FileLoader>, pool: Arc<TaskPool>) -> Self {
        Self {
            loader,
            pool,
            assets: Arc::default(),
        }
    }

    pub async fn load<T>(&self, path: &str) -> Result<Handle<T>, Error>
    where
        T: Asset,
    {
        let key = AssetKey {
            type_id: TypeId::of::<T>(),
            path: Cow::Borrowed(path),
        };

        {
            let assets = self.assets.read();

            if let Some(state) = assets.get(&key) {
                state.ref_count.fetch_add(1, Ordering::Relaxed);

                return Ok(Handle {
                    state: state.clone(),
                    assets: self.assets.clone(),
                    _marker: PhantomData,
                });
            }
        }

        let mut reader = self.loader.open(path)?;

        let key = AssetKey {
            type_id: TypeId::of::<T>(),
            path: Cow::Owned(path.to_owned()),
        };

        let asset = T::load(self, &mut reader).await?;

        let mut assets = self.assets.write();

        // Note that it is possible for another thread to have loaded the exact
        // same asset at the same time as us.
        // FIXME: For now we drop our asset and use theirs instead, but in the future
        // we should block all other threads while we load the asset (asynchronously).
        if let Some(state) = assets.get(&key) {
            state.ref_count.fetch_add(1, Ordering::Relaxed);

            return Ok(Handle {
                state: state.clone(),
                assets: self.assets.clone(),
                _marker: PhantomData,
            });
        }

        let state = Arc::new(AssetState {
            value: UntypedAsset(Box::new(asset)),
            // One reference for the handle we will return.
            ref_count: AtomicUsize::new(1),
            key: key.clone(),
        });

        assets.insert(key, state.clone());

        Ok(Handle {
            state,
            assets: self.assets.clone(),
            _marker: PhantomData,
        })
    }
}

pub trait Asset: Any + Send + Sync + 'static + Sized {
    async fn load(server: &AssetServer, reader: &mut Reader) -> Result<Self, Error>;
}

#[derive(Debug)]
pub struct Handle<T>
where
    T: Asset,
{
    state: Arc<AssetState>,
    assets: Arc<RwLock<HashMap<AssetKey<'static>, Arc<AssetState>>>>,
    _marker: PhantomData<T>,
}

impl<T> Clone for Handle<T>
where
    T: Asset,
{
    fn clone(&self) -> Self {
        self.state.ref_count.fetch_add(1, Ordering::Relaxed);

        Self {
            state: self.state.clone(),
            assets: self.assets.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for Handle<T>
where
    T: Asset,
{
    fn drop(&mut self) {
        if self.state.ref_count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        let mut assets = self.assets.write();

        // Note that it is possible for another thread to
        // increment the refcount just after it hit 0.
        // This is because unlike a Arc we have an additional "weak"
        // reference in the AssetServer that is not counted.
        // As such we need to check the refcount again AFTER we have
        // acquired the lock, preventing access from the AssetServer.
        // Only if the refcount is still zero are we able to delete
        // the asset.
        let state = self.state.ref_count.load(Ordering::Acquire);
        if state != 0 {
            return;
        }

        assets.remove(&self.state.key);
    }
}

impl<T> Deref for Handle<T>
where
    T: Asset,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.state.value.0.downcast_ref().unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct AssetKey<'a> {
    type_id: TypeId,
    path: Cow<'a, str>,
}

#[derive(Debug)]
struct AssetState {
    value: UntypedAsset,
    key: AssetKey<'static>,
    ref_count: AtomicUsize,
}

struct UntypedAsset(Box<dyn Any + Send + Sync + 'static>);

impl Debug for UntypedAsset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(UntypedAsset))
            .finish_non_exhaustive()
    }
}
