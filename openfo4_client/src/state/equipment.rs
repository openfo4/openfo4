use openfo4_asset::{AssetLoader, Handle};
use openfo4_esp::{FormId, RecordData, RecordKind};

use crate::assets::nif_v2::NifScene;
use crate::state::esp::EspLoader;

#[derive(Clone, Debug)]
pub struct EquipmentItem {
    pub models: Vec<Handle<NifScene>>,
}

pub async fn load_armor(
    loader: &AssetLoader,
    esp_loader: &EspLoader,
    armor_form: FormId,
    female: bool,
) -> EquipmentItem {
    let armor = esp_loader
        .lookup(Some(RecordKind::Armor), armor_form)
        .await
        .unwrap();
    let armor = match armor.data {
        RecordData::Armor(data) => data,
        _ => unreachable!(),
    };

    let mut models = Vec::new();

    for model in &*armor.models {
        let addon = esp_loader
            .lookup(Some(RecordKind::ArmorAddon), model.armor_addon)
            .await
            .unwrap();
        let addon = match addon.data {
            RecordData::ArmorAddon(data) => data,
            _ => unreachable!(),
        };

        let mut model_path = None;
        if female {
            if let Some(model) = addon.female_model.model {
                model_path = Some(model);
            }
        } else {
            if let Some(model) = addon.male_model.model {
                model_path = Some(model);
            }
        }

        let Some(model_path) = model_path else {
            continue;
        };

        let scene = loader
            .load::<NifScene, _>(model_path.as_str().unwrap())
            .await
            .unwrap();
        models.push(scene);
    }

    EquipmentItem { models }
}
