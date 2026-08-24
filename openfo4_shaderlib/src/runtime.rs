use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use game_render::shader::{Error, Shader};

use crate::reload::ReloadableShaderSource;

#[derive(Debug)]
pub struct ShaderSource {
    path: &'static str,
    source: OnceLock<ReloadableShaderSource>,
    shader: Mutex<Option<Arc<Shader>>>,
}

impl ShaderSource {
    pub const fn from_path(path: &'static str) -> Self {
        Self {
            path,
            source: OnceLock::new(),
            shader: Mutex::new(None),
        }
    }

    pub fn has_changed(&self) -> bool {
        self.source
            .get_or_init(|| ReloadableShaderSource::new(self.path()))
            .has_changed()
    }

    pub fn compile(&self) -> Result<Arc<Shader>, Error> {
        let source = self
            .source
            .get_or_init(|| ReloadableShaderSource::new(self.path()));

        let mut shader_slot = self.shader.lock().unwrap();

        if let Some(shader) = &*shader_slot
            && !self.has_changed()
        {
            return Ok(Arc::clone(shader));
        }

        source.reload_sources()?;
        let shader = Arc::new(source.compile()?);
        *shader_slot = Some(Arc::clone(&shader));
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
