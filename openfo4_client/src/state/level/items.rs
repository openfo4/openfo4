use hashbrown::HashSet;
use openfo4_core::record::RecordId;
use openfo4_esp::records::object_modification::ObjectModificationFormType;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::equipment::load_equipment_item;
use crate::state::esp::{EspLoader, GlobalRecord};
use crate::state::level::data_ext::RecordDataExt;
use crate::state::level::leveled_list::select_from_leveled_list;
use crate::systems::inventory::{ItemData, ItemKind, ItemStack, ItemVisibilityFlags};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject};

pub const VISIBILITY_PRIORITY_SKIN: u8 = 0;
pub const VISIBILITY_PRIORITY_WEAPON_ARMOR: u8 = 255;

#[derive(Clone, Debug)]
pub struct LoadItemProps {
    pub level: u16,
    /// If loading an object template, only include items with these keywords.
    pub object_template_keywords: HashSet<RecordId>,
}

pub async fn resolve_item(
    esp_loader: &EspLoader,
    script_loader: &ScriptLoader,
    record_id: RecordId,
    count: u64,
    attachments: &[RecordId],
    props: &LoadItemProps,
) -> Vec<ItemStack> {
    let mut items = Vec::new();

    let mut visited = HashSet::new();

    let mut queue = vec![(record_id, count, attachments.to_vec())];
    while let Some((record_id, count, attachments)) = queue.pop() {
        visited.insert(record_id);

        let record = esp_loader.lookup(None, record_id).await.unwrap();
        match &record.local.data {
            RecordData::LeveledItem(data) => {
                let mut data = data.clone();
                // Discard any entries that we have already seen, otherwise
                // we might get a cycle in the reference graph.
                data.entries.retain(|entry| {
                    esp_loader
                        .resolve_form(entry.reference, record_id)
                        .is_some_and(|id| !visited.contains(&id))
                });

                for entry in select_from_leveled_list(data.flags, &data.entries, count, props.level)
                {
                    let form = data.entries[entry.index].reference;
                    let count = entry.count * u64::from(data.entries[entry.index].count);
                    let rid = esp_loader.resolve_form(form, record_id).unwrap();
                    queue.push((rid, count, Vec::new()));
                }
            }
            RecordData::MiscObject(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_owned(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::Weapon(_) | RecordData::Armor(_) => {
                let mut attachments = attachments;
                if attachments.is_empty() {
                    attachments = select_object_attachments(
                        esp_loader,
                        record_id,
                        &record.local.data,
                        &props,
                    )
                    .await;
                }

                // FIXME: Merge this function into this file.
                let item =
                    load_equipment_item(esp_loader, script_loader, record_id, &attachments).await;

                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: item.name,
                    hidden: false,
                    kind: item.kind,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: item.base_keywords,
                    combined_keywords: item.combined_keywords,
                    data: item.data,
                    equipment_keys: Vec::new(),
                    equipment_slots: item.equipment_slots,
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags {
                        slots: item.visibility_slots,
                        priority: VISIBILITY_PRIORITY_WEAPON_ARMOR,
                    },
                });
            }
            RecordData::Ammo(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Ammo,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::Ingestible(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Aid,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::Key(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::Book(data) => {
                let keywords: HashSet<RecordId> = data
                    .keywords
                    .iter()
                    .flatten()
                    .map(|form| esp_loader.resolve_form(*form, record_id).unwrap())
                    .collect();

                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: keywords.clone(),
                    combined_keywords: keywords,
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::Component(data) => {
                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Junk,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: HashSet::new(),
                    combined_keywords: HashSet::new(),
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::Note(data) => {
                let script = load_item_script(&record, script_loader).await;

                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::Misc,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: HashSet::new(),
                    combined_keywords: HashSet::new(),
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            RecordData::HeadPart(data) => {
                items.push(ItemStack {
                    item: record_id,
                    name: data.id.as_str().unwrap().to_string(),
                    hidden: false,
                    kind: ItemKind::HeadPart,
                    count,
                    attachments,
                    equipped: false,
                    base_keywords: HashSet::new(),
                    combined_keywords: HashSet::new(),
                    data: ItemData::None,
                    equipment_keys: Vec::new(),
                    equipment_slots: HashSet::new(),
                    visible: HashSet::new(),
                    script: None,
                    visibility: ItemVisibilityFlags::default(),
                });
            }
            x => todo!("{:?}", x),
        }
    }

    items
}

async fn load_item_script(record: &GlobalRecord, loader: &ScriptLoader) -> Option<ScriptObject> {
    // Most items will have no scripts attached. Loading the base script
    // has no effect. If this item is needed as a script object later it
    // is lazily loaded.
    if record.local.data.virtual_machine_adapter().is_none() {
        return None;
    }

    loader.load_instance(record.id).await
}

async fn select_object_attachments(
    esp_loader: &EspLoader,
    id: RecordId,
    data: &RecordData,
    props: &LoadItemProps,
) -> Vec<RecordId> {
    let templates = match data {
        RecordData::Weapon(data) => &data.object_templates.templates,
        RecordData::Armor(data) => &data.object_templates.templates,
        _ => return Vec::new(),
    };

    let mut chosen_template = templates.iter().find(|t| t.item.default);

    // Choose the object template with the most matching keywords.
    // Exclude all templates where not all keywords are matching.
    let mut max_keywords = 0;

    'outer: for (index, template) in templates.iter().enumerate() {
        if u16::from(template.item.min_level) > props.level {
            continue;
        }

        if template.item.max_level != 0 && u16::from(template.item.max_level) < props.level {
            continue;
        }

        let mut next = Some(index);
        let mut keywords = 0;
        while let Some(index) = next {
            let template = templates.get(index).unwrap();

            for keyword in &template.item.keywords {
                let keyword = esp_loader.resolve_form(*keyword, id).unwrap();
                if !props.object_template_keywords.contains(&keyword) {
                    continue 'outer;
                }
            }

            keywords += template.item.keywords.len();
            next = template.item.parent_combination_index.map(|i| i.into());
        }

        if keywords > max_keywords {
            chosen_template = Some(template);
            max_keywords = keywords;
        }
    }

    let Some(template) = chosen_template else {
        return Vec::new();
    };

    let mut visited = HashSet::new();

    let mut attachments = Vec::new();

    let mut queue = Vec::new();
    for attachment in &template.item.includes {
        if attachment.optional && rand::random::<bool>() {
            continue;
        }

        let attachment_id = esp_loader.resolve_form(attachment.item_mod, id).unwrap();
        queue.push((attachment_id, !attachment.dont_use_all));
    }

    while let Some((id, use_all)) = queue.pop() {
        // We must protect against cyclic references in mod collections.
        if !visited.insert(id) {
            continue;
        }

        let attachment = esp_loader
            .lookup(Some(RecordKind::ObjectModification), id)
            .await
            .unwrap();
        match attachment.local.data {
            RecordData::ObjectModification(data) => {
                // We can generally differentiate between "normal" object mods
                // and mod collections (called "modcols" in the game).
                // Normal object mods are attachments that directly modify the visuals
                // and/or stats of the weapon/armor.
                // Mod collections only contain a list of other mods that are applicable,
                // and don't modify the weapon/armor directly.

                // Mod collections have form type "none".
                if data.data.form_type != ObjectModificationFormType::None {
                    attachments.push(id);
                    continue;
                }

                let mut chosen_mod = None;
                let mut chosen_mod_level = 0;

                for item in &data.data.includes {
                    if item.optional && rand::random::<bool>() {
                        continue;
                    }

                    if u16::from(item.min_level) > props.level {
                        continue;
                    }

                    if use_all {
                        let next_id = esp_loader.resolve_form(item.mod_, id).unwrap();
                        queue.push((next_id, !item.dont_use_all));
                    }

                    if item.min_level > chosen_mod_level || chosen_mod.is_none() {
                        chosen_mod = Some(item);
                        chosen_mod_level = item.min_level;
                    }
                }

                let Some(item) = chosen_mod else {
                    continue;
                };

                let id = esp_loader.resolve_form(item.mod_, id).unwrap();
                queue.push((id, !item.dont_use_all));
            }
            _ => todo!(),
        }
    }

    attachments
}
