use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Armor", "GetArmorAddons", get_armor_addons);
}

/// `GetArmorAddons`
///
/// ```text
/// ArmorAddon[] Function GetArmorAddons() native
/// ```
fn get_armor_addons(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Error> {
    todo!()
}
