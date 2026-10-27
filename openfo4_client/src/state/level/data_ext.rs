use openfo4_esp::records::{Array, Bounds, PositionRotation};
use openfo4_esp::vmad::VirtualMachineAdapter;
use openfo4_esp::{FormId, RecordData, ZString};

use crate::state::level::partial::LodLevel;

pub trait RecordDataExt {
    fn position(&self) -> Option<PositionRotation>;

    fn model(&self, lod: LodLevel) -> Option<&ZString>;

    fn object_bounds(&self) -> Option<Bounds>;

    fn keywords(&self) -> &[FormId];

    fn virtual_machine_adapter(&self) -> Option<&VirtualMachineAdapter>;
}

impl RecordDataExt for RecordData {
    fn position(&self) -> Option<PositionRotation> {
        match self {
            Self::PlacedObject(data) => Some(data.position),
            Self::PlacedNpc(data) => Some(data.position),
            Self::PlacedGrenade(data) => Some(data.position),
            Self::PlacedHazard(data) => Some(data.position),
            Self::PlacedMissile(data) => Some(data.position),
            _ => None,
        }
    }

    fn model(&self, lod: LodLevel) -> Option<&ZString> {
        match self {
            Self::STAT(data) => match lod {
                LodLevel::Normal => data.modl.model.as_ref(),
                LodLevel::Distant0
                    if let Some(lods) = &data.lods
                        && !lods.level0.is_empty() =>
                {
                    Some(&lods.level0)
                }
                _ => None,
            },
            Self::StaticCollection(data) => data.model.model.as_ref(),
            Self::MovableStatic(data) => data.model.model.as_ref(),
            Self::Door(data) => data.model.model.as_ref(),
            Self::Container(data) => data.model.model.as_ref(),
            Self::Activator(data) => data.model.model.as_ref(),
            Self::MiscObject(data) => data.model.model.as_ref(),
            Self::Furniture(data) => data.model.model.as_ref(),
            Self::Light(data) => data.model.model.as_ref(),
            Self::Ingestible(data) => data.model.model.as_ref(),
            Self::Ammo(data) => data.model.model.as_ref(),
            Self::Weapon(data) => data.model.model.as_ref(),
            Self::Armor(data) => data.male_world_model.model.as_ref(),
            Self::Book(data) => data.model.model.as_ref(),
            Self::Note(data) => data.model.model.as_ref(),
            Self::Key(data) => data.model.model.as_ref(),
            Self::ObjectModification(data) => data.model.model.as_ref(),
            Self::Projectile(data) => data.model.model.as_ref(),
            Self::TACT(data) => data.model.model.as_ref(),
            Self::Terminal(data) => data.model.model.as_ref(),
            Self::Tree(data) => data.model.model.as_ref(),
            Self::GRAS(data) => data.model.model.as_ref(),
            Self::Hazard(data) => data.model.model.as_ref(),
            Self::HeadPart(data) => data.model.model.as_ref(),
            Self::ArtObject(_) | Self::AddonNode(_) | Self::Impact(_) => todo!(),
            _ => None,
        }
    }

    fn object_bounds(&self) -> Option<Bounds> {
        match self {
            Self::STAT(data) => Some(data.object_bounds),
            Self::StaticCollection(data) => Some(data.object_bounds),
            Self::MovableStatic(data) => Some(data.object_bounds),
            Self::Door(data) => Some(data.object_bounds),
            Self::Container(data) => Some(data.object_bounds),
            Self::Activator(data) => Some(data.object_bounds),
            Self::MiscObject(data) => Some(data.object_bounds),
            Self::Furniture(data) => Some(data.object_bounds),
            Self::Light(data) => Some(data.object_bounds),
            Self::Ingestible(data) => Some(data.object_bounds),
            Self::Ammo(data) => Some(data.object_bounds),
            Self::Weapon(data) => data.object_bounds,
            Self::Armor(data) => data.object_bounds,
            Self::Book(data) => Some(data.objet_bounds),
            Self::Note(data) => Some(data.object_bounds),
            Self::Key(data) => Some(data.object_bounds),
            Self::Projectile(data) => Some(data.object_bounds),
            Self::TACT(data) => Some(data.obnd),
            Self::Terminal(data) => data.object_bounds,
            Self::Tree(data) => data.object_bounds,
            Self::Hazard(data) => Some(data.object_bounds),
            Self::ArtObject(_) | Self::AddonNode(_) => todo!(),
            _ => None,
        }
    }

    fn keywords(&self) -> &[FormId] {
        fn get_or_default(keywords: &Option<Array<FormId>>) -> &[FormId] {
            keywords.as_ref().map(|v| v.as_slice()).unwrap_or_default()
        }

        match self {
            Self::STAT(_) => &[],
            Self::StaticCollection(_) => &[],
            Self::MovableStatic(_) => &[],
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
            Self::MovableStatic(data) => data.virtual_machine_adapter.as_ref(),
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
            Self::Note(data) => data.virtual_machine_adapter.as_ref(),
            Self::Terminal(data) => data.virtual_machine_adapter.as_ref(),
            Self::PlacedNpc(data) => data.virtual_machine_adapter.as_ref(),
            Self::PlacedObject(data) => data.virtual_machine_adapter.as_ref(),
            _ => None,
        }
    }
}
