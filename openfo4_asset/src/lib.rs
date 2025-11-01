use std::any::TypeId;
use std::borrow::Borrow;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use event_listener::Event;
use hashbrown::HashMap;
use parking_lot::RwLock;

/// A loader for [`Asset`]s.
///
/// The loader contains information to efficiently load [`Asset`]s that have been registered.
#[derive(Clone, Debug, Default)]
pub struct AssetLoader {
    registry: Arc<RwLock<AssetRegistry>>,
}

impl AssetLoader {
    /// Creates a new `AssetLoader`.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(AssetRegistry::default())),
        }
    }

    /// Registers a new [`Asset`] that can be loaded using this `AssetLoader`.
    ///
    /// The `ctx` value is provided to every [`load`] operation for this asset.
    ///
    /// # Panics
    ///
    /// Panics if the same asset `T` was already registered in this `AssetLoader`.
    ///
    /// [`load`]: Asset::load
    pub fn register<T>(&self, ctx: T::Context)
    where
        T: Asset,
    {
        let mut registry = self.registry.write();
        registry.register::<T>(ctx);
    }

    /// Returns a [`Handle`] to an [`Asset`] if it is already loaded. Returns `None` if the asset
    /// is not currently loaded.
    ///
    /// # Panics
    ///
    /// Panics if the asset `T` was not previously registered with [`register`].
    ///
    /// [`register`]: Self::register
    pub fn get<T, Q>(&self, key: &Q) -> Option<Handle<T>>
    where
        T: Asset,
        T::Key: Borrow<Q>,
        Q: Eq + Hash + ToOwned<Owned = T::Key> + ?Sized,
    {
        let registry = self.registry.read();
        let Some(assets) = registry.get::<T>() else {
            panic!("asset {} not registered", std::any::type_name::<T>());
        };

        if let Some(AssetEntry::Loaded(asset)) = assets.assets.get(key) {
            asset.ref_count.fetch_add(1, Ordering::Relaxed);

            Some(Handle {
                registry: self.registry.clone(),
                asset: asset.clone(),
            })
        } else {
            None
        }
    }

    /// Loads an [`Asset`] by its key and returns a [`Handle`] to it.
    ///
    /// It the asset is already loaded it will return a new [`Handle`] to the same underlying
    /// asset. If the asset does not already exist, it will be loaded.
    ///
    /// # Errors
    ///
    /// Returns an error if the assets [`load`] function returns an error.
    ///
    /// # Panics
    ///
    /// Panics if the asset `T` was not previously registered with [`register`].
    ///
    /// [`load`]: Asset::load
    /// [`register`]: Self::register
    pub async fn load<T, Q>(&self, key: &Q) -> Result<Handle<T>, T::Error>
    where
        T: Asset,
        T::Key: Borrow<Q>,
        Q: Eq + Hash + ToOwned<Owned = T::Key> + ?Sized,
    {
        let ctx;

        let mut was_awoken = false;
        loop {
            let listener = {
                let registry = self.registry.read();
                let Some(assets) = registry.get::<T>() else {
                    panic!("asset {} not registered", std::any::type_name::<T>());
                };

                match assets.assets.get(key) {
                    Some(AssetEntry::Loaded(asset)) => {
                        // If we were awoken from another thread that thread
                        // will already have inserted our refcount value.
                        if !was_awoken {
                            asset.ref_count.fetch_add(1, Ordering::Relaxed);
                        }

                        return Ok(Handle {
                            registry: self.registry.clone(),
                            asset: asset.clone(),
                        });
                    }
                    // If another thread is loading the asset the waker exists and
                    // we can synchronize with it.
                    Some(AssetEntry::Loading(waker)) => {
                        // Note that the wake-up operation can only complete once
                        // we drop our read mutex, so there is no race condition
                        // between checking the asset state and `waker.listen()`.
                        waker.listen()
                    }
                    None => {
                        // No waker exists, i.e. no other thread is loading the asset.
                        // Upgrade our lock from readable to writable. Note that this
                        // will fully release the read lock, before acquiring the write
                        // lock so any threads may interleave.
                        drop(registry);
                        let mut registry = self.registry.write();
                        let assets = registry.get_mut::<T>().unwrap();

                        // Note that it is possible for another thread to interlave
                        // and load the entire asset while we upgrade our lock.
                        match assets.assets.get(key) {
                            Some(AssetEntry::Loaded(asset)) => {
                                asset.ref_count.fetch_add(1, Ordering::Relaxed);
                                return Ok(Handle {
                                    registry: self.registry.clone(),
                                    asset: asset.clone(),
                                });
                            }
                            // Note that it is possible for another thread to interlave
                            // and start loading while we upgrade our lock.
                            // In this case we will subscribe to that thread and go back
                            // to sleep.
                            Some(AssetEntry::Loading(waker)) => waker.listen(),
                            None => {
                                // The asset slot is now ours!
                                assets
                                    .assets
                                    .insert(key.to_owned(), AssetEntry::Loading(Event::new()));
                                ctx = registry.context::<T>().unwrap();
                                break;
                            }
                        }
                    }
                }
            };

            listener.await;
            was_awoken = true;
        }

        // Safety: We are allowed to access the value immutably while
        // we do not drop the underlying `AssetRegistry`.
        // The `AssetRegistry` in self is kept alive until this function
        // exits.
        let ctx = unsafe { &*ctx };

        let owned_key = key.to_owned();
        let res = T::load(self, &owned_key, ctx).await;

        let mut registry = self.registry.write();
        // SAFETY: We have already accessed the registry for `T`
        // above and if an asset exists once it can never be removed.
        let assets = unsafe { registry.get_mut::<T>().unwrap_unchecked() };

        let entry = assets.assets.get_mut(key).unwrap();
        let AssetEntry::Loading(waker) = entry else {
            unreachable!();
        };

        // Wake up all threads listening for our result.
        // Note that the number of listeners cannot change while we have
        // an exclusive lock.
        waker.notify(waker.total_listeners());

        let value = match res {
            Ok(value) => value,
            Err(err) => {
                assets.assets.remove(key).unwrap();
                return Err(err);
            }
        };

        // If other threads are waiting for our value we will initialize
        // the refcount with the number that all handles will return.
        // Note that an extra value of 1 is needed for our current thread.
        // The `total_listeners` value can not be modified while we have
        // and exclusive lock.
        let ref_count = AtomicUsize::new(1 + waker.total_listeners());

        let asset = Arc::new(AssetState {
            value,
            key: owned_key.clone(),
            ref_count,
        });

        *entry = AssetEntry::Loaded(asset.clone());

        Ok(Handle {
            registry: self.registry.clone(),
            asset,
        })
    }
}

