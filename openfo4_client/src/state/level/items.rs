use hashbrown::HashSet;
use openfo4_core::record::RecordId;
use openfo4_esp::RecordData;

use crate::state::equipment::load_equipment_item;
use crate::state::esp::EspLoader;
use crate::state::level::leveled_list::select_from_leveled_list;
use crate::systems::inventory::{ItemData, ItemKind, ItemStack};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};

// FIXME: Unhardcode this guy.
const PLAYER_LEVEL: u16 = 1000;

pub async fn resolve_item(
    esp_loader: &EspLoader,
    script_loader: &ScriptLoader,
    record_id: RecordId,
    count: u64,
    attachments: &[RecordId],
) -> Vec<ItemStack> {
    let mut items = Vec::new();

    let mut visited = HashSet::new();

    let mut queue = vec![(record_id, count, attachments.to_vec())];
    while let Some((record_id, count, attachments)) = queue.pop() {
        visited.insert(record_id);

        let record = esp_loader.lookup(None, record_id).await.unwrap();
        match record.local.data {
            RecordData::LeveledItem(mut data) => {
                // Discard any entries that we have already seen, otherwise
                // we might get a cycle in the reference graph.
                data.entries.retain(|entry| {
                    esp_loader
                        .resolve_form(entry.reference, record_id)
                        .is_some_and(|id| !visited.contains(&id))
                });

                let Some(index) = select_from_leveled_list(data.flags, &data.entries, PLAYER_LEVEL)
                else {
                    continue;
                };

                let form = data.entries[index].reference;
                let count = u64::from(data.entries[index].count);
                let rid = esp_loader.resolve_form(form, record_id).unwrap();
                queue.push((rid, count, Vec::new()));
            }
            RecordData::MiscObject(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Weapon(_) | RecordData::Armor(_) => {
                // FIXME: Merge this function into this file.
                let item =
                    load_equipment_item(esp_loader, script_loader, record_id, &attachments).await;

                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Ammo(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Ingestible(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Key(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Book(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Component(data) => {
                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            RecordData::Note(data) => {
                let script = load_item_script(record.id, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
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
                    script,
                });
            }
            x => todo!("{:?}", x),
        }
    }

    items
}

async fn load_item_script(record: RecordId, loader: &ScriptLoader) -> Option<ScriptObject> {
    let base_object = loader.load(record).await?.object;
    Some(loader.clone_object(&base_object).await)
}
