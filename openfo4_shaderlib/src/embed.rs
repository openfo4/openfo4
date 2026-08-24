use std::sync::OnceLock;

use game_render::shader::Shader;

#[derive(Debug)]
pub(crate) struct ShaderSource {
    bytes: &'static [u8],
    shader: OnceLock<Shader>,
}

impl ShaderSource {
    #[inline]
    pub(crate) fn has_changed(&self) -> bool {
        false
    }

    #[inline]
    pub(crate) fn compile(&self) -> &Shader {
        self.shader
            .get_or_init(|| Shader::from_spirv(self.bytes).unwrap())
    }

    pub(crate) const fn from_spirv(bytes: &'static [u8]) -> Self {
        Self {
            shader: OnceLock::new(),
            bytes,
        }
    }
}
