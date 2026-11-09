use hashbrown::HashMap;
use openfo4_common::string::Lowercase;
use openfo4_core::record::RecordId;
use openfo4_esp::records::faction::FactionCombatReaction;
use openfo4_esp::{RecordData, RecordKind};

use crate::state::esp::EspLoader;
use crate::systems::global::{Faction, FactionReaction, FormList};

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

        let forms = data.forms.iter().filter_map(|form| {
            esp_loader.resolve_form(*form, record.id).or_else(|| {
                // TODO: Don't do this, something with the id resolving is weird.
                tracing::warn!("failed to resolve {:?} in formlist {:?}", form, record.id);
                None
            })
        });
        lists.insert(record.id, FormList::new(forms));
    }

    lists
}

pub async fn load_factions(esp_loader: &EspLoader) -> HashMap<RecordId, Faction> {
    let mut factions = HashMap::new();

    for record in esp_loader
        .lookup_all(Some(RecordKind::Faction))
        .await
        .unwrap()
    {
        let RecordData::Faction(faction) = record.local.data else {
            unreachable!();
        };

        let mut relations = HashMap::new();
        for relation in faction.relations {
            let faction = esp_loader
                .resolve_form(relation.faction, record.id)
                .unwrap();

            let reaction = match relation.reaction {
                FactionCombatReaction::Neutral => FactionReaction::Neutral,
                FactionCombatReaction::Ally => FactionReaction::Ally,
                FactionCombatReaction::Enemy => FactionReaction::Enemy,
                FactionCombatReaction::Friend => FactionReaction::Friend,
            };

            relations.insert(faction, reaction);
        }

        factions.insert(
            record.id,
            Faction {
                relations,
                player_enemy: false,
                player_expelled: false,
            },
        );
    }

    factions
}

pub async fn load_voice_types(esp_loader: &EspLoader) -> HashMap<RecordId, Lowercase<String>> {
    let mut types = HashMap::new();

    for record in esp_loader
        .lookup_all(Some(RecordKind::VoiceType))
        .await
        .unwrap()
    {
        let RecordData::VoiceType(data) = record.local.data else {
            continue;
        };

        let name = data.id.as_ref().unwrap().as_str().unwrap();
        types.insert(record.id, Lowercase::new(name));
    }

    types
}
