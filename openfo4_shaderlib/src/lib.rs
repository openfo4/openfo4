pub mod pipeline;

#[cfg(not(openfo4_embed_shaders))]
mod reload;
#[cfg(not(openfo4_embed_shaders))]
mod runtime;

#[cfg(openfo4_embed_shaders)]
mod embed;

use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use game_render::shader::{Error, Shader};

/// Includes a [`ShaderSource`] file.
///
/// # Panics
///
/// Panics at compile-time when the given shader file does not exist or contains an invalid shader.
///
/// # Examples
///
/// ```ignore
/// # use game_shaderlib::{include_shader, ShaderSource};
/// #
/// static SHADER: ShaderSource = include_shader!("shaders/triangle.slang");
/// ```
#[macro_export]
macro_rules! include_shader {
    ($path:expr) => {
        $crate::include_shader_impl!($path)
    };
}

/// **DO NOT USE; INTERNAL IMPLEMENTATION DETAIL**
#[doc(hidden)]
#[cfg(not(openfo4_embed_shaders))]
#[macro_export]
macro_rules! include_shader_impl {
    ($path:expr) => {
        $crate::ShaderSource::from_path($path)
    };
}

/// **DO NOT USE; INTERNAL IMPLEMENTATION DETAIL**
#[doc(hidden)]
#[cfg(openfo4_embed_shaders)]
pub use macros::include_shader as include_shader_impl;

/// A source for a [`Shader`].
#[derive(Debug)]
pub struct ShaderSource {
    #[cfg(not(openfo4_embed_shaders))]
    source: runtime::ShaderSource,
    #[cfg(openfo4_embed_shaders)]
    source: embed::ShaderSource,
}

impl ShaderSource {
    /// Returns `true` when the shader source has changed and should be reloaded.
    #[inline]
    pub fn has_changed(&self) -> bool {
        self.source.has_changed()
    }

    /// Compiles the current version of the [`Shader`].
    #[inline]
    pub fn compile(&self) -> Result<ShaderRef<'_>, Error> {
        cfg_select! {
            not(openfo4_embed_shaders) => {
                let shader = self.source.compile()?;
                Ok(ShaderRef { shader, _marker: PhantomData })
            }
            openfo4_embed_shaders => {
                let shader = self.source.compile();
                Ok(ShaderRef { shader, _marker: PhantomData })
            }
        }
    }

    /// **DO NOT USE; INTERNAL IMPLEMENTATION DETAIL**
    ///
    /// Creates a new `ShaderSource` at runtime from the given `path`.
    #[doc(hidden)]
    #[cfg(not(openfo4_embed_shaders))]
    pub const fn from_path(path: &'static str) -> Self {
        Self {
            source: runtime::ShaderSource::from_path(path),
        }
    }

    /// **DO NOT USE; INTERNAL IMPLEMENTATION DETAIL**
    ///
    /// Creates a new `ShaderSoruce` at compile time from the given SPIR-V bytecode.
    #[doc(hidden)]
    #[cfg(openfo4_embed_shaders)]
    pub const fn from_spirv(bytes: &'static [u8]) -> Self {
        Self {
            source: embed::ShaderSource::from_spirv(bytes),
        }
    }
}

/// A reference to a compiled [`Shader`], returned by [`ShaderSource::compile`].
#[derive(Debug)]
pub struct ShaderRef<'a> {
    #[cfg(not(openfo4_embed_shaders))]
    shader: Arc<Shader>,
    #[cfg(openfo4_embed_shaders)]
    shader: &'a Shader,
    _marker: PhantomData<&'a Shader>,
}

impl Deref for ShaderRef<'_> {
    type Target = Shader;

    fn deref(&self) -> &Self::Target {
        &self.shader
    }
}
