use hashbrown::HashMap;
use openfo4_core::record::RecordId;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::esp::EspLoader;

pub async fn load_global_variables(esp_loader: &EspLoader) -> HashMap<RecordId, f32> {
    let mut globals = HashMap::new();

    for record in esp_loader
        .lookup_all(Some(RecordKind::GlobalVariable))
        .await
        .unwrap()
    {
        let RecordData::GLOB(data) = record.local.data else {
            unreachable!();
        };

        globals.insert(record.id, data.fltv);
    }

    globals
}
