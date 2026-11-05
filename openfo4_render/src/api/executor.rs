mod allocator;

use std::hash::Hash;
use std::num::NonZeroU32;
use std::slice;
use std::sync::Arc;

use hashbrown::HashMap;
use openfo4_common::collections::scratch_buffer::ScratchBuffer;
use openfo4_common::collections::vec_map::VecMap;
use openfo4_tracing::trace_span;

use crate::api::executor::allocator::Allocation;
use crate::api::{BufferDescriptor, RawTextureView};
use crate::backend::allocator::{MemoryManager, UsageFlags};
use crate::backend::descriptors::AllocatedDescriptorSet;
use crate::backend::vulkan::{
    self, Buffer, CommandEncoder, Device, Texture, TextureArea, TextureView,
};
use crate::backend::{
    AccessFlags, BufferBarrier, BufferUsage, BufferView, CopyBuffer, DrawIndexedIndirectCommand,
    DrawIndirectCommand, MemoryTypeFlags, PipelineBarriers, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, TextureBarrier, TextureViewDescriptor,
    TimestampPipelineStage, WriteDescriptorBinding, WriteDescriptorResource,
    WriteDescriptorResources,
};
use crate::statistics::Statistics;

use super::commands::{Command, CommandRef, ComputeCommand, ComputePassCmd, RenderPassCmd};
use super::queries::{ManagedQueryPool, QueryObject, QueryPoolSet};
use super::resources::DescriptorSetResource;
use super::scheduler::{Barrier, Step};
use super::{
    BufferId, DeletionEvent, DescriptorSetId, DrawCall, DrawCmd, PipelineId, ResourceId, Resources,
    SamplerId, TextureId,
};

use allocator::MemoryAllocator;

#[derive(Debug)]
pub(super) struct Executor {
    allocator: MemoryAllocator,
    buffers: VecMap<BufferId, BufferAlloc>,
    textures: VecMap<TextureId, TextureData>,
    descriptor_sets: HashMap<DescriptorSetId, DescriptorSetInner>,
    /// Cache for [`TextureView`] objects. The lifetime of the [`TextureView`]s is erased. The
    /// views must not outlive the [`TextureData`] for the same [`TextureId`].
    ///
    /// FIXME: We currently only drop the texture view when when the complete texture is destroyed.
    /// This works mostly fine, because textures are often reused in the same way each frame,
    /// however for some uses like mip creation this causes the views to linger around for longer
    /// than necessary.
    /// We should clean them up when the usage does no longer happen.
    texture_views: TextureViewCache,
}

impl Executor {
    pub fn new(device: Device, manager: MemoryManager, statistics: Arc<Statistics>) -> Self {
        Self {
            allocator: MemoryAllocator::new(device.clone(), manager, statistics),
            buffers: VecMap::new(),
            textures: VecMap::new(),
            descriptor_sets: HashMap::new(),
            texture_views: TextureViewCache::default(),
        }
    }

