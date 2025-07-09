use std::io::Read;

use image::{ImageBuffer, Rgba};

use crate::loader::FileLoader;

pub async fn load_image_buffer(
    loader: &FileLoader,
    mut path: String,
) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    if !path.starts_with("textures") {
        path = format!("textures/{}", path);
    }

    let mut reader = match loader.open(&path) {
        Ok(reader) => reader,
        Err(err) => {
            tracing::error!("failed to load texture: {}", err);
            return None;
        }
    };

    let mut buf = Vec::new();
    if let Err(err) = reader.read_to_end(&mut buf) {
        tracing::error!("failed to read from buffer: {}", err);
        return None;
    }

    Some(image::load_from_memory(&buf).unwrap().to_rgba8())
}
