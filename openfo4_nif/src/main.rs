use openfo4_nif::Nif;
use openfo4_nif::bgsm::Bgsm;

fn main() {
    // let x = std::fs::read("/home/robert/projects/fo4-engine/archive/raw/Data/materials/interiors/vault/vault111/v111cablebundle01.bgsm").unwrap();
    let x = std::fs::read("/home/robert/projects/fo4-engine/archive/raw/Data/meshes/setdressing/bricksblocks/cinderblocksingle01.nif").unwrap();

    let nif = Nif::decode(&x[..]).unwrap();
    dbg!(&nif);
}
