use hashbrown::HashMap;
use openfo4_core::record::RecordId;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::esp::EspLoader;
use crate::systems::global::FormList;

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

pub async fn load_form_lists(esp_loader: &EspLoader) -> HashMap<RecordId, FormList> {
    let mut lists = HashMap::new();

    for record in esp_loader
        .lookup_all(Some(RecordKind::FormList))
        .await
        .unwrap()
    {
        let RecordData::FormList(data) = record.local.data else {
            unreachable!();
        };

        let forms = data
            .forms
            .iter()
            .map(|form| esp_loader.resolve_form(*form, record.id).unwrap());
        lists.insert(record.id, FormList::new(forms));
    }

    lists
}
