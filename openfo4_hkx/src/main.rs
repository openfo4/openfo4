use openfo4_hkx::HkxFile;

fn main() {
    let x = std::fs::read(
        "../../archive/raw/Data/meshes/actors/character/behaviors/raiderrootbehavior.hkx",
    )
    .unwrap();
    let f = HkxFile::decode(&x[..]).unwrap();
    dbg!(&f);
}
