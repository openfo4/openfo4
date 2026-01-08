use openfo4_script::runtime::{Caller, Exception};

use super::{Bool, Int, Object, State};

/// `GetFormID`
///
/// ```text
/// int Function GetFormID() native
/// ```
pub fn get_form_id(caller: Caller<'_, State>, self_ptr: isize) -> Result<Int, Exception> {
    todo!()
}

/// `GetGoldValue`
///
/// ```text
/// int Function GetGoldValue() native
/// ```
pub fn get_gold_value(caller: Caller<'_, State>, self_ptr: isize) -> Result<Int, Exception> {
    todo!()
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

/// `HasKeywordInFormList`
///
/// ```text
/// bool Function HasKeywordInFormList(FormList akKeywordList) native
/// ```
pub fn has_keyword_in_form_list(
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
pub fn player_knows(caller: Caller<'_, State>) -> Result<Bool, Exception> {
    todo!()
}
