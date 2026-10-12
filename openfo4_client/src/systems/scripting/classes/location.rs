use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::classes::Object;
use crate::systems::scripting::state::State;

#[rustfmt::skip]
pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Location", "AddLinkedLocation", add_linked_location);
    engine.register_if_exists("Location", "GetAllLinkedLocations", get_all_linked_locations);
    engine.register_if_exists("Location", "GetKeywordData", get_keyword_data);
    engine.register_if_exists("Location", "GetRefTypeAliveCount", get_ref_type_alive_count);
    engine.register_if_exists("Location", "GetRefTypeDeadCount", get_ref_type_dead_count);
    engine.register_if_exists("Location", "HasCommonParent", has_common_parent);
    engine.register_if_exists("Location", "HasEverBeenCleared", has_ever_been_cleared);
    engine.register_if_exists("Location", "HasRefType", has_ref_type);
    engine.register_if_exists("Location", "IsCleared", is_cleared);
    engine.register_if_exists("Location", "IsChild", is_child);
    engine.register_if_exists("Location", "IsLinkedLocation", is_linked_location);
    engine.register_if_exists("Location", "IsLoaded", is_loaded);
    engine.register_if_exists("Location", "RemoveLinkedLocation", remove_linked_location);
    engine.register_if_exists("Location", "Reset", reset);
    engine.register_if_exists("Location", "SetKeywordData", set_keyword_data);
    engine.register_if_exists("Location", "SetCleared", set_cleared);
}

/// `AddLinkedLocation`
///
/// ```text
/// Function AddLinkedLocation(Location akLoc, Keyword akKeyword) native
/// ```
fn add_linked_location(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_loc: Object,
    _ak_keyword: Object,
) -> Result<(), Error> {
    todo!()
}

/// `GetAllLinkedLocations`
///
/// ```text
/// Location[] Function GetAllLinkedLocations(Keyword akKeyword) native
/// ```
fn get_all_linked_locations(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_keyword: Object,
) -> Result<Object, Error> {
    todo!()
}

/// `GetKeywordData`
///
/// ```text
/// float Function GetKeywordData(Keyword akKeyword) native
/// ```
fn get_keyword_data(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_keyword: Object,
) -> Result<f32, Error> {
    todo!()
}

/// `GetRefTypeAliveCount`
///
/// ```text
/// int Function GetRefTypeAliveCount(LocationRefType akRefType) native
/// ```
fn get_ref_type_alive_count(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_ref_type: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `GetRefTypeDeadCount`
///
/// ```text
/// int Function GetRefTypeDeadCount(LocationRefType akRefType) native
/// ```
fn get_ref_type_dead_count(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_ref_type: Object,
) -> Result<i32, Error> {
    todo!()
}

/// `HasCommonParent`
///
/// ```text
/// bool Function HasCommonParent(Location akOther, Keyword akFilter) native
/// ```
fn has_common_parent(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
    _ak_filter: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `HasEverBeenCleared`
///
/// ```text
/// bool Function HasEverBeenCleared() native
/// ```
fn has_ever_been_cleared(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `HasRefType`
///
/// ```text
/// bool Function HasRefType(LocationRefType akRefType) native
/// ```
fn has_ref_type(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_ref_type: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsCleared`
///
/// ```text
/// bool Function IsCleared() native
/// ```
fn is_cleared(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `IsChild`
///
/// ```text
/// bool Function IsChild(Location akOther) native
/// ```
fn is_child(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_other: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsLinkedLocation`
///
/// ```text
/// bool Function IsLinkedLocation(Location akLocation, Keyword akKeyword) native
/// ```
fn is_linked_location(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_location: Object,
    _ak_keyword: Object,
) -> Result<bool, Error> {
    todo!()
}

/// `IsLoaded`
///
/// ```text
/// bool Function IsLoaded() native
/// ```
fn is_loaded(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<bool, Error> {
    todo!()
}

/// `RemoveLinkedLocation`
///
/// ```text
/// Function RemoveLinkedLocation(Location akLoc, Keyword akKeyword) native
/// ```
fn remove_linked_location(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_loc: Object,
    _ak_keyword: Object,
) -> Result<(), Error> {
    todo!()
}

/// `Reset`
///
/// ```text
/// Function Reset() native
/// ```
fn reset(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Error> {
    todo!()
}

/// `SetKeywordData`
///
/// ```text
/// Function SetKeywordData(Keyword akKeyword, float afData) native
/// ```
fn set_keyword_data(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ak_keyword: Object,
    _af_data: f32,
) -> Result<(), Error> {
    todo!()
}

/// `SetCleared`
///
/// ```text
/// Function SetCleared(bool abCleared) native
/// ```
fn set_cleared(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _ab_cleared: bool,
) -> Result<(), Error> {
    todo!()
}
