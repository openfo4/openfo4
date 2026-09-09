use openfo4_script::runtime::{Caller, Error};

use crate::systems::scripting::state::State;

use super::{Bool, Float, Int, Object};

/// `SetInstanceVolume`
///
/// ```text
/// Function SetInstanceVolume(int aiPlaybackInstance, float afVolume) native global
/// ```
pub fn set_instance_volume(
    caller: Caller<'_, State>,
    instance: Int,
    volume: Float,
) -> Result<(), Error> {
    todo!()
}

/// `StopInstance`
///
/// ```text
/// Function StopInstance(int aiPlaybackInstance) native global
/// ```
pub fn stop_instance(caller: Caller<'_, State>, instance: Int) -> Result<(), Error> {
    todo!()
}

/// `Play`
///
/// ```text
/// int Function Play(ObjectReference akSource) native
/// ```
pub fn play(caller: Caller<'_, State>, self_ptr: Object, source: Object) -> Result<Int, Error> {
    todo!();
}

/// `PlayAndWait`
///
/// ```text
/// bool Function PlayAndWait(ObjectReference akSource) native
/// ```
pub fn play_and_wait(
    caller: Caller<'_, State>,
    self_ptr: Object,
    source: Object,
) -> Result<Bool, Error> {
    todo!()
}
