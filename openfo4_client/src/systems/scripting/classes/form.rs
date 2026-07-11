use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Exception};

use crate::systems::scripting::classes::EngineExt;

use super::{Bool, Int, Object, State};

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
fn get_form_id(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `GetGoldValue`
///
/// ```text
/// int Function GetGoldValue() native
/// ```
fn get_gold_value(caller: Caller<'_, State>, self_ptr: Object) -> Result<Int, Exception> {
    todo!()
}

/// `HasKeyword`
///
/// ```text
/// bool Function HasKeyword(Keyword akKeyword) native
/// ```
fn has_keyword(
    caller: Caller<'_, State>,
    self_ptr: Object,
    keyword: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `HasKeywordInFormList`
///
/// ```text
/// bool Function HasKeywordInFormList(FormList akKeywordList) native
/// ```
fn has_keyword_in_form_list(
    caller: Caller<'_, State>,
    self_ptr: Object,
    formlist: Object,
) -> Result<Bool, Exception> {
    todo!()
}

/// `PlayerKnows`
///
/// ```text
/// bool Function PlayerKnows() native
/// ```
fn player_knows(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Exception> {
    todo!()
}

/// `StartObjectProfiling`
///
/// ```text
/// Function StartObjectProfiling() native
/// ```
fn start_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Exception> {
    Ok(())
}

/// `StopObjectProfiling`
///
/// ```text
/// Function StopObjectProfiling() native
/// ```
fn stop_object_profiling(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<(), Exception> {
    Ok(())
}
