use openfo4_esp::records::Array;
use openfo4_esp::vmad::VirtualMachineAdapter;
use openfo4_esp::{FormId, RecordData};

pub trait RecordDataExt {
    fn keywords(&self) -> &[FormId];

    fn virtual_machine_adapter(&self) -> Option<&VirtualMachineAdapter>;
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

    fn virtual_machine_adapter(&self) -> Option<&VirtualMachineAdapter> {
        match self {
            Self::STAT(data) => data.virtual_machine_adapter.as_ref(),
            Self::MSTT(data) => data.virtual_machine_adapter.as_ref(),
            Self::Door(data) => data.virtual_machine_adapter.as_ref(),
            Self::Container(data) => data.virtual_machine_adapter.as_ref(),
            Self::Npc(data) => data.virtual_machine_adapter.as_ref(),
            Self::MiscObject(data) => data.virtual_machine_adapter.as_ref(),
            Self::Activator(data) => data.virtual_machine_adapter.as_ref(),
            Self::TACT(data) => data.vmad.as_ref(),
            Self::Furniture(data) => data.virtual_machine_adapter.as_ref(),
            Self::Light(data) => data.virtual_machine_adapter.as_ref(),
            Self::Armor(data) => data.virtual_machine_adapter.as_ref(),
            Self::Weapon(data) => data.virtual_machine_adapter.as_ref(),
            Self::Book(data) => data.virtual_machine_adapter.as_ref(),
            Self::Key(data) => data.virtual_machine_adapter.as_ref(),
            Self::PlacedNpc(data) => data.virtual_machine_adapter.as_ref(),
            Self::PlacedObject(data) => data.virtual_machine_adapter.as_ref(),
            _ => todo!(),
        }
    }
}