    pub fn execute(
        &mut self,
        resources: &Resources,
        steps: Vec<Step<&CommandRef<'_>, ResourceId>>,
        encoder: &mut CommandEncoder<'_>,
        queries: &QueryPoolSet,
    ) -> TemporaryResources {
        let _span = trace_span!("Executor::execute").entered();

        let mut render_passes = 0;
        let mut compute_passes = 0;

        for step in &steps {
            match step {
                Step::Node(cmd) => match cmd.as_ref() {
                    Command::RenderPass(_) => {
                        render_passes += 1;
                    }
                    Command::ComputePass(_) => {
                        compute_passes += 1;
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        // Two timestamps for each pass: 1 before and 1 after
        // plus two timestamps to measure the entire command buffer time.
        let query_count = NonZeroU32::new(render_passes * 2 + compute_passes * 2 + 2).unwrap();
        let mut query_pool = queries.get(query_count);

        let mut tmp = TemporaryResources::default();

        let mut barriers = Vec::new();

        unsafe {
            let index = query_pool.next_index(QueryObject::BeginCommands);
            encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::None);
        }

        let staging = self.prepare_staging_pool(resources, encoder);

        for step in steps {
            // Batch all barrier steps together, then when the new step is not
            // a barrier emit all barriers at once.
            if !barriers.is_empty() && !step.is_barrier() {
                insert_barriers(self, &barriers, encoder);
                barriers.clear();
            }

            match step {
                Step::Node(cmd) => match cmd.as_ref() {
                    Command::CreateBuffer(cmd) => {
                        let buffer = self.allocator.create_buffer(&cmd.descriptor);

                        debug_assert!(!self.buffers.contains_key(cmd.id));
                        self.buffers.insert(cmd.id, buffer);
                    }
                    Command::CreateTexture(cmd) => {
                        let texture = match cmd.resource.lock().take() {
                            Some(texture) => TextureData::Physical(texture),
                            None => {
                                let texture = self
                                    .allocator
                                    .create_texture(&cmd.descriptor, UsageFlags::empty());

                                TextureData::Virtual(texture)
                            }
                        };

                        debug_assert!(!self.textures.contains_key(cmd.id));
                        self.textures.insert(cmd.id, texture);
                    }
                    Command::CreateDescriptorSet(id) => {
                        let set = build_descriptor_set(self, resources, *id);

                        self.descriptor_sets.insert(*id, set);
                    }
                    Command::DestoryBuffer(id) => {
                        debug_assert!(self.buffers.contains_key(*id));

                        if let Some(buffer) = self.buffers.remove(*id) {
                            unsafe {
                                self.allocator.dealloc(buffer.allocation);
                            }
                        }
                    }
                    Command::DestroyTexture(id) => {
                        debug_assert!(self.textures.contains_key(*id));

                        // Destroy all texture views BEFORE destroying the texture.
                        self.texture_views.remove(*id);

                        if let Some(texture) = self.textures.remove(*id) {
                            match texture {
                                TextureData::Physical(_) => {}
                                TextureData::Virtual(texture) => unsafe {
                                    self.allocator.dealloc(texture.allocation);
                                },
                            }
                        }
                    }
                    Command::DestroyDescriptorSet(id) => {
                        debug_assert!(self.descriptor_sets.contains_key(id));

                        self.descriptor_sets.remove(id);
                    }
                    Command::WriteBuffer(cmd) => {
                        let buffer = self.buffers.get_mut(cmd.buffer).unwrap();

                        let mut offset = cmd.offset;
                        staging.copy(cmd.staging_memory_offset, cmd.count, |src| unsafe {
                            encoder.copy_buffer_to_buffer(
                                src.buffer(),
                                src.offset(),
                                &buffer.buffer,
                                offset,
                                src.len(),
                            );

                            offset += src.len();
                        });

                        tmp.buffers.insert(cmd.buffer);
                    }
                    Command::CopyBufferToBuffer(cmd) => {
                        let src = self.buffers.get(cmd.src).unwrap();
                        let dst = self.buffers.get(cmd.dst).unwrap();

                        // SAFETY:
                        // We have inserted the proper barriers such that the source is
                        // `TRANSFER_READ` and the destination is `TRANSFER_WRITE`.
                        unsafe {
                            encoder.copy_buffer_to_buffer(
                                &src.buffer,
                                cmd.src_offset,
                                &dst.buffer,
                                cmd.dst_offset,
                                cmd.count.get(),
                            );
                        }

                        // Both buffers must be kept alive until the copy
                        // operation is complete.
                        tmp.buffers.insert(cmd.src);
                        tmp.buffers.insert(cmd.dst);
                    }
                    Command::CopyBufferToTexture(cmd) => {
                        let src = self.buffers.get(cmd.src).unwrap();
                        let dst = self.textures.get(cmd.dst).unwrap();

                        // SAFETY:
                        // We have inserted the proper barriers such that the source buffer
                        // is `TRANSFER_READ` and the texture mip is `TRANSFER_WRITE`.
                        unsafe {
                            encoder.copy_buffer_to_texture(
                                CopyBuffer {
                                    buffer: &src.buffer,
                                    offset: cmd.src_offset,
                                    layout: cmd.layout,
                                },
                                dst.texture(),
                                cmd.dst_mip_level,
                            );
                        }

                        // Both buffer and texture must be kept alive until the
                        // copy operation is complete.
                        tmp.buffers.insert(cmd.src);
                        tmp.textures.insert(cmd.dst);
                    }
                    Command::CopyTextureToTexture(cmd) => {
                        let src = self.textures.get(cmd.src).unwrap();
                        let dst = self.textures.get(cmd.dst).unwrap();

                        // Safety:
                        // We have inserted the propert barriers such that the source texture
                        // is only `TRANSFER_READ` and the destination texture is only
                        // `TRANSFER_WRITE`.
                        unsafe {
                            encoder.copy_texture_to_texture(
                                src.texture(),
                                cmd.src_mip_level,
                                dst.texture(),
                                cmd.dst_mip_level,
                            );
                        }

                        // Both textures need to be kept alive until the copy operation is complete.
                        tmp.textures.insert(cmd.src);
                        tmp.textures.insert(cmd.dst);
                    }
                    Command::ClearTexture(cmd) => {
                        let texture = self.textures.get(cmd.id).unwrap();

                        // SAFETY:
                        // - We have inserted barriers such that only TRANSFER_WRITE is set
                        //   when the operation is recorded.
                        // - We will insert another barrier before reading or writing the texture.
                        unsafe {
                            encoder.clear_texture(texture.texture(), cmd.mip_level, cmd.value);
                        }

                        tmp.textures.insert(cmd.id);
                    }
                    Command::TextureTransition(cmd) => {
                        // The texture is not explicitly used anywhere else, but
                        // it still must be kept alive for this frame since it
                        // is used in a barrier.
                        tmp.textures.insert(cmd.texture.id);
                    }
                    Command::Blit(cmd) => {
                        let src = self.textures.get(cmd.src.texture).unwrap();
                        let dst = self.textures.get(cmd.dst.texture).unwrap();

                        unsafe {
                            encoder.blit(
                                TextureArea {
                                    texture: src.texture(),
                                    mip_level: cmd.src.mip_level,
                                    min: cmd.src.min,
                                    max: cmd.src.max,
                                },
                                TextureArea {
                                    texture: dst.texture(),
                                    mip_level: cmd.dst.mip_level,
                                    min: cmd.dst.min,
                                    max: cmd.dst.max,
                                },
                                cmd.filter,
                            );
                        }

                        tmp.textures.insert(cmd.src.texture);
                        tmp.textures.insert(cmd.dst.texture);
                    }
                    Command::RenderPass(cmd) => {
                        run_render_pass(self, resources, &mut tmp, cmd, encoder, &mut query_pool);
                    }
                    Command::ComputePass(cmd) => {
                        run_compute_pass(self, resources, &mut tmp, cmd, encoder, &mut query_pool);
                    }
                },
                Step::Barrier(barrier) => {
                    barriers.push(barrier);
                }
            }
        }

        // Flush any remaining barriers.
        if !barriers.is_empty() {
            insert_barriers(self, &barriers, encoder);
        }

        unsafe {
            let index = query_pool.next_index(QueryObject::EndCommands);
            encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::All);
        }

        tmp.query_pool = Some(query_pool);
        tmp.staging = staging;

        tmp
    }

    fn prepare_staging_pool(
        &mut self,
        resources: &Resources,
        encoder: &mut CommandEncoder<'_>,
    ) -> StagingPool {
        // Note that we are taking the Vec and dropping it
        // at the end of the execution. We do this instead of
        // reusing the allocated memory because there may sometimes
        // be big chunks of data to be uploaded and that would result
        // in significant memory waste for most frames. Reallocating the
        // Vec is relatively cheap.
        // let staging_memory_pool = core::mem::take(&mut *resources.staging_memory.lock());
        let mut staging_memory_pool = resources.staging_memory.lock();

        let mut offset = 0;
        let mut buffers = Vec::new();

        while offset < staging_memory_pool.len() {
            let remaining = staging_memory_pool.len() - offset;
            let size = self.allocator.max_allocation_size().min(remaining);

            let mut buffer = self.allocator.create_buffer(&BufferDescriptor {
                size,
                usage: BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
                flags: UsageFlags::HOST_VISIBLE,
            });

            buffer.allocation.as_bytes()[0..size]
                .copy_from_slice(&staging_memory_pool[offset..offset + size]);

            // If the memory of the buffer is not HOST_COHERENT it needs to
            // be flushed, otherwise it may never become visible to the device.
            if !buffer
                .allocation
                .flags()
                .contains(MemoryTypeFlags::HOST_COHERENT)
            {
                unsafe {
                    buffer.allocation.flush();
                }
            }

            buffers.push((buffer, offset));
            offset += size;
        }

        debug_assert_eq!(offset, staging_memory_pool.len());

        let barriers = buffers
            .iter()
            .map(|(buffer, _)| BufferBarrier {
                buffer: &buffer.buffer,
                offset: 0,
                size: buffer.buffer.size(),
                src_access: AccessFlags::empty(),
                dst_access: AccessFlags::TRANSFER_READ,
            })
            .collect::<Vec<_>>();

        unsafe {
            encoder.insert_pipeline_barriers(&PipelineBarriers {
                buffer: &barriers,
                texture: &[],
            });
        }

        let new_cap = staging_memory_pool.len().next_power_of_two();
        staging_memory_pool.clear();
        staging_memory_pool.shrink_to(new_cap);

        StagingPool { buffers }
    }
}

fn run_render_pass(
    executor: &mut Executor,
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &RenderPassCmd,
    encoder: &mut CommandEncoder<'_>,
    query_pool: &mut ManagedQueryPool,
) {
    for attachment in &cmd.color_attachments {
        let view = attachment.target;
        executor.texture_views.get_or_insert_with(view, || {
            let texture = executor.textures.get(view.texture).unwrap();

            let view = texture.texture().create_view(&TextureViewDescriptor {
                base_mip_level: view.base_mip_level,
                mip_levels: view.mip_levels,
            });

            // SAFETY: We guarantee that we drop the view before the underlying
            // texture is dropped.
            unsafe { view.make_static() }
        });
    }

    if let Some(attachment) = &cmd.depth_stencil_attachment {
        let view = RawTextureView {
            texture: attachment.texture,
            base_mip_level: 0,
            mip_levels: 1,
        };
        executor.texture_views.get_or_insert_with(view, || {
            let texture = executor.textures.get(view.texture).unwrap();

            let view = texture.texture().create_view(&TextureViewDescriptor {
                base_mip_level: view.base_mip_level,
                mip_levels: view.mip_levels,
            });

            // SAFETY: We guarantee that we drop the view before the underlying
            // texture is dropped.
            unsafe { view.make_static() }
        });
    }

    let mut color_attachments = Vec::new();
    for attachment in &cmd.color_attachments {
        // Attachment must be kept alive until the render pass completes.
        tmp.textures.insert(attachment.target.texture);

        let view = executor.texture_views.get(&attachment.target).unwrap();

        color_attachments.push(RenderPassColorAttachment {
            view,
            load_op: attachment.load_op,
            store_op: attachment.store_op,
            access: AccessFlags::COLOR_ATTACHMENT_WRITE,
        });
    }

    let depth_attachment = cmd.depth_stencil_attachment.as_ref().map(|attachment| {
        // Attachment must be kept alive until the render pass completes.
        tmp.textures.insert(attachment.texture);

        // Depth stencil attachment can only be a single mip.
        let view = executor
            .texture_views
            .get(&RawTextureView {
                texture: attachment.texture,
                base_mip_level: 0,
                mip_levels: 1,
            })
            .unwrap();

        RenderPassDepthStencilAttachment {
            depth_load_op: attachment.load_op,
            depth_store_op: attachment.store_op,
            view,
            access: AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE,
        }
    });

    // Record a timestamp before running the render pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::BeginPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::None);
    }

    // SAFETY: We have inserted the necessary barrier so that all
    // color/depth attachments have the appropriate access flags set.
    let mut render_pass = unsafe {
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &color_attachments,
            depth_stencil_attachment: depth_attachment.as_ref(),
        })
    };

    let mut pipelines = Vec::new();

    for cmd in &cmd.cmds {
        match cmd {
            DrawCmd::SetPipeline(id) => {
                let pipeline = resources.pipelines.get(id.0).unwrap();

                // We store the pipeline.inner `Arc` value in a Vec,
                // preventing it to get dropped for the duration of the following
                // commands.
                pipelines.push(pipeline.inner.clone());
                render_pass.bind_pipeline(unsafe { &*Arc::as_ptr(&pipeline.inner) });

                // Pipeline must be kept alive until the render pass
                // completes.
                tmp.pipelines.insert(*id);
            }
            DrawCmd::SetDescriptorSet(index, id) => {
                let set = executor.descriptor_sets.get(id).unwrap();

                render_pass.bind_descriptor_set(*index, set.set.raw());

                // Descriptor set must be kept alive until the render pass
                // completes.
                tmp.descriptor_sets.insert(*id);
            }
            DrawCmd::SetIndexBuffer(id, format, range) => {
                let buffer = executor.buffers.get(*id).unwrap();
                let view = buffer.buffer.slice(*range);

                unsafe {
                    render_pass.bind_index_buffer(view, *format);
                }

                // Index buffer must be kept alive until the render pass
                // completes.
                tmp.buffers.insert(*id);
            }
            DrawCmd::SetPushConstants(data, stages, offset) => {
                render_pass.set_push_constants(*stages, *offset, data);
            }
            DrawCmd::Draw(DrawCall::Draw(call)) => {
                render_pass.draw(call.vertices.clone(), call.instances.clone());
            }
            DrawCmd::Draw(DrawCall::DrawIndexed(call)) => {
                render_pass.draw_indexed(
                    call.indices.clone(),
                    call.vertex_offset,
                    call.instances.clone(),
                );
            }
            DrawCmd::Draw(DrawCall::DrawIndirect(call)) => {
                let buffer = executor.buffers.get(call.buffer).unwrap();

                render_pass.draw_indirect(
                    &buffer.buffer,
                    call.offset,
                    call.count,
                    size_of::<DrawIndirectCommand>() as u32,
                );

                tmp.buffers.insert(call.buffer);
            }
            DrawCmd::Draw(DrawCall::DrawIndexedIndirect(call)) => {
                let buffer = executor.buffers.get(call.buffer).unwrap();

                render_pass.draw_indexed_indirect(
                    &buffer.buffer,
                    call.offset,
                    call.count,
                    size_of::<DrawIndexedIndirectCommand>() as u32,
                );

                tmp.buffers.insert(call.buffer);
            }
            DrawCmd::Draw(DrawCall::DrawMeshTasks(call)) => {
                render_pass.draw_mesh_tasks(call.x, call.y, call.z);
            }
        }
    }

    drop(render_pass);

    // Record a second timestamp after the render pass render pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::EndPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::Graphics);
    }
}

