use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use egui::epaint::Primitive;
use egui::{Context, Event, ImageData, Key, PointerButton, Pos2, RawInput, Rect, ViewportId};
use game_input::keyboard::KeyCode;
use game_input::mouse::MouseButton;
use game_render::backend::TextureFormat;
use game_render::texture::Image;
use game_ui_render::{Mesh, MeshId, TextureId, UiRenderer, Vertex};
use game_window::events::WindowEvent;
use glam::{UVec2, Vec2, Vec4};

pub use egui;

#[derive(Debug)]
pub struct EguiContext {
    input: RawInput,
    surface_size: UVec2,
    cursor_position_in_points: Pos2,
    surface_scale: f32,
    context: Context,
    renderer: Arc<UiRenderer>,
    start_time: Instant,

    meshes: Vec<MeshId>,
    textures: HashMap<egui::TextureId, TextureState>,

    ui_scale: f32,
}

impl EguiContext {
    pub fn new(renderer: Arc<UiRenderer>) -> Self {
        let mut input = RawInput::default();

        input
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(1.0);
        input.viewport_id = ViewportId::ROOT;

        Self {
            input,
            surface_size: UVec2::ZERO,
            surface_scale: 1.0,
            cursor_position_in_points: Pos2::ZERO,
            context: Context::default(),
            renderer,
            meshes: Vec::new(),
            textures: HashMap::new(),
            ui_scale: 1.0,
            start_time: Instant::now(),
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::WindowResized(event) => {
                self.surface_size = event.size();
            }
            WindowEvent::WindowScaleFactorChanged(event) => {
                self.surface_scale = event.scale_factor as f32;
            }
            WindowEvent::CursorMoved(event) => {
                self.cursor_position_in_points =
                    Pos2::new(event.position.x, event.position.y) / self.ui_scale;
                self.input
                    .events
                    .push(Event::PointerMoved(self.cursor_position_in_points));
            }
            WindowEvent::MouseButtonInput(event) => {
                let Some(button) = map_cursor_button(event.button) else {
                    return;
                };

                self.input.events.push(Event::PointerButton {
                    pos: self.cursor_position_in_points,
                    button,
                    pressed: event.state.is_pressed(),
                    modifiers: self.input.modifiers,
                });
            }
            WindowEvent::CursorEntered(_) => {
                self.input.focused = true;
            }
            WindowEvent::CursorLeft(_) => {
                self.input.events.push(Event::PointerGone);
                self.input.focused = false;
            }
            WindowEvent::KeyboardInput(event) => {
                match event.key_code {
                    Some(KeyCode::LControl) | Some(KeyCode::RControl) => {
                        self.input.modifiers.ctrl = event.state.is_pressed();
                        self.input.modifiers.command = event.state.is_pressed();
                        return;
                    }
                    Some(KeyCode::LAlt) | Some(KeyCode::RAlt) => {
                        self.input.modifiers.alt = event.state.is_pressed();
                        return;
                    }
                    Some(KeyCode::LShift) | Some(KeyCode::RShift) => {
                        self.input.modifiers.shift = event.state.is_pressed();
                        return;
                    }
                    _ => {}
                }

                if let Some(key) = event.key_code
                    && let Some(key) = map_key(key)
                {
                    self.input.events.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed: event.state.is_pressed(),
                        repeat: false,
                        modifiers: self.input.modifiers,
                    });
                }

                if let Some(text) = &event.text
                    && !text.is_empty()
                    && text.chars().all(|c| !c.is_control())
                    && event.state.is_pressed()
                {
                    // Ignore commands
                    let is_cmd = self.input.modifiers.ctrl || self.input.modifiers.command;
                    if event.state.is_pressed() && !is_cmd {
                        self.input.events.push(Event::Text(text.to_string()));
                    }
                }
            }
            _ => {}
        }
    }

    /// Returns the inner egui [`Context`].
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Draws the UI using the given closure.
    ///
    /// This function processes all inputs, draws the UI using the given closure and submits it to
    /// the renderer.
    pub fn draw<F>(&mut self, f: F)
    where
        F: FnMut(&Context),
    {
        self.ui_scale = self.surface_scale * self.context.zoom_factor();

        let mut input = self.input.take();
        input
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.surface_scale);
        input.viewport_id = ViewportId::ROOT;
        input.screen_rect = Some(Rect {
            min: Pos2::ZERO,
            max: Pos2::new(
                self.surface_size.x as f32 / self.ui_scale,
                self.surface_size.y as f32 / self.ui_scale,
            ),
        });
        input.time = Some(self.start_time.elapsed().as_secs_f64());

        let output = self.context.run(input, f);

        for mesh in self.meshes.drain(..) {
            self.renderer.destroy_mesh(mesh);
        }

        for id in output.textures_delta.free {
            let Some(state) = self.textures.remove(&id) else {
                continue;
            };

            self.renderer.destroy_texture(state.id);
        }

        for (id, texture) in output.textures_delta.set {
            let data = match &texture.image {
                ImageData::Color(data) => data
                    .pixels
                    .iter()
                    .map(|pixel| pixel.to_array())
                    .flatten()
                    .collect::<Vec<_>>(),
            };

            let (data, size) = match self.textures.get_mut(&id) {
                Some(state) => {
                    self.renderer.destroy_texture(state.id);

                    if let Some([start_x, start_y]) = texture.pos {
                        for x in 0..texture.image.width() {
                            for y in 0..texture.image.height() {
                                let src_offset = (y * texture.image.width() + x) * 4;
                                let dst_offset =
                                    ((start_y + y) * state.size.x as usize + start_x + x) * 4;

                                state.data[dst_offset..dst_offset + 4]
                                    .copy_from_slice(&data[src_offset..src_offset + 4]);
                            }
                        }

                        (state.data.clone(), state.size)
                    } else {
                        (
                            data,
                            UVec2::new(texture.image.width() as u32, texture.image.height() as u32),
                        )
                    }
                }
                None => {
                    debug_assert!(texture.is_whole());
                    (
                        data,
                        UVec2::new(texture.image.width() as u32, texture.image.height() as u32),
                    )
                }
            };

            let tex = self.renderer.create_texture(Image::new(
                size,
                TextureFormat::Rgba8UnormSrgb,
                data.clone(),
            ));

            self.textures.insert(
                id,
                TextureState {
                    id: tex,
                    data,
                    size,
                },
            );
        }

        let primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);

        for primitive in primitives {
            match primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let texture = self.textures.get(&mesh.texture_id).unwrap();

                    let mesh = Mesh {
                        vertices: mesh
                            .vertices
                            .iter()
                            .map(|v| Vertex {
                                position: Vec2::new(v.pos.x, v.pos.y) * output.pixels_per_point,
                                uv: Vec2::new(v.uv.x, v.uv.y),
                                color: Vec4::from_array(
                                    v.color.to_array().map(|v| v as f32 / 255.0),
                                ),
                            })
                            .collect(),
                        indices: mesh.indices,
                        texture: Some(texture.id),
                    };

                    let id = self.renderer.create_mesh(mesh);
                    self.meshes.push(id);
                }
                Primitive::Callback(_) => {
                    unreachable!("Primitive::Callback is not supported");
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct TextureState {
    size: UVec2,
    data: Vec<u8>,
    id: TextureId,
}

fn map_cursor_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

fn map_key(key: KeyCode) -> Option<Key> {
    match key {
        KeyCode::Key1 | KeyCode::Numpad1 => Some(Key::Num1),
        KeyCode::Key2 | KeyCode::Numpad2 => Some(Key::Num2),
        KeyCode::Key3 | KeyCode::Numpad3 => Some(Key::Num3),
        KeyCode::Key4 | KeyCode::Numpad4 => Some(Key::Num4),
        KeyCode::Key5 | KeyCode::Numpad5 => Some(Key::Num5),
        KeyCode::Key6 | KeyCode::Numpad6 => Some(Key::Num6),
        KeyCode::Key7 | KeyCode::Numpad7 => Some(Key::Num7),
        KeyCode::Key8 | KeyCode::Numpad8 => Some(Key::Num8),
        KeyCode::Key9 | KeyCode::Numpad9 => Some(Key::Num9),
        KeyCode::Key0 | KeyCode::Numpad0 => Some(Key::Num0),
        KeyCode::A => Some(Key::A),
        KeyCode::B => Some(Key::B),
        KeyCode::C => Some(Key::C),
        KeyCode::D => Some(Key::D),
        KeyCode::E => Some(Key::E),
        KeyCode::F => Some(Key::F),
        KeyCode::G => Some(Key::G),
        KeyCode::H => Some(Key::H),
        KeyCode::I => Some(Key::I),
        KeyCode::J => Some(Key::J),
        KeyCode::K => Some(Key::K),
        KeyCode::L => Some(Key::L),
        KeyCode::M => Some(Key::M),
        KeyCode::N => Some(Key::N),
        KeyCode::O => Some(Key::O),
        KeyCode::P => Some(Key::P),
        KeyCode::Q => Some(Key::Q),
        KeyCode::R => Some(Key::R),
        KeyCode::S => Some(Key::S),
        KeyCode::T => Some(Key::T),
        KeyCode::U => Some(Key::U),
        KeyCode::V => Some(Key::V),
        KeyCode::W => Some(Key::W),
        KeyCode::X => Some(Key::X),
        KeyCode::Y => Some(Key::Y),
        KeyCode::Z => Some(Key::Z),
        KeyCode::Escape => Some(Key::Escape),
        KeyCode::F1 => Some(Key::F1),
        KeyCode::F2 => Some(Key::F2),
        KeyCode::F3 => Some(Key::F3),
        KeyCode::F4 => Some(Key::F4),
        KeyCode::F5 => Some(Key::F5),
        KeyCode::F6 => Some(Key::F6),
        KeyCode::F7 => Some(Key::F7),
        KeyCode::F8 => Some(Key::F8),
        KeyCode::F9 => Some(Key::F9),
        KeyCode::F10 => Some(Key::F10),
        KeyCode::F11 => Some(Key::F11),
        KeyCode::F12 => Some(Key::F12),
        KeyCode::F13 => Some(Key::F13),
        KeyCode::F14 => Some(Key::F14),
        KeyCode::F15 => Some(Key::F15),
        KeyCode::F16 => Some(Key::F16),
        KeyCode::F17 => Some(Key::F17),
        KeyCode::F18 => Some(Key::F18),
        KeyCode::F19 => Some(Key::F19),
        KeyCode::F20 => Some(Key::F20),
        KeyCode::F21 => Some(Key::F21),
        KeyCode::F22 => Some(Key::F22),
        KeyCode::F23 => Some(Key::F23),
        KeyCode::F24 => Some(Key::F24),
        KeyCode::Insert => Some(Key::Insert),
        KeyCode::Home => Some(Key::Home),
        KeyCode::Delete => Some(Key::Delete),
        KeyCode::End => Some(Key::End),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::Left => Some(Key::ArrowLeft),
        KeyCode::Up => Some(Key::ArrowUp),
        KeyCode::Right => Some(Key::ArrowRight),
        KeyCode::Down => Some(Key::PageDown),
        KeyCode::Return => Some(Key::Enter),
        KeyCode::Space => Some(Key::Space),
        KeyCode::Backslash => Some(Key::Backslash),
        KeyCode::Colon => Some(Key::Colon),
        KeyCode::Comma => Some(Key::Comma),
        KeyCode::Equals => Some(Key::Equals),
        KeyCode::Period => Some(Key::Period),
        KeyCode::Slash => Some(Key::Slash),
        KeyCode::Plus => Some(Key::Plus),
        KeyCode::Minus => Some(Key::Minus),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Backspace => Some(Key::Backspace),
        _ => None,
    }
}
