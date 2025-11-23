use crate::classes::{Bool, Int, Object, State};
use crate::runtime::Caller;

/// `GetFormID`
///
/// ```text
/// int Function GetFormID() native
/// ```
pub fn get_form_id(caller: Caller<'_, State>, self_ptr: isize) -> Int {
    todo!()
}

/// `GetGoldValue`
///
/// ```text
/// int Function GetGoldValue() native
/// ```
pub fn get_gold_value(caller: Caller<'_, State>, self_ptr: isize) -> Int {
    todo!()
}

/// `HasKeyword`
///
/// ```text
/// bool Function HasKeyword(Keyword akKeyword) native
/// ```
pub fn has_keyword(caller: Caller<'_, State>, self_ptr: Object, keyword: Object) -> Bool {
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
) -> Bool {
    todo!()
}

/// `PlayerKnows`
///
/// ```text
/// bool Function PlayerKnows() native
/// ```
pub fn player_knows(caller: Caller<'_, State>) -> Bool {
    todo!()
}