fn run_compute_pass(
    executor: &mut Executor,
    resources: &Resources,
    tmp: &mut TemporaryResources,
    cmd: &ComputePassCmd,
    encoder: &mut CommandEncoder<'_>,
    query_pool: &mut ManagedQueryPool,
) {
    // Record a timestamp before running the compute pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::BeginPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::None);
    }

    let mut compute_pass = encoder.begin_compute_pass();

    let mut pipelines = Vec::new();

    for cmd in &cmd.cmds {
        match cmd {
            ComputeCommand::SetPipeline(id) => {
                let pipeline = resources.pipelines.get(id.0).unwrap();

                // We store the pipeline.inner `Arc` value in a Vec,
                // preventing it to get dropped for the duration of the following
                // commands.
                pipelines.push(pipeline.inner.clone());
                compute_pass.bind_pipeline(unsafe { &*Arc::as_ptr(&pipeline.inner) });

                // Pipeline must be kept alive until the render pass
                // completes.
                tmp.pipelines.insert(*id);
            }
            ComputeCommand::SetDescriptorSet(index, id) => {
                let set = executor.descriptor_sets.get(id).unwrap();

                compute_pass.bind_descriptor_set(*index, set.set.raw());

                // Descriptor set must be kept alive until the render pass
                // completes.
                tmp.descriptor_sets.insert(*id);
            }
            ComputeCommand::SetPushConstants(data, stages, offset) => {
                compute_pass.set_push_constants(*stages, *offset, data);
            }
            ComputeCommand::Dispatch(x, y, z) => {
                compute_pass.dispatch(*x, *y, *z);
            }
        }
    }

    drop(compute_pass);

    // Record a second timestamp after the render compute render pass.
    unsafe {
        let index = query_pool.next_index(QueryObject::EndPass(cmd.name));
        encoder.write_timestamp_query(query_pool.pool(), index, TimestampPipelineStage::Compute);
    }
}

