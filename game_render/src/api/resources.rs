use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_queue::SegQueue;
use game_common::collections::slab;
use game_common::collections::vec_map::{Index, VecMap};
use hashbrown::HashMap;
use parking_lot::{Mutex, MutexGuard};
use sharded_slab::Slab;

use crate::api::BufferDescriptor;
use crate::backend::descriptors::DescriptorSetAllocator;
use crate::backend::vulkan::Device;
use crate::backend::{AccessFlags, DescriptorBinding, TextureDescriptor, vulkan};

use super::{BindingMap, DeletionEvent, RawTextureView, ResourceId, ResourceMap};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(usize);

impl Index for BufferId {
    fn index(&self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(usize);

impl Index for TextureId {
    fn index(&self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DescriptorSetId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SamplerId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PipelineId(pub usize);

#[derive(Debug)]
pub struct Resources {
    pub samplers: Slab<SamplerInner>,
    pub descriptor_set_layouts: Slab<DescriptorSetLayoutInner>,
    pub descriptor_sets: Slab<DescriptorSetInner>,
    pub pipelines: Slab<PipelineInner>,
    pub descriptor_allocator: DescriptorSetAllocator,
    pub deletion_queue: SegQueue<DeletionEvent>,
    /// Staging memory used to `WriteBuffer` commands.
    pub staging_memory: Mutex<Vec<u8>>,
    // FIXME: These can go behind the same lock.
    buffers: Mutex<slab::Slab<BufferDescriptor>>,
    buffer_rcs: Mutex<slab::Slab<RefCount>>,
    textures: Mutex<slab::Slab<TextureDescriptor>>,
    buffer_access: Mutex<slab::Slab<AccessFlags>>,
    texture_rcs: Mutex<slab::Slab<RefCount>>,
    // FIXME: Can we flatten this array?
    texture_access: Mutex<slab::Slab<Vec<AccessFlags>>>,
    pub cache: Mutex<CachePool>,
}

impl Resources {
    pub fn new(device: Device) -> Self {
        Self {
            pipelines: Slab::new(),
            descriptor_sets: Slab::new(),
            descriptor_set_layouts: Slab::new(),
            descriptor_allocator: DescriptorSetAllocator::new(device),
            samplers: Slab::new(),
            deletion_queue: SegQueue::new(),
            staging_memory: Mutex::new(Vec::new()),
            buffers: Mutex::new(slab::Slab::new()),
            buffer_rcs: Mutex::new(slab::Slab::new()),
            buffer_access: Mutex::new(slab::Slab::new()),
            textures: Mutex::new(slab::Slab::new()),
            texture_rcs: Mutex::new(slab::Slab::new()),
            texture_access: Mutex::new(slab::Slab::new()),
            cache: Mutex::new(CachePool::default()),
        }
    }

    pub fn create_buffer(&self, descriptor: BufferDescriptor) -> BufferId {
        let mut buffers = self.buffers.lock();
        let mut buffer_rcs = self.buffer_rcs.lock();
        let mut buffer_access = self.buffer_access.lock();

        let index0 = buffer_rcs.insert(RefCount::new());
        let index1 = buffer_access.insert(AccessFlags::empty());
        let index2 = buffers.insert(descriptor);
        debug_assert_eq!(index0, index1);
        debug_assert_eq!(index0, index2);

        BufferId(index0)
    }

    pub fn buffer_descriptor(&self, id: BufferId) -> BufferDescriptor {
        let buffers = self.buffers.lock();
        *buffers.get(id.0).unwrap()
    }

    pub fn with_buffer_access_flags<T, F>(&self, id: BufferId, f: F) -> T
    where
        F: FnOnce(&mut AccessFlags) -> T,
    {
        let mut buffers = self.buffer_access.lock();
        f(buffers.get_mut(id.0).unwrap())
    }

    pub fn with_buffer_rc<T, F>(&self, id: BufferId, f: F) -> T
    where
        F: FnOnce(&RefCount) -> T,
    {
        let buffers = self.buffer_rcs.lock();
        let ref_count = buffers.get(id.0).unwrap();
        f(&ref_count)
    }

    pub fn destroy_buffer(&self, id: BufferId) {
        let mut buffers = self.buffers.lock();
        let mut buffer_rcs = self.buffer_rcs.lock();
        let mut buffer_access = self.buffer_access.lock();

        buffers.remove(id.0);
        buffer_rcs.remove(id.0);
        buffer_access.remove(id.0);
    }

    pub fn create_texture(&self, descriptor: TextureDescriptor) -> TextureId {
        let mut textures = self.textures.lock();
        let mut texture_rcs = self.texture_rcs.lock();
        let mut texture_access = self.texture_access.lock();

        let index0 = texture_rcs.insert(RefCount::new());
        let index1 =
            texture_access.insert(vec![AccessFlags::empty(); descriptor.mip_levels as usize]);
        let index2 = textures.insert(descriptor);
        debug_assert_eq!(index0, index1);
        debug_assert_eq!(index0, index2);

        TextureId(index0)
    }

    pub fn texture_descriptor(&self, id: TextureId) -> TextureDescriptor {
        let textures = self.textures.lock();
        *textures.get(id.0).unwrap()
    }

    pub fn with_texture_access_flags<T, F>(&self, id: TextureId, f: F) -> T
    where
        F: FnOnce(&mut [AccessFlags]) -> T,
    {
        let mut textures = self.texture_access.lock();
        f(textures.get_mut(id.0).unwrap())
    }

    pub fn with_texture_rc<T, F>(&self, id: TextureId, f: F) -> T
    where
        F: FnOnce(&RefCount) -> T,
    {
        let textures = self.texture_rcs.lock();
        let ref_count = textures.get(id.0).unwrap();
        f(ref_count)
    }

    pub fn destroy_texture(&self, id: TextureId) {
        let mut textures = self.textures.lock();
        let mut texture_rcs = self.texture_rcs.lock();
        let mut texture_access = self.texture_access.lock();

        textures.remove(id.0);
        texture_rcs.remove(id.0);
        texture_access.remove(id.0);
    }

    pub fn access_view_mut(&self) -> ResourceAccessViewMut<'_> {
        ResourceAccessViewMut {
            buffers: self.buffer_access.lock(),
            textures: self.texture_access.lock(),
        }
    }
}

/// View into [`Resources`] that provides exclusive access to [`AccessFlags`] of all resources.
pub struct ResourceAccessViewMut<'a> {
    buffers: MutexGuard<'a, slab::Slab<AccessFlags>>,
    textures: MutexGuard<'a, slab::Slab<Vec<AccessFlags>>>,
}

impl<'a> ResourceMap for ResourceAccessViewMut<'a> {
    type Id = ResourceId;

    fn access(&self, id: Self::Id) -> AccessFlags {
        match id {
            ResourceId::Buffer(buffer) => unsafe { *self.buffers.get_unchecked(buffer.0) },
            ResourceId::Texture { id, mip_level } => unsafe {
                *self
                    .textures
                    .get_unchecked(id.0)
                    .get_unchecked(mip_level as usize)
            },
        }
    }

    fn set_access(&mut self, id: Self::Id, access: AccessFlags) {
        match id {
            ResourceId::Buffer(buffer) => unsafe {
                *self.buffers.get_unchecked_mut(buffer.0) = access;
            },
            ResourceId::Texture { id, mip_level } => unsafe {
                *self
                    .textures
                    .get_unchecked_mut(id.0)
                    .get_unchecked_mut(mip_level as usize) = access;
            },
        }
    }
}

#[derive(Debug)]
pub struct SamplerInner {
    pub inner: vulkan::Sampler,
    pub ref_count: RefCount,
}

#[derive(Debug)]
pub struct DescriptorSetLayoutInner {
    pub inner: vulkan::DescriptorSetLayout,
    pub ref_count: RefCount,
    pub bindings: HashMap<u32, DescriptorBinding>,
}

#[derive(Debug)]
pub struct DescriptorSetInner {
    pub bindings: VecMap<u32, DescriptorSetResource>,
    pub num_buffers: u32,
    pub num_samplers: u32,
    pub num_sampled_texture_arrays: u32,
    pub num_storage_texture_arrays: u32,
    pub layout: DescriptorSetLayoutId,
    pub ref_count: RefCount,
}

/// A resource that can be bound to a descriptor set.
#[derive(Debug)]
pub enum DescriptorSetResource {
    UniformBuffer(BufferId),
    StorageBuffer(BufferId),
    Sampler(SamplerId),
    SampledTexture(Vec<RawTextureView>),
    StorageTexture(Vec<RawTextureView>),
}

#[derive(Debug)]
pub struct PipelineInner {
    pub inner: Arc<vulkan::Pipeline>,
    pub bindings: BindingMap,
    pub ref_count: RefCount,
}

#[derive(Debug)]
pub struct RefCount {
    // We maintain two separate counters.
    // The external counter indicates the number resource handles accessible
    // from user code.
    // The internal counter indicates the total number of resource references.
    // Every external reference also is a internal reference.
    internal: AtomicUsize,
    external: AtomicUsize,
}

impl RefCount {
    pub const fn new() -> Self {
        Self {
            internal: AtomicUsize::new(1),
            external: AtomicUsize::new(1),
        }
    }
}

impl RefCount {
    pub fn external(&self) -> usize {
        self.external.load(Ordering::Relaxed)
    }

    pub fn internal(&self) -> usize {
        self.internal.load(Ordering::Relaxed)
    }

    pub fn increment_internal(&self) {
        self.internal.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_external(&self) {
        self.internal.fetch_add(1, Ordering::Relaxed);
        self.external.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the reference counter and returns whether the refcount was the last one.
    #[must_use]
    pub fn decrement_internal(&self) -> bool {
        self.decrement_many_internal(1)
    }

    #[must_use]
    pub fn decrement_many_internal(&self, count: usize) -> bool {
        if self.internal.fetch_sub(count, Ordering::Release) != count {
            return false;
        }

        self.internal.load(Ordering::Acquire);
        true
    }

    /// Decrements the reference counter and returns whether the refcount was the last one.
    #[must_use]
    pub fn decrement_external(&self) -> bool {
        // The following Release store will order the external
        // update before the internal update.
        self.external.fetch_sub(1, Ordering::Relaxed);
        self.decrement_many_internal(1)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CachePool {
    buffers: HashMap<BufferDescriptor, Vec<BufferId>>,
    textures: HashMap<TextureDescriptor, Vec<TextureId>>,
}

impl CachePool {
    pub fn insert_buffer(&mut self, descriptor: BufferDescriptor, id: BufferId) {
        self.buffers.entry(descriptor).or_default().push(id);
    }

    pub fn take_buffer(&mut self, descriptor: &BufferDescriptor) -> Option<BufferId> {
        self.buffers.get_mut(descriptor).and_then(|vec| vec.pop())
    }

    pub fn insert_texture(&mut self, descriptor: TextureDescriptor, id: TextureId) {
        self.textures.entry(descriptor).or_default().push(id);
    }

    pub fn take_texture(&mut self, descriptor: &TextureDescriptor) -> Option<TextureId> {
        self.textures.get_mut(descriptor).and_then(|vec| vec.pop())
    }

    pub fn drain_buffers(&mut self) -> impl Iterator<Item = BufferId> {
        self.buffers.drain().map(|(_, v)| v.into_iter()).flatten()
    }

    pub fn drain_textures(&mut self) -> impl Iterator<Item = TextureId> {
        self.textures.drain().map(|(_, v)| v.into_iter()).flatten()
    }
}
