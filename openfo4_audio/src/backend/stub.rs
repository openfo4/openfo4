use crate::renderer::Renderer;

use super::Backend;

#[derive(Copy, Clone, Debug, Default)]
pub struct StubBackend;

impl Backend for StubBackend {
    fn create_output_stream(&mut self, _renderer: Renderer) {}
}
