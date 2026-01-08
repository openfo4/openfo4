use openfo4_script::runtime::{Caller, Exception};

use super::{Bool, Float, Int, Object, State};

/// `SetInstanceVolume`
///
/// ```text
/// Function SetInstanceVolume(int aiPlaybackInstance, float afVolume) native global
/// ```
pub fn set_instance_volume(
    caller: Caller<'_, State>,
    instance: Int,
    volume: Float,
) -> Result<(), Exception> {
    todo!()
}

/// `StopInstance`
///
/// ```text
/// Function StopInstance(int aiPlaybackInstance) native global
/// ```
pub fn stop_instance(caller: Caller<'_, State>, instance: Int) -> Result<(), Exception> {
    todo!()
}

/// `Play`
///
/// ```text
/// int Function Play(ObjectReference akSource) native
/// ```
pub fn play(caller: Caller<'_, State>, self_ptr: Object, source: Object) -> Result<Int, Exception> {
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
) -> Result<Bool, Exception> {
    todo!()
}
