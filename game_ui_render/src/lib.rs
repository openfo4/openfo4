use std::sync::Mutex;

use crossbeam_channel::Sender;
use game_common::collections::arena::{Arena, Key};
use game_render::graph::{NodeLabel, SlotFlags, SlotKind, SlotLabel};
use game_render::texture::image::MipImage;
use game_render::{FINAL_RENDER_PASS, Renderer};
use glam::{Vec2, Vec4};

use crate::pass::UiPass;

mod pass;

#[derive(Debug)]
pub struct UiRenderer {
    events: Sender<Event>,
    meshes: Mutex<Arena<()>>,
    textures: Mutex<Arena<()>>,
}

impl UiRenderer {
    pub fn new(renderer: &mut Renderer) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();

        renderer.with_command_queue_and_graph(|graph, queue| {
            let ui_pass = UiPass::new(queue, rx);
            const UI_PASS: NodeLabel = NodeLabel::new("UI_RENDER_PASS");

            graph.add_node(UI_PASS, ui_pass);
            graph.add_node_dependency(UI_PASS, FINAL_RENDER_PASS);
            graph.add_slot_dependency(
                UI_PASS,
                SlotLabel::SURFACE,
                SlotKind::Texture,
                SlotFlags::READ | SlotFlags::WRITE,
            );
        });

        Self {
            events: tx,
            meshes: Mutex::default(),
            textures: Mutex::default(),
        }
    }

    pub fn create_mesh(&self, mesh: Mesh) -> MeshId {
        let mut meshes = self.meshes.lock().unwrap();
        let key = MeshId(meshes.insert(()));
        drop(meshes);

        self.events.send(Event::CreateMesh(mesh, key)).unwrap();
        key
    }

    pub fn create_texture<T>(&self, texture: T) -> TextureId
    where
        T: Into<MipImage>,
    {
        let texture = texture.into();

        let mut textures = self.textures.lock().unwrap();
        let key = TextureId(textures.insert(()));
        drop(textures);

        self.events
            .send(Event::CreateTexture(texture, key))
            .unwrap();
        key
    }

    pub fn destroy_mesh(&self, mesh: MeshId) {
        let mut meshes = self.meshes.lock().unwrap();
        if meshes.remove(mesh.0).is_none() {
            return;
        }

        drop(meshes);
        self.events.send(Event::DestroyMesh(mesh)).unwrap();
    }

    pub fn destroy_texture(&self, texture: TextureId) {
        let mut textures = self.textures.lock().unwrap();
        if textures.remove(texture.0).is_none() {
            return;
        }

        drop(textures);
        self.events.send(Event::DestroyTexture(texture)).unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture: Option<TextureId>,
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vec2,
    pub uv: Vec2,
    pub color: Vec4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(Key);

#[derive(Clone, Debug)]
pub enum Event {
    CreateMesh(Mesh, MeshId),
    CreateTexture(MipImage, TextureId),
    DestroyTexture(TextureId),
    DestroyMesh(MeshId),
}
