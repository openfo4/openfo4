use openfo4_script::{
    engine::Engine,
    runtime::{Caller, Exception},
};

use crate::systems::scripting::classes::{Object, State};

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_native_function("Location", "AddLinkedLocation", add_linked_location);
    engine.register_native_function("Location", "GetAllLinkedLocations", get_all_linked_locations);
    engine.register_native_function("Location", "GetKeywordData", get_keyword_data);
    engine.register_native_function("Location", "GetRefTypeAliveCount", get_ref_type_alive_count);
    engine.register_native_function("Location", "GetRefTypeDeadCount", get_ref_type_dead_count);
    engine.register_native_function("Location", "HasCommonParent", has_common_parent);
    engine.register_native_function("Location", "HasEverBeenCleared", has_ever_been_cleared);
    engine.register_native_function("Location", "HasRefType", has_ref_type);
    engine.register_native_function("Location", "IsCleared", is_cleared);
    engine.register_native_function("Location", "IsChild", is_child);
    engine.register_native_function("Location", "IsLinkedLocation", is_linked_location);
    engine.register_native_function("Location", "IsLoaded", is_loaded);
    engine.register_native_function("Location", "RemoveLinkedLocation", remove_linked_location);
    engine.register_native_function("Location", "Reset", reset);
    engine.register_native_function("Location", "SetKeywordData", set_keyword_data);
    engine.register_native_function("Location", "SetCleared", set_cleared);
}

/// `AddLinkedLocation`
///
/// ```text
/// Function AddLinkedLocation(Location akLoc, Keyword akKeyword) native
/// ```
fn add_linked_location(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_loc: Object,
    ak_keyword: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `GetAllLinkedLocations`
///
/// ```text
/// Location[] Function GetAllLinkedLocations(Keyword akKeyword) native
/// ```
fn get_all_linked_locations(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword: Object,
) -> Result<isize, Exception> {
    todo!()
}

/// `GetKeywordData`
///
/// ```text
/// float Function GetKeywordData(Keyword akKeyword) native
/// ```
fn get_keyword_data(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword: Object,
) -> Result<f32, Exception> {
    todo!()
}

/// `GetRefTypeAliveCount`
///
/// ```text
/// int Function GetRefTypeAliveCount(LocationRefType akRefType) native
/// ```
fn get_ref_type_alive_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ref_type: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `GetRefTypeDeadCount`
///
/// ```text
/// int Function GetRefTypeDeadCount(LocationRefType akRefType) native
/// ```
fn get_ref_type_dead_count(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ref_type: Object,
) -> Result<i32, Exception> {
    todo!()
}

/// `HasCommonParent`
///
/// ```text
/// bool Function HasCommonParent(Location akOther, Keyword akFilter) native
/// ```
fn has_common_parent(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
    ak_filter: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `HasEverBeenCleared`
///
/// ```text
/// bool Function HasEverBeenCleared() native
/// ```
fn has_ever_been_cleared(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `HasRefType`
///
/// ```text
/// bool Function HasRefType(LocationRefType akRefType) native
/// ```
fn has_ref_type(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_ref_type: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsCleared`
///
/// ```text
/// bool Function IsCleared() native
/// ```
fn is_cleared(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `IsChild`
///
/// ```text
/// bool Function IsChild(Location akOther) native
/// ```
fn is_child(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_other: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsLinkedLocation`
///
/// ```text
/// bool Function IsLinkedLocation(Location akLocation, Keyword akKeyword) native
/// ```
fn is_linked_location(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_location: Object,
    ak_keyword: Object,
) -> Result<bool, Exception> {
    todo!()
}

/// `IsLoaded`
///
/// ```text
/// bool Function IsLoaded() native
/// ```
fn is_loaded(caller: Caller<'_, State>, self_ptr: Object) -> Result<bool, Exception> {
    todo!()
}

/// `RemoveLinkedLocation`
///
/// ```text
/// Function RemoveLinkedLocation(Location akLoc, Keyword akKeyword) native
/// ```
fn remove_linked_location(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_loc: Object,
    ak_keyword: Object,
) -> Result<(), Exception> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(caller: Caller<'_, State>, self_ptr: Object) -> Result<(), Exception> {
    todo!()
}

/// `SetKeywordData`
///
/// ```text
/// Function SetKeywordData(Keyword akKeyword, float afData) native
/// ```
fn set_keyword_data(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ak_keyword: Object,
    af_data: f32,
) -> Result<(), Exception> {
    todo!()
}

/// `SetCleared`
///
/// ```text
/// Function SetCleared(bool abCleared) native
/// ```
fn set_cleared(
    caller: Caller<'_, State>,
    self_ptr: Object,
    ab_cleared: bool,
) -> Result<(), Exception> {
    todo!()
}
