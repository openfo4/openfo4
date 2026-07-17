use openfo4_script::runtime::{Caller, Error};

use super::{Bool, Object, State};

/// `Cast`
///
/// ```text
/// Function Cast(ObjectReference akSource, ObjectReference akTarget = None) native
/// ```
pub fn cast(
    caller: Caller<'_, State>,
    self_ptr: Object,
    source: Object,
    target: Object,
) -> Result<(), Error> {
    todo!()
}

/// `IsHostile`
///
/// ```text
/// bool Function IsHostile() native
/// ```
pub fn is_hostile(caller: Caller<'_, State>, self_ptr: Object) -> Result<Bool, Error> {
    todo!()
}

/// `RemoteCast`
///
/// ```text
/// Function RemoteCast(ObjectReference akSource, Actor akBlameActor, ObjectReference akTarget = None) native
/// ```
pub fn remote_cast(
    caller: Caller<'_, State>,
    self_ptr: Object,
    source: Object,
    blame_actor: Object,
    target: Object,
) -> Result<(), Error> {
    todo!()
}
