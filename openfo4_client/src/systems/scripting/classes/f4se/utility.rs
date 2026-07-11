use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::{EngineExt, Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Utility", "VarToVarArray", var_to_var_array);
    engine.register_if_exists("Utility", "VarArrayToVar", var_array_to_var);
}

/// `VarToVarArray`
///
/// ```text
/// Var[] Function VarToVarArray(Var v) native global
/// ```
fn var_to_var_array(caller: Caller<'_, State>, v: Object) -> Result<Object, Exception> {
    todo!()
}

/// `VarArrayToVar`
///
/// ```text
/// Var Function VarArrayToVar(Var[] v) native global
/// ```
fn var_array_to_var(caller: Caller<'_, State>, v: Object) -> Result<Object, Exception> {
    todo!()
}