/// An asset resources that can be loaded from a [`AssetLoader`].
pub trait Asset: Sized + Send + Sync + 'static {
    /// A key type to uniquely identify every asset of this type.
    type Key: Eq + Hash + Clone + Send + Sync + 'static;

    /// An error that can occur while loading this asset.
    type Error;

    /// Addtional context passed to `load`.
    type Context: Send + Sync + 'static;

    /// Loads the asset with the given `key`.
    ///
    /// The `load` function receives access to the [`AssetLoader`] to recursively load other assets.
    fn load(
        loader: &AssetLoader,
        key: &Self::Key,
        ctx: &Self::Context,
    ) -> impl Future<Output = Result<Self, Self::Error>>;
}

pub struct Assets<T>
where
    T: Asset,
{
    assets: HashMap<T::Key, AssetEntry<T>>,
}

enum AssetEntry<T>
where
    T: Asset,
{
    /// The asset is already loaded.
    Loaded(Arc<AssetState<T>>),
    /// The asset is currently loading.
    ///
    /// The event token is used to wake all up all listeners.
    Loading(Event),
}

impl<T> Assets<T>
where
    T: Asset,
{
    fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }
}

#[derive(Debug, Default)]
struct AssetRegistry {
    // Type erased HashMap<TypeId, Box<Assets<T>>
    // where T is Asset assocaited with the TypeId.
    // Also note that the number of asset types should be relatively constant
    // over the entire program duration, so we do not bother removing elements
    // from this outer map.
    assets: HashMap<TypeId, *mut ()>,
    type_infos: HashMap<TypeId, TypeInfo>,
}

