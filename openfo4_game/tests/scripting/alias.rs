use openfo4_esp::records::{Quest, QuestReferenceAlias};
use openfo4_game::systems::scripting::loader::AliasKey;

use crate::runner::{DefaultExt, TestRunner};

#[test]
fn alias_get_owning_quest() {
    let mut builder = TestRunner::builder();

    let quest_id = builder.add_record(Quest::default_with(|quest| {
        let mut alias = QuestReferenceAlias::default();
        alias.id = 1;
        quest.aliases.references.push(alias);
    }));

    let mut runner = builder.build();

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

    let quest = runner.script(&quest, "Quest");
    runner.call_assert(&alias, "GetOwningQuest", [], quest);
}
