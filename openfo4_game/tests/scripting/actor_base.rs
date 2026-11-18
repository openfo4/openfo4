use openfo4_esp::records::{Array, Class, Keyword, Npc, NpcBaseFlags, Race};
use openfo4_game::systems::scripting::invocations::Value;

use crate::runner::{DefaultExt, TestRunner};

#[test]
fn form_has_keyword() {
    let mut builder = TestRunner::builder();

    let keyword = builder.add_record(Keyword::default());

    let actor_f = builder.add_record(Npc::default());
    let actor_t = builder.add_record(Npc::default_with(|npc| {
        npc.keywords = Some(Array(vec![keyword.form]));
    }));

    let mut runner = builder.build();

    let keyword = runner.load(keyword);
    let keyword = runner.script(&keyword, "Keyword");

    let actor_f = runner.load(actor_f);
    let actor_t = runner.load(actor_t);

    runner.call_assert(
        actor_f,
        "HasKeyword",
        [Value::Object(keyword.clone())],
        false,
    );
    runner.call_assert(actor_t, "HasKeyword", [Value::Object(keyword)], true);
}

#[test]
fn actor_base_get_class() {
    let mut builder = TestRunner::builder();

    let class = builder.add_record(Class::default());
    let actor = builder.add_record(Npc::default_with(|npc| {
        npc.class = Some(class.form);
    }));

    let mut runner = builder.build();
    let class = runner.load(class);
    let class = runner.script(&class, "Class");
    let actor = runner.load(actor);

    runner.call_assert(actor, "GetClass", [], class);
}

#[test]
fn actor_base_get_race() {
    let mut builder = TestRunner::builder();

    let race = builder.add_record(Race::default());
    let actor = builder.add_record(Npc::default_with(|npc| {
        npc.race = race.form;
    }));

    let mut runner = builder.build();
    let race = runner.load(race);
    let race = runner.script(&race, "Race");
    let actor = runner.load(actor);

    runner.call_assert(actor, "GetRace", [], race);
}

#[test]
fn actor_base_get_level_exact() {
    let mut builder = TestRunner::builder();

    let mut actor = Npc::default();
    actor.base_stats.level = 23;
    let actor = builder.add_record(actor);

    let mut runner = builder.build();
    let actor = runner.load(actor);

    runner.call_assert(actor, "GetLevelExact", [], 23);
}

#[test]
fn actor_base_get_sex() {
    let mut builder = TestRunner::builder();

    let actor_m = builder.add_record(Npc::default());

    let mut actor_f = Npc::default();
    actor_f.base_stats.flags.insert(NpcBaseFlags::FEMALE);
    let actor_f = builder.add_record(actor_f);

    let mut runner = builder.build();
    let actor_m = runner.load(actor_m);
    let actor_f = runner.load(actor_f);

    runner.call_assert(actor_m, "GetSex", [], 0);
    runner.call_assert(actor_f, "GetSex", [], 1);
}

#[test]
fn actor_base_is_essential() {
    let mut builder = TestRunner::builder();

    let actor_f = builder.add_record(Npc::default());
    let actor_t = builder.add_record(Npc::default_with(|npc| {
        npc.base_stats.flags |= NpcBaseFlags::ESSENTIAL;
    }));

    let mut runner = builder.build();
    let actor_f = runner.load(actor_f);
    let actor_t = runner.load(actor_t);

    runner.call_assert(actor_f, "IsEssential", [], false);
    runner.call_assert(actor_t, "IsEssential", [], true);
}

#[test]
fn actor_base_is_invulnerable() {
    let mut builder = TestRunner::builder();

    let actor_f = builder.add_record(Npc::default());
    let actor_t = builder.add_record(Npc::default_with(|npc| {
        npc.base_stats.flags |= NpcBaseFlags::INVULNERABLE;
    }));

    let mut runner = builder.build();
    let actor_f = runner.load(actor_f);
    let actor_t = runner.load(actor_t);

    runner.call_assert(actor_f, "IsInvulnerable", [], false);
    runner.call_assert(actor_t, "IsInvulnerable", [], true);
}

#[test]
fn actor_base_is_protected() {
    let mut builder = TestRunner::builder();

    let actor_f = builder.add_record(Npc::default());
    let actor_t = builder.add_record(Npc::default_with(|npc| {
        npc.base_stats.flags |= NpcBaseFlags::PROTECTED;
    }));

    let mut runner = builder.build();
    let actor_f = runner.load(actor_f);
    let actor_t = runner.load(actor_t);

    runner.call_assert(actor_f, "IsProtected", [], false);
    runner.call_assert(actor_t, "IsProtected", [], true);
}

#[test]
fn actor_base_is_unique() {
    let mut builder = TestRunner::builder();

    let actor_f = builder.add_record(Npc::default());
    let actor_t = builder.add_record(Npc::default_with(|npc| {
        npc.base_stats.flags |= NpcBaseFlags::UNIQUE;
    }));

    let mut runner = builder.build();
    let actor_f = runner.load(actor_f);
    let actor_t = runner.load(actor_t);

    runner.call_assert(actor_f, "IsUnique", [], false);
    runner.call_assert(actor_t, "IsUnique", [], true);
}