fn build_descriptor_set(
    executor: &mut Executor,
    resources: &Resources,
    id: DescriptorSetId,
) -> DescriptorSetInner {
    let _span = trace_span!("build_descriptor_set").entered();

    let descriptor_set = resources.descriptor_sets.get(id.0).unwrap();
    let layout = resources
        .descriptor_set_layouts
        .get(descriptor_set.layout.0)
        .unwrap();

    let buffer_views = ScratchBuffer::new(descriptor_set.num_buffers as usize);

    let num_texture_arrays = descriptor_set.num_sampled_texture_arrays as usize
        + descriptor_set.num_storage_texture_arrays as usize;
    let texture_view_refs = ScratchBuffer::new(num_texture_arrays);

    let mut bindings = Vec::new();

    // Ensure that each texture view has a raw backing `TextureView` object.
    // We cannot easily do this while building the descriptor set, because
    // all references from the hashmap must stay stable and must never create
    // a `TextureView` object twice for the same descriptor.
    for (_, resource) in descriptor_set.bindings.iter() {
        let (DescriptorSetResource::SampledTexture(views)
        | DescriptorSetResource::StorageTexture(views)) = resource
        else {
            continue;
        };

        for view in views {
            executor.texture_views.get_or_insert_with(*view, || {
                let texture = executor.textures.get(view.texture).unwrap();

                let view = texture.texture().create_view(&TextureViewDescriptor {
                    base_mip_level: view.base_mip_level,
                    mip_levels: view.mip_levels,
                });

                // SAFETY: We guarantee that we drop the view before the underlying
                // texture is dropped.
                unsafe { view.make_static() }
            });
        }
    }

    for (binding, resource) in descriptor_set.bindings.iter() {
        let resource = match resource {
            DescriptorSetResource::UniformBuffer(buffer, range) => {
                let buffer = executor.buffers.get(*buffer).unwrap();
                let view = buffer.buffer.slice(*range);
                let view = buffer_views.insert(view);

                WriteDescriptorResource::UniformBuffer(slice::from_ref(view))
            }
            DescriptorSetResource::StorageBuffer(buffer, range) => {
                let buffer = executor.buffers.get(*buffer).unwrap();
                let view = buffer.buffer.slice(*range);
                let view = buffer_views.insert(view);

                WriteDescriptorResource::StorageBuffer(slice::from_ref(view))
            }
            DescriptorSetResource::SampledTexture(views)
            | DescriptorSetResource::StorageTexture(views) => {
                let raw_view_refs = texture_view_refs.insert(ScratchBuffer::new(views.len()));

                for view in views {
                    // This cannot fail; We have created a `TextureView` object
                    // for each descriptor in the previous loop.
                    let view_ref = executor.texture_views.get(view).unwrap();
                    raw_view_refs.insert(view_ref);
                }

                match resource {
                    DescriptorSetResource::SampledTexture(_) => {
                        WriteDescriptorResource::Texture(raw_view_refs.as_slice())
                    }
                    DescriptorSetResource::StorageTexture(_) => {
                        WriteDescriptorResource::StorageTexture(raw_view_refs.as_slice())
                    }
                    _ => unreachable!(),
                }
            }
            DescriptorSetResource::Sampler(sampler) => {
                let sampler = resources.samplers.get(sampler.0).unwrap();
                let sampler = unsafe { &*(&sampler.inner as *const vulkan::Sampler) };

                WriteDescriptorResource::Sampler(slice::from_ref(&sampler))
            }
        };

        bindings.push(WriteDescriptorBinding { binding, resource });
    }

    let mut set = resources.descriptor_allocator.alloc(&layout.inner).unwrap();
    unsafe {
        set.raw_mut().update(&WriteDescriptorResources {
            bindings: &bindings,
        });
    }

    DescriptorSetInner { set }
}

