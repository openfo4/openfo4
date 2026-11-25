use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::UVec2;
use openfo4_render::api::{
    CommandQueue, DepthStencilAttachment, RenderPassColorAttachment, RenderPassDescriptor, Texture,
    TextureViewDescriptor,
};
use openfo4_render::backend::{ClearValue, LoadOp, StoreOp, TextureDescriptor, TextureUsage};
use openfo4_render::graph::{Node, RenderContext, SlotLabel};
use openfo4_tracing::trace_span;
use parking_lot::Mutex;

use crate::camera::CameraUniform;
use crate::entities::{CameraId, SceneId};
use crate::passes::main_pass::MainPass;
use crate::passes::sky::SkyPass;
use crate::passes::update::RawSceneData;
use crate::passes::{DEPTH_FORMAT, HDR_FORMAT, HDR_TEXTURE, OPAQUE_DEPTH, State};

pub struct OpaqueVertexForwardPass {
    state: Arc<Mutex<State>>,
    sky: SkyPass,
    main: Mutex<MainPass>,
}

impl OpaqueVertexForwardPass {
    pub fn new(queue: &CommandQueue<'_>, state: Arc<Mutex<State>>) -> Self {
        Self {
            sky: { SkyPass::new(state.lock().sky.clone(), queue) },
            state,
            main: Mutex::new(MainPass::new(queue)),
        }
    }

    fn render_scene_with_camera(
        &self,
        ctx: &mut RenderContext<'_, '_>,
        state: &mut State,
        scene: SceneId,
        camera: CameraId,
        size: UVec2,
    ) {
        let _span = trace_span!("ForwardPass::render_scene_with_camera").entered();

        let output = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE
                | TextureUsage::TRANSFER_DST,
        });

        let depth = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
        });

        let camera = state
            .scenes
            .get(scene)
            .unwrap()
            .cameras
            .get(&camera)
            .unwrap()
            .clone();

        self.sky.run(ctx, &camera, &output);

        let mut main = self.main.lock();
        main.render(ctx.queue, size, state, camera, scene, &output, &depth);

        ctx.write(HDR_TEXTURE, output).unwrap();
        ctx.write(OPAQUE_DEPTH, depth).unwrap();
    }

    fn clear_pass(&self, ctx: &mut RenderContext<'_, '_>, size: UVec2) {
        let texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: HDR_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING
                | TextureUsage::RENDER_ATTACHMENT
                | TextureUsage::STORAGE,
        });

        let depth_texture = ctx.queue.create_texture(&TextureDescriptor {
            size,
            mip_levels: 1,
            format: DEPTH_FORMAT,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::RENDER_ATTACHMENT,
        });

        ctx.queue.run_render_pass(&RenderPassDescriptor {
            name: "Forward",
            color_attachments: &[RenderPassColorAttachment {
                target: &texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Clear(ClearValue::BLACK),
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: Some(&DepthStencilAttachment {
                texture: &depth_texture,
                load_op: LoadOp::Clear(0.0),
                store_op: StoreOp::Store,
            }),
        });

        ctx.write(HDR_TEXTURE, texture).unwrap();
        ctx.write(OPAQUE_DEPTH, depth_texture).unwrap();
    }
}

impl Node for OpaqueVertexForwardPass {
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>) {
        let _span = trace_span!("OpaqueVertexForwardPass::render").entered();

        let size = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().size();

        let mut state = self.state.lock();
        for (scene_id, scene) in state.scenes.iter() {
            for (camera_id, camera) in scene.cameras.iter() {
                if camera.target == ctx.render_target {
                    let camera_id = *camera_id;
                    self.render_scene_with_camera(ctx, &mut state, scene_id, camera_id, size);
                    return;
                }
            }
        }

        // If we don't have any camera to render we just
        // emit a black texture.
        self.clear_pass(ctx, size);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct ShadingPipelineConstants {
    scene: RawSceneData,
    camera: CameraUniform,
    num_directional_lights: u32,
    num_point_lights: u32,
    num_spot_lights: u32,
    _pad0: u32,
    viewport_size: UVec2,
}
