use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{EngineExt, Object, State};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Cell", "GetWaterType", get_water_type);
}

/// `GetWaterType`
///
/// ```text
/// WaterType Function GetWaterType() native
/// ```
fn get_water_type(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}