fn insert_barriers(
    executor: &mut Executor,
    barriers: &[Barrier<ResourceId>],
    encoder: &mut CommandEncoder<'_>,
) {
    let mut buffer_barriers = Vec::new();
    let mut texture_barriers = Vec::<TextureBarrier<'_>>::new();

    let mut last_texture_barrier_id = None;

    for barrier in barriers {
        match barrier.resource {
            ResourceId::Buffer(id) => {
                let buffer = executor.buffers.get(id).unwrap();

                buffer_barriers.push(BufferBarrier {
                    buffer: &buffer.buffer,
                    offset: buffer.buffer.slice(..).offset(),
                    size: buffer.buffer.slice(..).len(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                });
            }
            ResourceId::Texture { id, mip_level } => {
                let texture = executor.textures.get(id).unwrap();

                // If possible barriers are emitted in batches for each texture
                // with the mip levels ascending, so if a chain of consecutive
                // mip levels is modified we can batch all those barriers into one.
                if let Some(last_barrier) = texture_barriers.last_mut() {
                    if last_texture_barrier_id == Some(id)
                        && last_barrier.src_access == barrier.src_access
                        && last_barrier.dst_access == barrier.dst_access
                        && last_barrier.base_mip_level + last_barrier.mip_levels == mip_level
                    {
                        last_barrier.mip_levels += 1;

                        last_texture_barrier_id = Some(id);
                        continue;
                    }
                }

                texture_barriers.push(TextureBarrier {
                    texture: texture.texture(),
                    src_access: barrier.src_access,
                    dst_access: barrier.dst_access,
                    base_mip_level: mip_level,
                    mip_levels: 1,
                });

                last_texture_barrier_id = Some(id);
            }
        }
    }

    debug_assert!(!buffer_barriers.is_empty() || !texture_barriers.is_empty());
    unsafe {
        encoder.insert_pipeline_barriers(&PipelineBarriers {
            buffer: &buffer_barriers,
            texture: &texture_barriers,
        });
    }
}

#[derive(Debug, Default)]
pub struct TemporaryResources {
    descriptor_sets: CountingSet<DescriptorSetId>,
    buffers: CountingSet<BufferId>,
    textures: CountingSet<TextureId>,
    pipelines: CountingSet<PipelineId>,
    samplers: CountingSet<SamplerId>,
    pub query_pool: Option<ManagedQueryPool>,
    // The staging buffers must be kept alive.
    staging: StagingPool,
}

impl TemporaryResources {
    pub(super) fn destroy(mut self, resources: &Resources, executor: &mut Executor) {
        for (id, count) in self.buffers.drain() {
            if resources.with_buffer_rc(id, |ref_count| ref_count.decrement_many_internal(count)) {
                resources.deletion_queue.push(DeletionEvent::Buffer(id));
            }
        }

        for (id, count) in self.textures.drain() {
            if resources.with_texture_rc(id, |ref_count| ref_count.decrement_many_internal(count)) {
                resources.deletion_queue.push(DeletionEvent::Texture(id));
            }
        }

        for (id, count) in self.descriptor_sets.drain() {
            let set = resources.descriptor_sets.get(id.0).unwrap();

            if set.ref_count.decrement_many_internal(count) {
                resources
                    .deletion_queue
                    .push(DeletionEvent::DescriptorSet(id));
            }
        }

        for (id, count) in self.samplers.drain() {
            let sampler = resources.samplers.get(id.0).unwrap();

            if sampler.ref_count.decrement_many_internal(count) {
                resources.deletion_queue.push(DeletionEvent::Sampler(id));
            }
        }

        for (id, count) in self.pipelines.drain() {
            let pipeline = resources.pipelines.get(id.0).unwrap();

            if pipeline.ref_count.decrement_many_internal(count) {
                resources.deletion_queue.push(DeletionEvent::Pipeline(id));
            }
        }

        if let Some(query_pool) = self.query_pool {
            unsafe {
                query_pool.release();
            }
        }

        for (buffer, _) in self.staging.buffers.drain(..) {
            unsafe {
                executor.allocator.dealloc(buffer.allocation);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct CountingSet<T>(HashMap<T, usize>);

impl<T> CountingSet<T> {
    fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<T> CountingSet<T>
where
    T: Hash + Eq,
{
    fn insert(&mut self, key: T) {
        *self.0.entry(key).or_default() += 1;
    }

    fn drain(&mut self) -> impl Iterator<Item = (T, usize)> + '_ {
        self.0.drain()
    }
}

impl<T> Default for CountingSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Extend<T> for CountingSet<T>
where
    T: Hash + Eq,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in iter {
            self.insert(elem);
        }
    }
}

#[derive(Debug)]
struct DescriptorSetInner {
    set: AllocatedDescriptorSet,
}

#[derive(Debug)]
pub enum TextureData {
    Physical(Texture),
    Virtual(TextureAlloc),
}

impl TextureData {
    pub fn texture(&self) -> &Texture {
        match &self {
            Self::Physical(tex) => tex,
            Self::Virtual(tex) => &tex.texture,
        }
    }
}

#[derive(Debug)]
struct BufferAlloc {
    buffer: Buffer,
    allocation: Allocation,
}

#[derive(Debug)]
pub struct TextureAlloc {
    texture: Texture,
    allocation: Allocation,
}

#[derive(Debug, Default)]
struct TextureViewCache {
    textures: VecMap<TextureId, Vec<RawTextureView>>,
    views: HashMap<RawTextureView, TextureView<'static>>,
}

impl TextureViewCache {
    pub fn remove(&mut self, id: TextureId) {
        if let Some(views) = self.textures.remove(id) {
            for view in views {
                self.views.remove(&view);
            }
        }
    }

    pub fn get_or_insert_with<F>(&mut self, view: RawTextureView, f: F) -> &TextureView<'static>
    where
        F: FnOnce() -> TextureView<'static>,
    {
        self.views.entry(view).or_insert_with(|| {
            self.textures.entry(view.texture).or_default().push(view);
            f()
        })
    }

    pub fn get(&self, view: &RawTextureView) -> Option<&TextureView<'static>> {
        self.views.get(view)
    }
}

/// A pool of staging buffers.
#[derive(Debug, Default)]
struct StagingPool {
    buffers: Vec<(BufferAlloc, usize)>,
}

impl StagingPool {
    // TODO: Make this return an iterator.
    fn copy<F>(&self, mut offset: usize, mut count: usize, mut f: F)
    where
        F: FnMut(BufferView<'_>),
    {
        debug_assert_ne!(count, 0);
        debug_assert!(!self.buffers.is_empty());

        let mut index = self
            .buffers
            .iter()
            .position(|(_, off)| *off > offset)
            .unwrap_or(self.buffers.len())
            .saturating_sub(1);

        // It is very possible for a single write destination to span multiple
        // staging buffers. In the "staging allocation" continues in the next
        // buffer.
        while count != 0 {
            let (buffer, start) = &self.buffers[index];
            let start = offset - start;
            let len = usize::min(buffer.buffer.size() - start, count);

            let slice = buffer.buffer.slice(start..start + len);
            f(slice);

            count -= len;
            offset += len;
            index += 1;
        }
    }
}
