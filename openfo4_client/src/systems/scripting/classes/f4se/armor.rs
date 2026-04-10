use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_native_function("Armor", "GetArmorAddons", get_armor_addons);
}

/// `GetArmorAddons`
///
/// ```text
/// ArmorAddon[] Function GetArmorAddons() native
/// ```
fn get_armor_addons(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}
