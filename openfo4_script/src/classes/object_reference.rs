use crate::classes::{Float, Object, State};
use crate::runtime::Caller;

/// `GetValue`
///
/// ```text
/// float Function GetValue(ActorValue akAV) native
/// ```
pub fn get_value(caller: Caller<'_, State>, self_ptr: Object, actor_value: Object) -> Float {
    todo!()
}

/// `SetValue`
///
/// ```text
/// Function SetValue(ActorValue akAV, float afValue) native
/// ```
pub fn set_value(caller: Caller<'_, State>, self_ptr: Object, actor_value: Object, value: Float) {
    todo!()
}

/// `GetBaseValue`
///
/// ```text
/// float Function GetBaseValue(ActorValue akAV) native
/// ```
pub fn get_base_value(caller: Caller<'_, State>, self_ptr: Object, actor_value: Object) -> Float {
    todo!()
}

/// `SetActorOwner`
///
/// ```text
/// Function SetActorOwner(ActorBase akActorBase, bool abNoCrime = false) native
/// ```
pub fn set_actor_owner(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_base: Object,
    no_crime: bool,
) {
    todo!()
}

/// `GetBaseObject`
///
/// ```text
/// Form Function GetBaseObject() native
/// ```
pub fn get_base_object(caller: Caller<'_, State>, self_ptr: Object) -> Object {
    todo!()
}
