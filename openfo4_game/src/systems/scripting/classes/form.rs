use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::EngineExt;
use crate::systems::scripting::state::State;

use super::{Bool, Object};

pub fn register(engine: &mut Engine) {
    engine.register_if_exists("Form", "GetFormID", get_form_id);
    engine.register_if_exists("Form", "GetGoldValue", get_gold_value);
    engine.register_if_exists("Form", "HasKeyword", has_keyword);
    engine.register_if_exists("Form", "HasKeywordInFormList", has_keyword_in_form_list);
    engine.register_if_exists("Form", "PlayerKnows", player_knows);
    engine.register_if_exists("Form", "StartObjectProfiling", start_object_profiling);
    engine.register_if_exists("Form", "StopObjectProfiling", stop_object_profiling);
}

/// `GetFormID`
///
/// ```text
/// int Function GetFormID() native
/// ```
fn get_form_id(caller: Caller<'_, State>, self_ptr: Object) -> i32 {
    let scripts = caller.data().objects.get_by_addr(self_ptr).unwrap();
    // Objects that derive from `Form` always have a `RecordId`.
    let id = scripts.record().unwrap();

    id.form.form().cast_signed()
}

/// `GetGoldValue`
///
/// ```text
/// int Function GetGoldValue() native
/// ```
fn get_gold_value(caller: Caller<'_, State>, self_ptr: Object) -> i32 {
    let scripts = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let form = scripts.data().form().unwrap();
    form.value
}

/// `HasKeyword`
///
/// ```text
/// bool Function HasKeyword(Keyword akKeyword) native
/// ```
fn has_keyword(caller: Caller<'_, State>, self_ptr: Object, keyword: Object) -> bool {
    if keyword.is_null() {
        return false;
    }

    let form = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let form = form.data().form().unwrap();
    let keyword = caller
        .data()
        .objects
        .get_by_addr(keyword)
        .and_then(|s| s.record())
        .unwrap();

    form.keywords.contains(&keyword)
}

/// `HasKeywordInFormList`
///
/// ```text
/// bool Function HasKeywordInFormList(FormList akKeywordList) native
/// ```
fn has_keyword_in_form_list(caller: Caller<'_, State>, self_ptr: Object, formlist: Object) -> bool {
    if formlist.is_null() {
        return false;
    }

    let form = caller.data().objects.get_by_addr(self_ptr).unwrap();
    let form = form.data().form().unwrap();
    let list = caller
        .data()
        .objects
        .get_by_addr(formlist)
        .and_then(|s| s.record())
        .unwrap();
    let lists = caller.data().globals.form_lists.lock();
    let list = lists.get(&list).unwrap();

    list.iter().any(|keyword| form.keywords.contains(&keyword))
}

/// `PlayerKnows`
///
/// ```text
/// bool Function PlayerKnows() native
/// ```
fn player_knows(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Bool, Error> {
    todo!()
}

/// `StartObjectProfiling`
///
/// ```text
/// Function StartObjectProfiling() native
/// ```
fn start_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) {}

/// `StopObjectProfiling`
///
/// ```text
/// Function StopObjectProfiling() native
/// ```
fn stop_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) {}
