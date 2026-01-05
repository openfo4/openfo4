use crate::classes::{Bool, Float, Int, Object, State};
use crate::runtime::{Caller, Exception};

/// `Activate`
///
/// ```text
/// bool Function Activate(ObjectReference akActivator, bool abDefaultProcessingOnly = false) native
/// ```
pub fn activate(
    caller: Caller<'_, State>,
    object: Object,
    activator: Object,
    default_processing_only: Bool,
) -> Result<Bool, Exception> {
    todo!()
}

/// `AddItem`
///
/// ```text
/// Function AddItem(Form akItemToAdd, int aiCount = 1, bool abSilent = false) native
/// ```
pub fn add_item(
    caller: Caller<'_, State>,
    object: Object,
    item: Object,
    count: Int,
    silent: Bool,
) -> Result<(), Exception> {
    todo!()
}

/// `DamageValue`
///
/// ```text
/// Function DamageValue(ActorValue akAV, float afDamage) native
/// ```
pub fn damage_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    damage: Float,
) -> Result<(), Exception> {
    todo!()
}

/// `Delete`
///
/// ```text
/// Function Delete() native
/// ```
pub fn delete(caller: Caller<'_, State>, object: Object) -> Result<(), Exception> {
    todo!()
}

/// `DeleteWhenAble`
///
/// Overwritten non-native function.
///
/// ```text
/// Function DeleteWhenAble()
/// ```
pub fn delete_when_able(caller: Caller<'_, State>, object: Object) -> Result<(), Exception> {
    todo!()
}

/// `Disable`
///
/// ```text
/// Function Disable(bool abFadeOut = False) native
/// ```
pub fn disable(caller: Caller<'_, State>, object: Object, fade: Bool) -> Result<(), Exception> {
    todo!()
}

/// `GetValue`
///
/// ```text
/// float Function GetValue(ActorValue akAV) native
/// ```
pub fn get_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
) -> Result<Float, Exception> {
    todo!()
}

/// `SetValue`
///
/// ```text
/// Function SetValue(ActorValue akAV, float afValue) native
/// ```
pub fn set_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    value: Float,
) -> Result<(), Exception> {
    todo!()
}

/// `GetBaseValue`
///
/// ```text
/// float Function GetBaseValue(ActorValue akAV) native
/// ```
pub fn get_base_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
) -> Result<Float, Exception> {
    dbg!(&self_ptr);
    todo!()
}

/// `GetBaseObject`
///
/// ```text
/// Form Function GetBaseObject() native
/// ```
pub fn get_base_object(caller: Caller<'_, State>, self_ptr: Object) -> Result<Object, Exception> {
    todo!()
}

/// `GetDistance`
///
/// ```text
/// float Function GetDistance(ObjectReference akOther) native
/// ```
pub fn get_distance(
    caller: Caller<'_, State>,
    self_ptr: Object,
    other: Object,
) -> Result<Float, Exception> {
    todo!()
}

/// `GetItemCount`
///
/// ```text
/// int Function GetItemCount(Form akItem) native
/// ```
pub fn get_item_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
) -> Result<Int, Exception> {
    todo!()
}

/// `GetLinkedRef`
///
/// ```text
/// ObjectReference Function GetLinkedRef(Keyword apKeyword = None) native
/// ```
pub fn get_linked_ref(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Object, Exception> {
    let linked_ref = caller.data().provider.linked_ref(self_ptr, keyword);
    Ok(linked_ref.unwrap_or(Object::NULL))
}

/// `HasKeyword`
///
/// ```text
/// bool Function HasKeyword(Keyword akKeyword) native
/// ```
pub fn has_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `IsOwnedBy`
///
/// ```text
/// bool Function IsOwnedBy(Actor akOwner) native
/// ```
pub fn is_owned_by(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor: Object,
) -> Result<Bool, Exception> {
    let owner = caller.data().provider.get_owner(self_ptr);
    Ok(owner == actor)
}

/// `RemoveItem`
///
/// ```text
/// Function RemoveItem(Form akItemToRemove, int aiCount = 1, bool abSilent = false, ObjectReference akOtherContainer = None) native
/// ```
pub fn remove_item(
    caller: Caller<'_, State>,
    self_ptr: Object,
    item: Object,
    count: Int,
    silent: Bool,
    other: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `RestoreValue`
///
/// ```text
/// Function RestoreActorValue(ActorValue akAV, float afAmount) native
/// ```
pub fn restore_value(
    caller: Caller<'_, State>,
    self_ptr: Object,
    actor_value: Object,
    amount: Float,
) -> Result<(), Exception> {
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
) -> Result<(), Exception> {
    todo!()
}
