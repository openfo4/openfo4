use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;

use bytes::Buf;
use openfo4_esp::{EspFile, FormId, RecordChildren, RecordData, RecordKind, Text};

fn main() {
    println!("Hello, world!");

    // let mut buf =
    //     std::fs::read("archive/raw/Data/meshes/actors/alien/characters/aliencharacter.hkx")
    //         .unwrap();
    // hkx::HkxFile::decode(&mut buf);

    // return;

    let prewar_vault = FormId(0xCA260);

    let mut file = BufReader::new(File::open("../../Fallout4.esm").unwrap());
    let mut file = EspFile::new(file).unwrap();

    // file.children(RecordChildren::Cells, FormId(0x3C)).unwrap();

    // let record = file.validate().unwrap();
    // dbg!(record.len());

    // dbg!(file.lookup(None, FormId(0x17f6d)));
    // dbg!(file.lookup(None, FormId(0xb1f08)));
    // dbg!(file.lookup(None, FormId(0x166729)));
    // dbg!(file.lookup(None, FormId(0x3c0f0)));
    // todo!()
    dbg!(file.lookup(None, FormId(0x80e)));
    dbg!(file.lookup(None, FormId(0x3648)));

    // return;

    let anims = file
        .lookup_all(Some(RecordKind::Animation))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    file.lookup_all(None)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let anims = file
        .lookup_all(Some(RecordKind::Animation))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    dbg!(anims);

    // let gmst = file.find_record(RecordKind::GameSetting, FormId(0x8BD));
    // let gmst = file.lookup(Some(RecordKind::Keyword), FormId(0xC1));
    // dbg!(&gmst);
    // let ent = file.lookup(Some(RecordKind::Cell), prewar_vault);
    // dbg!(&ent);
    for record in file
        .children(RecordChildren::TemporaryEntities, prewar_vault)
        .unwrap()
    {
        // dbg!(&record);
        // match &record.data {
        //     RecordData::PlacedObject(obj) => {
        //         let base = obj.base_object;
        //         let obj = file.lookup(None, base).unwrap();
        //         dbg!(&obj);
        //     }
        //     _ => (),
        // }
    }

    // let mut data = std::fs::read("Fallout4.esm").unwrap();
    // let mut buf = &data[..];

    // let res = (0..8)
    //     .map(|_| read_record(&mut buf).unwrap())
    //     .flatten()
    //     .collect::<Vec<_>>();
    // let out = serde_json::to_string_pretty(&res).unwrap();
    // std::fs::write("out.json", out).unwrap();

    // while buf.has_remaining() {
    //     read_record(&mut buf).unwrap();
    // }
}
