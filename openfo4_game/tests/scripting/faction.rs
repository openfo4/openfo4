use openfo4_esp::records::faction::Faction;
use openfo4_game::systems::global::FactionReaction;
use openfo4_game::systems::scripting::invocations::Value;

use crate::runner::TestRunner;

#[test]
fn faction_set_ally() {
    let mut builder = TestRunner::builder();

    let faction_id1 = builder.add_record(Faction::default());
    let faction_id2 = builder.add_record(Faction::default());

    let mut runner = builder.build();

    let faction1 = runner.load(faction_id1);
    let faction2 = runner.load(faction_id2);

    for (
        self_is_friend_to_other,
        other_is_friend_to_self,
        faction1_to_faction2,
        faction2_to_faction1,
    ) in [
        (false, false, FactionReaction::Ally, FactionReaction::Ally),
        (false, true, FactionReaction::Ally, FactionReaction::Friend),
        (true, false, FactionReaction::Friend, FactionReaction::Ally),
        (true, true, FactionReaction::Friend, FactionReaction::Friend),
    ] {
        let other = runner.script(&faction2, "Faction");
        runner.call(
            &faction1,
            "SetAlly",
            [
                Value::Object(other),
                Value::Bool(self_is_friend_to_other),
                Value::Bool(other_is_friend_to_self),
            ],
        );

        let factions = runner.game.globals.factions.lock();

        let faction1 = factions.get(&faction_id1).unwrap();
        assert_eq!(faction1.relation(faction_id2), faction1_to_faction2);

        let faction2 = factions.get(&faction_id2).unwrap();
        assert_eq!(faction2.relation(faction_id1), faction2_to_faction1);
    }
}

#[test]
fn faction_set_enemy() {
    let mut builder = TestRunner::builder();

    let faction_id1 = builder.add_record(Faction::default());
    let faction_id2 = builder.add_record(Faction::default());

    let mut runner = builder.build();

    let faction1 = runner.load(faction_id1);
    let faction2 = runner.load(faction_id2);

    for (
        self_is_neutral_to_other,
        other_is_neutral_to_self,
        faction1_to_faction2,
        faction2_to_faction1,
    ) in [
        (false, false, FactionReaction::Enemy, FactionReaction::Enemy),
        (
            false,
            true,
            FactionReaction::Enemy,
            FactionReaction::Neutral,
        ),
        (
            true,
            false,
            FactionReaction::Neutral,
            FactionReaction::Enemy,
        ),
        (
            true,
            true,
            FactionReaction::Neutral,
            FactionReaction::Neutral,
        ),
    ] {
        let other = runner.script(&faction2, "Faction");
        runner.call(
            &faction1,
            "SetEnemy",
            [
                Value::Object(other),
                Value::Bool(self_is_neutral_to_other),
                Value::Bool(other_is_neutral_to_self),
            ],
        );

        let factions = runner.game.globals.factions.lock();

        let faction1 = factions.get(&faction_id1).unwrap();
        assert_eq!(faction1.relation(faction_id2), faction1_to_faction2);

        let faction2 = factions.get(&faction_id2).unwrap();
        assert_eq!(faction2.relation(faction_id1), faction2_to_faction1);
    }
}
