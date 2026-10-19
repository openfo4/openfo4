use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use openfo4_core_pipeline::camera::{Camera, CameraUniform};
use openfo4_render::api::{
    BufferInitDescriptor, FragmentStage, PipelineStage, RenderPassColorAttachment,
    RenderPassDescriptor, Texture, TextureViewDescriptor, VertexStage,
};
use openfo4_render::backend::allocator::UsageFlags;
use openfo4_render::backend::{
    BufferUsage, ColorTargetState, DescriptorType, Face, FrontFace, LoadOp, PrimitiveTopology,
    ShaderStages, StoreOp, TextureFormat,
};
use openfo4_render::graph::{Node, RenderContext, SlotLabel};
use openfo4_shaderlib::include_shader;
use openfo4_shaderlib::pipeline::{Pipeline, PipelineBinding, PipelineConfig, ShaderConfig};
use openfo4_tracing::trace_span;
use parking_lot::{Mutex, RwLock};

use super::DrawCommand;

static SHADER: openfo4_shaderlib::ShaderSource = include_shader!("/game_gizmos/shaders/line.wgsl");

pub struct GizmoPass {
    camera: Arc<Mutex<Option<Camera>>>,
    elements: Arc<RwLock<Vec<DrawCommand>>>,
    vertex_buffer: Mutex<Vec<Vertex>>,
    pipelines: Mutex<HashMap<TextureFormat, Pipeline<ColorTargetState>>>,
}

impl GizmoPass {
    pub(crate) fn new(
        elements: Arc<RwLock<Vec<DrawCommand>>>,
        camera: Arc<Mutex<Option<Camera>>>,
    ) -> Self {
        Self {
            elements,
            camera,
            vertex_buffer: Mutex::new(Vec::new()),
            pipelines: Mutex::default(),
        }
    }

    fn update_buffers(&self) {
        let cmds = self.elements.read();

        let mut vertex_buffer = self.vertex_buffer.lock();
        vertex_buffer.clear();

        for cmd in &*cmds {
            vertex_buffer.push(Vertex {
                position: cmd.start.to_array(),
                color: cmd.color.as_rgba(),
                _pad0: 0,
            });
            vertex_buffer.push(Vertex {
                position: cmd.end.to_array(),
                color: cmd.color.as_rgba(),
                _pad0: 0,
            });
        }
    }
}

impl Node for GizmoPass {
    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let _span = trace_span!("GizmoPass::render").entered();

        let Some(camera) = &*self.camera.lock() else {
            return;
        };

        self.update_buffers();
        let vertex_buffer = self.vertex_buffer.lock();

        // Don't start a render pass with 0 vertices, this will cause problems
        // because the vertex SSBO must contain at least one element.
        if vertex_buffer.len() == 0 {
            return;
        }

        let camera_buffer = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[CameraUniform::new(
                camera.transform,
                camera.projection,
            )]),
            usage: BufferUsage::UNIFORM,
            flags: UsageFlags::empty(),
        });

        let vertices = ctx.queue.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vertex_buffer),
            usage: BufferUsage::STORAGE,
            flags: UsageFlags::empty(),
        });

        let surface_texture = ctx.read::<Texture>(SlotLabel::SURFACE).unwrap().clone();

        let mut pipelines = self.pipelines.lock();
        let pipeline = pipelines
            .entry(surface_texture.format())
            .or_insert_with(|| make_pipeline(surface_texture.format()));

        let mut pass = pipeline.prepare(ctx.queue);
        pass.set(
            "camera",
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            &camera_buffer,
        );
        pass.set(
            "lines",
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            &vertices,
        );

        let mut pass = pass.run_render_pass(&RenderPassDescriptor {
            name: "Gizmos",
            color_attachments: &[RenderPassColorAttachment {
                target: &surface_texture.create_view(&TextureViewDescriptor::default()),
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
            }],
            depth_stencil_attachment: None,
        });

        pass.draw(0..2, 0..(vertex_buffer.len() / 2) as u32);
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    _pad0: u32,
    color: [f32; 4],
}

fn make_pipeline(format: TextureFormat) -> Pipeline<ColorTargetState> {
    Pipeline::new_with_ctx(
        PipelineConfig {
            shaders: vec![ShaderConfig { source: &SHADER }],
            bindings: vec![
                PipelineBinding {
                    name: "camera",
                    stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    kind: DescriptorType::Uniform,
                },
                PipelineBinding {
                    name: "lines",
                    stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    kind: DescriptorType::Storage,
                },
            ],
            builder: Box::new(move |_queue, shaders, target| {
                vec![
                    PipelineStage::Vertex(VertexStage {
                        shader: &shaders[0],
                        entry: "vs_main",
                        topology: PrimitiveTopology::LineList,
                        spec_consts: Default::default(),
                    }),
                    PipelineStage::Fragment(FragmentStage {
                        shader: &shaders[0],
                        entry: "fs_main",
                        targets: std::slice::from_ref(target),
                        front_face: FrontFace::Ccw,
                        cull_mode: Some(Face::Back),
                        depth_stencil_state: None,
                        spec_consts: Default::default(),
                    }),
                ]
            }),
        },
        ColorTargetState {
            format,
            blend: None,
        },
    )
}
