use crate::classes::{Int, State};
use crate::runtime::{Caller, Exception, ObjectAddr};

/// `GetAggressionAV`
///
/// ```text
/// ActorValue Function GetAggressionAV() native global
/// ```
pub fn get_agression_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.aggression.addr())
}

/// `GetAgilityAV`
///
/// ```text
/// ActorValue Function GetAgilityAV() native global
/// ```
pub fn get_agility_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.agility.addr())
}

/// `GetCharismaAV`
///
/// ```text
/// ActorValue Function GetCharismaAV() native global
/// ```
pub fn get_charisma_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.charisma.addr())
}

/// `GetConfidenceAV`
///
/// ```text
/// ActorValue Function GetConfidenceAV() native global
/// ```
pub fn get_confidence_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.confidence.addr())
}

/// `GetDifficulty`
///
/// ```text
/// int Function GetDifficulty() native global
/// ```
pub fn get_difficulty(caller: Caller<'_, State>) -> Result<Int, Exception> {
    todo!()
}

/// `GetEnduranceAV`
///
/// ```text
/// ActorValue Function GetEnduranceAV() native global
/// ```
pub fn get_endurance_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.endurance.addr())
}

/// `GetForm`
///
/// ```text
/// Form Function GetForm(int aiFormID) native global
/// ```
pub fn get_form(caller: Caller<'_, State>, form_id: Int) -> Result<ObjectAddr, Exception> {
    todo!()
}

/// `GetFormFromFile`
///
/// ```text
/// Form Function GetFormFromFile(int aiFormID, string asFilename) native global
/// ```
pub fn get_form_from_file(
    caller: Caller<'_, State>,
    form_id: Int,
    filename: isize,
) -> Result<ObjectAddr, Exception> {
    todo!()
}

/// `GetHealthAV`
///
/// ```text
/// ActorValue Function GetHealthAV() native global
/// ```
pub fn get_health_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.health.addr())
}

/// `GetIntelligenceAV`
///
/// ```text
/// ActorValue Function GetIntelligenceAV() native global
/// ```
pub fn get_intelligence_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.intelligence.addr())
}

/// `GetLuckAV`
///
/// ```text
/// ActorValue Function GetLuckAV() native global
/// ```
pub fn get_luck_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.luck.addr())
}

/// `GetPerceptionAV`
///
/// ```text
/// ActorValue Function GetPerceptionAV() native global
/// ```
pub fn get_perception_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.luck.addr())
}

/// `GetPlayer`
///
/// ```text
/// Actor Function GetPlayer() native global
/// ```
pub fn get_player(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().player.addr())
}

/// `GetSuspiciousAV`
///
/// ```text
/// ActorValue Function GetSuspiciousAV() native global
/// ```
pub fn get_suspicious_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.suspicious.addr())
}

/// `GetStrengthAV`
///
/// ```text
/// ActorValue Function GetStrengthAV() native global
/// ```
pub fn get_strength_av(caller: Caller<'_, State>) -> Result<ObjectAddr, Exception> {
    Ok(caller.data().actor_values.strength.addr())
}
