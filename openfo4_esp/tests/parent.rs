// `test.esp` contents:
// - Cells:
//   - 804
//     - Placed Object 805
//     - Placed Object 806
//   - 800
//     - Placed Object 801
//     - Placed Object 802
// - Worldspaces:
//   - 807
//     - Cell 809
//       - Placed Object 80a
//       - Placed Object 80b
//     - Cell 80c
//       - Placed Object 80d
//       - Placed Object 80e

use futures_lite::future::block_on;
use openfo4_esp::{EspFile, FormId};
use openfo4_io::fs::File;

const FILE: &str = "tests/parent.esp";

#[test]
fn worldspace_cell_parents() {
    let file = File::open(FILE).unwrap();
    let file = block_on(EspFile::new(file)).unwrap();

    assert_eq!(file.parent(FormId(0x804)), None);
    assert_eq!(file.parent(FormId(0x805)), Some(FormId(0x804)));
    assert_eq!(file.parent(FormId(0x806)), Some(FormId(0x804)));

    assert_eq!(file.parent(FormId(0x800)), None);
    assert_eq!(file.parent(FormId(0x801)), Some(FormId(0x800)));
    assert_eq!(file.parent(FormId(0x802)), Some(FormId(0x800)));

    assert_eq!(file.parent(FormId(0x807)), None);
    assert_eq!(file.parent(FormId(0x809)), Some(FormId(0x807)));
    assert_eq!(file.parent(FormId(0x80c)), Some(FormId(0x807)));

    assert_eq!(file.parent(FormId(0x80a)), Some(FormId(0x809)));
    assert_eq!(file.parent(FormId(0x80b)), Some(FormId(0x809)));

    assert_eq!(file.parent(FormId(0x80d)), Some(FormId(0x80c)));
    assert_eq!(file.parent(FormId(0x80e)), Some(FormId(0x80c)));
}
