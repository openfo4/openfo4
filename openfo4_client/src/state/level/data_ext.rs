use openfo4_esp::records::Array;
use openfo4_esp::{FormId, RecordData};

pub trait RecordDataExt {
    fn keywords(&self) -> &[FormId];
}

impl RecordDataExt for RecordData {
    fn keywords(&self) -> &[FormId] {
        fn get_or_default(keywords: &Option<Array<FormId>>) -> &[FormId] {
            keywords.as_ref().map(|v| v.as_slice()).unwrap_or_default()
        }

        match self {
            Self::STAT(_) => &[],
            Self::StaticCollection(_) => &[],
            Self::MSTT(_) => &[],
            Self::Door(_) => &[],
            Self::Container(data) => get_or_default(&data.keywords),
            Self::Npc(data) => get_or_default(&data.keywords),
            Self::IdleMarker(data) => &data.keywords,
            Self::MiscObject(data) => get_or_default(&data.keywords),
            Self::Activator(data) => get_or_default(&data.keywords),
            Self::Furniture(data) => get_or_default(&data.keywords),
            Self::Light(data) => get_or_default(&data.keywords),
            Self::Ingestible(data) => get_or_default(&data.keywords),
            Self::Armor(data) => get_or_default(&data.keywords),
            Self::Weapon(data) => get_or_default(&data.keywords),
            Self::Ammo(data) => get_or_default(&data.keywords),
            Self::Book(data) => get_or_default(&data.keywords),
            Self::Key(data) => get_or_default(&data.keywords),
            _ => &[],
        }
    }
}
