use openfo4_esp::records::{PlacedObject, Quest, QuestAliasFlags, QuestReferenceAlias};
use openfo4_game::systems::scripting::invocations::Value;
use openfo4_game::systems::scripting::loader::AliasKey;

use crate::runner::{DefaultExt, TestRunner};

#[test]
fn reference_alias_get_reference() {
    let mut builder = TestRunner::builder();

    let refr = builder.add_reference_record(PlacedObject::default());
    let quest_id = builder.add_record(Quest::default_with(|quest| {
        let mut alias = QuestReferenceAlias::default();
        alias.id = 1;
        alias.forced_reference = Some(refr.form);
        quest.aliases.references.push(alias);
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);
    let mut quest = runner.load_with_aliases(quest_id);
    let alias = quest
        .aliases
        .remove(&AliasKey {
            accessor: quest_id,
            owner: quest_id,
            id: 1,
        })
        .unwrap();
    let quest = quest.object;

    runner.call_assert(&alias, "GetReference", [], Value::Null);

    // Start the quest to fill aliases.
    runner.call(&quest, "Start", []);

    let refr = runner.script(&refr, "ObjectReference");
    runner.call_assert(&alias, "GetReference", [], refr);

    // Stopping the quest will clear aliases.
    runner.call(&quest, "Stop", []);
    runner.call_assert(&alias, "GetReference", [], Value::Null);
}

#[test]
fn reference_alias_clear() {
    let mut builder = TestRunner::builder();

    let refr = builder.add_reference_record(PlacedObject::default());
    let quest_id = builder.add_record(Quest::default_with(|quest| {
        let mut alias = QuestReferenceAlias::default();
        alias.id = 1;
        alias.forced_reference = Some(refr.form);
        alias.flags |= QuestAliasFlags::OPTIONAL;
        quest.aliases.references.push(alias);
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);
    let mut quest = runner.load_with_aliases(quest_id);
    let alias = quest
        .aliases
        .remove(&AliasKey {
            accessor: quest_id,
            owner: quest_id,
            id: 1,
        })
        .unwrap();
    let quest = quest.object;

    runner.call(&quest, "Start", []);

    let refr = runner.script(&refr, "ObjectReference");
    runner.call_assert(&alias, "GetReference", [], refr);

    runner.call(&alias, "Clear", []);
    runner.call_assert(&alias, "GetReference", [], Value::Null);
}

#[test]
fn reference_alias_clear_non_optional() {
    let mut builder = TestRunner::builder();

    let refr = builder.add_reference_record(PlacedObject::default());
    let quest_id = builder.add_record(Quest::default_with(|quest| {
        let mut alias = QuestReferenceAlias::default();
        alias.id = 1;
        alias.forced_reference = Some(refr.form);
        quest.aliases.references.push(alias);
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);
    let mut quest = runner.load_with_aliases(quest_id);
    let alias = quest
        .aliases
        .remove(&AliasKey {
            accessor: quest_id,
            owner: quest_id,
            id: 1,
        })
        .unwrap();
    let quest = quest.object;

    runner.call(&quest, "Start", []);

    let refr = runner.script(&refr, "ObjectReference");
    runner.call_assert(&alias, "GetReference", [], refr.clone());

    runner.call_fallible(&alias, "Clear", []);
    runner.call_assert(&alias, "GetReference", [], refr);
}

#[test]
fn reference_alias_force_to_ref() {
    let mut builder = TestRunner::builder();

    let refr = builder.add_reference_record(PlacedObject::default());
    let quest_id = builder.add_record(Quest::default_with(|quest| {
        let mut alias = QuestReferenceAlias::default();
        alias.id = 1;
        alias.forced_reference = Some(refr.form);
        quest.aliases.references.push(alias);
    }));

    let mut runner = builder.build();

    let refr = runner.load(refr);
    let mut quest = runner.load_with_aliases(quest_id);
    let alias = quest
        .aliases
        .remove(&AliasKey {
            accessor: quest_id,
            owner: quest_id,
            id: 1,
        })
        .unwrap();
    let quest = quest.object;

    runner.call(&quest, "Start", []);

    let refr = runner.script(&refr, "ObjectReference");
    runner.call_assert(&alias, "GetReference", [], refr.clone());

    runner.call_fallible(&alias, "Clear", []);
    runner.call_assert(&alias, "GetReference", [], refr);
}
