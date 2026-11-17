use openfo4_core::record::RecordId;

/// Request that the scene of a quest is started.
#[derive(Copy, Clone, Debug)]
pub struct StartQuestScene {
    pub scene: RecordId,
}

/// Request that the scene of a quest is stopped.
#[derive(Copy, Clone, Debug)]
pub struct StopQuestScene {
    pub scene: RecordId,
}
