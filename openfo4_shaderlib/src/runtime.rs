use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use openfo4_render::shader::Shader;

use crate::reload::{BoxError, ReloadableShaderSource};

#[derive(Debug)]
pub struct ShaderSource {
    path: &'static str,
    source: OnceLock<ReloadableShaderSource>,
    shader: Mutex<Option<ShaderRev>>,
    rev: AtomicU64,
}

impl ShaderSource {
    pub const fn from_path(path: &'static str) -> Self {
        Self {
            path,
            source: OnceLock::new(),
            shader: Mutex::new(None),
            rev: AtomicU64::new(0),
        }
    }

    pub fn has_changed(&self) -> bool {
        let rev = self
            .source
            .get_or_init(|| ReloadableShaderSource::new(self.path()))
            .rev();

        self.rev.swap(rev, Ordering::SeqCst) != rev
    }

    pub fn compile(&self) -> Result<Arc<Shader>, BoxError> {
        let source = self
            .source
            .get_or_init(|| ReloadableShaderSource::new(self.path()));

        // We should take the revision number before we compile the shader.
        // This might mean that we compile an already outdated shader, but
        // we can correctly update it on the next compile call.
        // If we took the revision after compilation we might not see a change
        // and forever return an outdated shader.
        let rev = source.rev();

        let mut shader_slot = self.shader.lock().unwrap();

        // Don't recompile the shader when the revision number of unchanged,
        // which indicates that the shader source has not changed.
        if let Some(shader) = &*shader_slot
            && shader.rev == rev
        {
            return Ok(Arc::clone(&shader.shader));
        }

        source.reload_sources()?;
        let shader = Arc::new(source.compile()?);
        *shader_slot = Some(ShaderRev {
            shader: Arc::clone(&shader),
            rev,
        });
        Ok(shader)
    }

    fn path(&self) -> PathBuf {
        let path = self.path.strip_prefix('/').unwrap_or(self.path);
        WORKSPACE.join(path)
    }
}

static WORKSPACE: LazyLock<PathBuf> = LazyLock::new(|| {
    let output = Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
});

#[derive(Debug)]
struct ShaderRev {
    shader: Arc<Shader>,
    rev: u64,
}
