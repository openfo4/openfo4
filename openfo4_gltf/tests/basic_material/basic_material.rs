use glam::UVec2;
use openfo4_common::components::Color;
use openfo4_gltf::AlphaMode;
use openfo4_gltf::GltfData;
use openfo4_render::backend::TextureFormat;
use openfo4_render::texture::Image;

const ALPHA_MODE: AlphaMode = AlphaMode::Opaque;

const BASE_COLOR: Color = Color::from_rgba([1.0, 1.0, 1.0, 1.0]);
const BASE_COLOR_TEXTURE: &[u8] = include_bytes!("test_texture.png");

const METALLIC: f32 = 0.0;
const ROUGHNESS: f32 = 0.5;

#[test]
fn basic_material_gltf() {
    let data = GltfData::from_file("./tests/basic_material/basic_material.gltf").unwrap();
    validate_output(&data);
}

#[test]
fn basic_material_glb() {
    let data = GltfData::from_file("./tests/basic_material/basic_material.glb").unwrap();
    validate_output(&data);
}

#[test]
fn basic_material_gltf_embedded() {
    let data = GltfData::from_file("./tests/basic_material/basic_material_embedded.gltf").unwrap();
    validate_output(&data);
}

fn validate_output(data: &GltfData) {
    let nodes = &data.scenes[0].nodes;
    let node = nodes
        .values()
        .filter(|node| node.material.is_some())
        .nth(0)
        .unwrap();

    let material = &data.materials[&node.material.unwrap()];

    assert_eq!(material.alpha_mode, ALPHA_MODE);
    assert_eq!(material.base_color, BASE_COLOR);
    assert_eq!(material.roughness, ROUGHNESS);
    assert_eq!(material.metallic, METALLIC);

    let base_color_texture = &data.images[&material.base_color_texture.unwrap()];

    assert_eq!(base_color_texture, &load_image(BASE_COLOR_TEXTURE));
}

fn load_image(buf: &[u8]) -> Image {
    let img = image::load_from_memory(buf).unwrap().into_rgba8();
    Image::new(
        UVec2::new(img.width(), img.height()),
        TextureFormat::Rgba8UnormSrgb,
        img.into_raw(),
    )
}
