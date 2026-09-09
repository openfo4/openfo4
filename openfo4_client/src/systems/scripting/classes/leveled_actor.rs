use openfo4_script::engine::Engine;
use openfo4_script::runtime::Caller;

use crate::systems::global::LevelListAddition;
use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("LeveledActor", "AddForm", add_form);
    engine.register_if_exists("LeveledActor", "Revert", revert);
}

/// `AddForm`
///
/// ```text
/// Function AddForm(Form apForm, int aiLevel) native
/// ```
fn add_form(caller: Caller<'_, State>, list: Object, item: Object, level: i32) {
    let list = caller.data().objects.get_by_addr(list).unwrap();
    let list = list.record().unwrap();

    let item = caller.data().objects.get_by_addr(item).unwrap();
    let item = item.record().unwrap();

    // TODO: Validate here that this is actually an actorbase record.

    let level = u16::try_from(level).unwrap_or(u16::MAX);

    let mut lists = caller.data().globals.mods.leveled_list.lock();
    lists.entry(list).or_default().push(LevelListAddition {
        item,
        level,
        count: 1,
    });
}

/// `Revert`
///
/// ```text
/// Function Revert() native
/// ```
fn revert(caller: Caller<'_, State>, list: Object) {
    let list = caller.data().objects.get_by_addr(list).unwrap();
    let list = list.record().unwrap();

    let mut lists = caller.data().globals.mods.leveled_list.lock();
    lists.remove(&list);
}