impl AssetRegistry {
    fn register<T>(&mut self, ctx: T::Context)
    where
        T: Asset,
    {
        let type_id = TypeId::of::<T>();
        assert!(
            !self.type_infos.contains_key(&type_id),
            "cannot register asset type more than once"
        );

        unsafe fn drop_fn<T>(info: TypeInfo, ptr: *mut ())
        where
            T: Asset,
        {
            let ptr = ptr.cast::<Assets<T>>();

            unsafe {
                drop(Box::from_raw(ptr));
            }

            unsafe {
                let context = info.context.cast::<T::Context>().cast_mut();
                drop(Box::from_raw(context));
            }
        }

        let context = Box::into_raw(Box::new(ctx)).cast::<()>();

        self.type_infos.insert(
            type_id,
            TypeInfo {
                drop_fn: drop_fn::<T>,
                context,
            },
        );

        let boxed = Box::new(Assets::<T>::new());
        let ptr = Box::into_raw(boxed).cast::<()>();

        self.assets.insert(type_id, ptr);
    }

    /// Returns a raw pointer to the context of a specific asset type.
    ///
    /// The pointer is safe to access, even while this `AssetRegistry` is accessed by other methods
    /// as long as the `AssetRegistry` is not dropped while the pointer is alive.
    fn context<T>(&self) -> Option<*const T::Context>
    where
        T: Asset,
    {
        let type_id = TypeId::of::<T>();
        let info = self.type_infos.get(&type_id)?;

        let ptr = info.context.cast::<T::Context>();
        Some(ptr)
    }

    fn get<T>(&self) -> Option<&Assets<T>>
    where
        T: Asset,
    {
        let type_id = TypeId::of::<T>();

        let ptr = *self.assets.get(&type_id)?;
        let ptr = ptr.cast::<Assets<T>>();

        Some(unsafe { &*ptr })
    }

    fn get_mut<T>(&mut self) -> Option<&mut Assets<T>>
    where
        T: Asset,
    {
        let type_id = TypeId::of::<T>();

        let ptr = *self.assets.get(&type_id)?;
        let ptr = ptr.cast::<Assets<T>>();

        Some(unsafe { &mut *ptr })
    }
}

impl Drop for AssetRegistry {
    fn drop(&mut self) {
        for (type_id, ptr) in self.assets.drain() {
            let info = self.type_infos.remove(&type_id).unwrap();

            unsafe {
                (info.drop_fn)(info, ptr);
            }
        }
    }
}

// SAFETY: All resources for assets are required to satisfy `Send`, `Sync` and must
// be static, so we send/share them access threads.
unsafe impl Send for AssetRegistry {}
unsafe impl Sync for AssetRegistry {}

#[derive(Clone, Debug)]
struct TypeInfo {
    /// Drops `HashMap<T::Key, Arc<AssetState<T>>>`.
    drop_fn: unsafe fn(TypeInfo, *mut ()),
    /// Untyped `Box<T::Context>`.
    context: *const (),
}

/// A reference counted handle to some [`Asset`].
///
/// While the `Handle` exists the asset will be kept loaded.
///
/// The asset `T` can be accessed through the [`Deref`] implementation.
pub struct Handle<T>
where
    T: Asset,
{
    registry: Arc<RwLock<AssetRegistry>>,
    asset: Arc<AssetState<T>>,
}

impl<T> Clone for Handle<T>
where
    T: Asset,
{
    fn clone(&self) -> Self {
        self.asset.ref_count.fetch_add(1, Ordering::Relaxed);

        Self {
            registry: self.registry.clone(),
            asset: self.asset.clone(),
        }
    }
}

impl<T> Drop for Handle<T>
where
    T: Asset,
{
    fn drop(&mut self) {
        if self.asset.ref_count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        let mut registry = self.registry.write();

        // Note that it is possible for another thread to
        // increment the refcount just after it hit 0.
        // This is because unlike a Arc we have an additional "weak"
        // reference in the AssetLoader that is not counted.
        // As such we need to check the refcount again AFTER we have
        // acquired the lock, preventing access from the AssetLoader.
        // Only if the refcount is still zero are we able to delete
        // the asset.
        let state = self.asset.ref_count.load(Ordering::Acquire);
        if state != 0 {
            return;
        }

        let assets = unsafe { registry.get_mut::<T>().unwrap_unchecked() };
        assets.assets.remove(&self.asset.key);
    }
}

