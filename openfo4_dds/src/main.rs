use image::{ImageBuffer, Rgba};
use openfo4_dds::Image;

fn main() {
    let x = std::fs::read(
        "/home/robert/projects/fo4-engine/archive/raw/Data/textures/ammo/5mm/5mmammobox_d.dds",
    )
    .unwrap();
    let x = std::fs::read(
        "../../archive/raw/Data/textures/actors/protectron/protectronarmormilitary_lgrad.dds",
    )
    .unwrap();

    let image = Image::parse(&x).unwrap();

    let mip = image.mips[0].bytes.to_vec();

    let mut buf = ImageBuffer::<Rgba<u8>, _>::new(image.mips[0].size.x, image.mips[0].size.y);
    buf.copy_from_slice(&mip);

    buf.save_with_format("out.png", image::ImageFormat::Png)
        .unwrap();
}
