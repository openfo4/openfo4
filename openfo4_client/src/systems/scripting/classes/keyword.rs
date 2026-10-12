use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::state::State;

use super::{Bool, Int, Object};

/// `SendStoryEvent`
///
/// ```text
/// Function SendStoryEvent(Location akLoc = None, ObjectReference akRef1 = None, ObjectReference akRef2 = None, int aiValue1 = 0, int aiValue2 = 0) native
/// ```
pub fn send_story_event(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _location: Object,
    _ref1: Object,
    _ref2: Object,
    _value1: Int,
    _value2: Int,
) -> Result<(), Error> {
    todo!()
}

/// `SendStoryEventAndWait`
///
/// ```text
/// bool Function SendStoryEventAndWait(Location akLoc = None, ObjectReference akRef1 = None, ObjectReference akRef2 = None, int aiValue1 = 0, int aiValue2 = 0) native
/// ```
pub fn send_story_event_and_wait(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _location: Object,
    _ref1: Object,
    _ref2: Object,
    _value1: Int,
    _value2: Int,
) -> Result<Bool, Error> {
    todo!()
}