impl<T> Deref for Handle<T>
where
    T: Asset,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.asset.value
    }
}

impl<T> Debug for Handle<T>
where
    T: Asset + Debug,
    T::Key: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { registry, asset } = &self;

        f.debug_struct("Handle")
            .field("registry", registry)
            .field("asset", asset)
            .finish()
    }
}

#[derive(Debug)]
struct AssetState<T>
where
    T: Asset,
{
    value: T,
    key: T::Key,
    ref_count: AtomicUsize,
}

#[derive(Clone)]
pub enum HandleOrAsset<T>
where
    T: Asset,
{
    Asset(T),
    Handle(Handle<T>),
}

impl<T> Deref for HandleOrAsset<T>
where
    T: Asset,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Asset(value) => value,
            Self::Handle(handle) => handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::sync::Once;

    use futures_lite::future::block_on;

    use crate::{Asset, AssetLoader};

    struct AssetA {
        value: String,
    }

    impl Asset for AssetA {
        type Error = Infallible;
        type Key = String;
        type Context = ();

        async fn load(
            _loader: &AssetLoader,
            key: &Self::Key,
            _ctx: &Self::Context,
        ) -> Result<Self, Self::Error> {
            Ok(Self {
                value: key.to_owned(),
            })
        }
    }

    /// Asset that can not be loaded more than once.
    struct OnceAsset;

    impl Asset for OnceAsset {
        type Error = Infallible;
        type Key = ();
        type Context = ();

        async fn load(
            _loader: &AssetLoader,
            _key: &Self::Key,
            _ctx: &Self::Context,
        ) -> Result<Self, Self::Error> {
            static ONCE: Once = Once::new();

            if ONCE.is_completed() {
                panic!("OnceAsset was loaded more than once")
            }

            ONCE.call_once(|| {});
            Ok(Self)
        }
    }

    #[test]
    fn insert_assets() {
        let loader = AssetLoader::new();
        loader.register::<AssetA>(());

        let a = block_on(loader.load::<AssetA, _>("Hello World")).unwrap();
        assert_eq!(a.value, "Hello World");
    }

    #[test]
    fn asset_caching() {
        let loader = AssetLoader::new();
        loader.register::<OnceAsset>(());

        let a = block_on(loader.load::<OnceAsset, _>(&())).unwrap();
        let b = block_on(loader.load::<OnceAsset, _>(&())).unwrap();

        drop(a);
        drop(b);
    }

    #[test]
    fn load_with_context() {
        #[derive(Clone, Debug)]
        struct AssetWithContext(String);

        impl Asset for AssetWithContext {
            type Key = ();
            type Error = Infallible;
            type Context = String;

            async fn load(
                _loader: &AssetLoader,
                _key: &Self::Key,
                ctx: &Self::Context,
            ) -> Result<Self, Self::Error> {
                Ok(Self(ctx.clone()))
            }
        }

        let ctx = "with_context".to_owned();

        let loader = AssetLoader::new();
        loader.register::<AssetWithContext>(ctx.clone());

        let a = block_on(loader.load::<AssetWithContext, _>(&())).unwrap();
        assert_eq!(a.0, ctx);
    }

    // Miri test to ensure panics in the load function
    // do not break invariants.
    #[test]
    #[should_panic]
    fn asset_load_panic() {
        struct PanickingAsset;

        impl Asset for PanickingAsset {
            type Key = String;
            type Error = Infallible;
            type Context = String;

            async fn load(
                _loader: &AssetLoader,
                _key: &Self::Key,
                _ctx: &Self::Context,
            ) -> Result<Self, Self::Error> {
                panic!()
            }
        }

        let loader = AssetLoader::new();
        loader.register::<PanickingAsset>("test".to_owned());

        block_on(loader.load::<PanickingAsset, _>("hello world")).unwrap();
    }
}
