use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::state::State;

use super::{Bool, Float, Int, Object};

/// `SetInstanceVolume`
///
/// ```text
/// Function SetInstanceVolume(int aiPlaybackInstance, float afVolume) native global
/// ```
pub fn set_instance_volume(
    _caller: Caller<'_, State>,
    _instance: Int,
    _volume: Float,
) -> Result<(), Error> {
    todo!()
}

/// `StopInstance`
///
/// ```text
/// Function StopInstance(int aiPlaybackInstance) native global
/// ```
pub fn stop_instance(_caller: Caller<'_, State>, _instance: Int) -> Result<(), Error> {
    todo!()
}

/// `Play`
///
/// ```text
/// int Function Play(ObjectReference akSource) native
/// ```
pub fn play(_caller: Caller<'_, State>, _self_ptr: Object, _source: Object) -> Result<Int, Error> {
    todo!();
}

/// `PlayAndWait`
///
/// ```text
/// bool Function PlayAndWait(ObjectReference akSource) native
/// ```
pub fn play_and_wait(
    _caller: Caller<'_, State>,
    _self_ptr: Object,
    _source: Object,
) -> Result<Bool, Error> {
    todo!()
}
