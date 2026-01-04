use crate::classes::{Bool, Int, Object, State};
use crate::runtime::{Caller, Exception};

/// `SendStoryEvent`
///
/// ```text
/// Function SendStoryEvent(Location akLoc = None, ObjectReference akRef1 = None, ObjectReference akRef2 = None, int aiValue1 = 0, int aiValue2 = 0) native
/// ```
pub fn send_story_event(
    caller: Caller<'_, State>,
    self_ptr: Object,
    location: Object,
    ref1: Object,
    ref2: Object,
    value1: Int,
    value2: Int,
) -> Result<(), Exception> {
    todo!()
}

/// `SendStoryEventAndWait`
///
/// ```text
/// bool Function SendStoryEventAndWait(Location akLoc = None, ObjectReference akRef1 = None, ObjectReference akRef2 = None, int aiValue1 = 0, int aiValue2 = 0) native
/// ```
pub fn send_story_event_and_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    location: Object,
    ref1: Object,
    ref2: Object,
    value1: Int,
    value2: Int,
) -> Result<Bool, Exception> {
    todo!()
}
