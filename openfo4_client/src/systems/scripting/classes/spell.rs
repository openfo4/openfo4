use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::state::State;

use super::{Bool, Object};

/// `Cast`
///
/// ```text
/// Function Cast(ObjectReference akSource, ObjectReference akTarget = None) native
/// ```
pub fn cast(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _source: Object,
    _target: Object,
) -> Result<(), Error> {
    todo!()
}

/// `IsHostile`
///
/// ```text
/// bool Function IsHostile() native
/// ```
pub fn is_hostile(_caller: Caller<'_, State>, _self_ptr: Object) -> Result<Bool, Error> {
    todo!()
}

/// `RemoteCast`
///
/// ```text
/// Function RemoteCast(ObjectReference akSource, Actor akBlameActor, ObjectReference akTarget = None) native
/// ```
pub fn remote_cast(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _source: Object,
    _blame_actor: Object,
    _target: Object,
) -> Result<(), Error> {
    todo!()
}
