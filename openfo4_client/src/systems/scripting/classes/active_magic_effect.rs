use openfo4_script::runtime::{Caller, Exception};

use super::{Object, State};

/// `Dispel`
///
/// ```text
/// Function Dispel() native
/// ```
pub fn dispel(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `GetBaseObject`
///
/// ```text
/// MagicEffect Function GetBaseObject() native
/// ```
pub fn get_base_object(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetCasterActor`
///
/// ```text
/// Actor Function GetCasterActor() native
/// ```
pub fn get_caster_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetTargetActor`
///
/// ```text
/// Actor Function GetTargetActor() native
/// ```
pub fn get_target_actor(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}
