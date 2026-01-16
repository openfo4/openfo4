use hashbrown::HashSet;
use openfo4_core::record::RecordId;
use openfo4_esp::RecordData;

use crate::state::equipment::load_equipment_item;
use crate::state::esp::EspLoader;
use crate::state::level::leveled_list::select_from_leveled_list;
use crate::systems::inventory::{ItemData, ItemKind, ItemStack};

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

pub async fn resolve_item(
    esp_loader: &EspLoader,
    record_id: RecordId,
    count: u64,
) -> Vec<ItemStack> {
    let mut items = Vec::new();

    let mut queue = vec![(record_id, count)];
    while let Some((parent_rid, count)) = queue.pop() {
        let record = esp_loader.lookup(None, record_id).await.unwrap();
        match record.local.data {
            RecordData::LeveledItem(data) => {
                let Some(index) = select_from_leveled_list(data.flags, &data.entries, PLAYER_LEVEL)
                else {
                    continue;
                };

                let form = data.entries[index].reference;
                let count = u64::from(data.entries[index].count);
                let rid = esp_loader.resolve_form(form, parent_rid).unwrap();
                queue.push((rid, count));
            }
            RecordData::MiscObject(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, parent_rid).unwrap())
                    .collect();

                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_owned(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            RecordData::Weapon(_) | RecordData::Armor(_) => {
                // FIXME: Merge this function into this file.
                let item = load_equipment_item(esp_loader, parent_rid, &[]).await;

                items.push(ItemStack {
                    item: parent_rid,
                    name: item.name,
                    hidden: false,
                    kind: item.kind,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: item.base_keywords,
                    combined_keywords: item.combined_keywords,
                    data: item.data,
                    equipment_keys: Vec::new(),
                    equipment_slots: item.equipment_slots,
                    visibility_slots: item.visibility_slots,
                    visible: true,
                });
            }
            RecordData::Ammo(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, parent_rid).unwrap())
                    .collect();

                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Ammo,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            RecordData::Ingestible(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, parent_rid).unwrap())
                    .collect();

                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Aid,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            RecordData::Key(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, parent_rid).unwrap())
                    .collect();

                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            RecordData::Book(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, parent_rid).unwrap())
                    .collect();

                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            RecordData::Component(data) => {
                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Junk,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: HashSet::new(),
                    combined_keywords: HashSet::new(),
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            RecordData::Note(data) => {
                items.push(ItemStack {
                    item: parent_rid,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments: Vec::new(),
                    equipped: false,
                    base_keywords: HashSet::new(),
                    combined_keywords: HashSet::new(),
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visibility_slots: HashSet::new(),
                    visible: true,
                });
            }
            x => todo!("{:?}", x),
        }
    }

    items
}
