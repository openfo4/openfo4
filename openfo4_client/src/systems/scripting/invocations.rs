use std::borrow::Cow;

use openfo4_script::runtime::{ObjectAddr, ObjectRef};

use crate::systems::scripting::classes::TimerId;
use crate::systems::scripting::loader::ScriptObject;

#[derive(Clone, Debug)]
pub struct EventInvocation {
    /// The receiver of the script invocation.
    pub receiver: ScriptObject,
    // Event handler function name.
    pub name: Cow<'static, str>,
    pub args: Vec<Value>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(i32),
    ObjectRef(ObjectRef),
    ObjectAddr(ObjectAddr),
}

#[derive(Clone, Debug)]
pub struct QuestInit {
    pub quest: ScriptObject,
}

impl From<QuestInit> for EventInvocation {
    fn from(value: QuestInit) -> Self {
        Self {
            receiver: value.quest,
            name: Cow::Borrowed("OnQuestInit"),
            args: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct QuestSetStage {
    pub quest: ScriptObject,
    pub stage: u16,
    pub item: u16,
}

impl From<QuestSetStage> for EventInvocation {
    fn from(value: QuestSetStage) -> Self {
        Self {
            receiver: value.quest,
            name: Cow::Borrowed("OnStageSet"),
            args: vec![
                Value::Int(value.stage.into()),
                Value::Int(value.item.into()),
            ],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub object: ScriptObject,
    pub timer: TimerId,
}

impl From<Timer> for EventInvocation {
    fn from(value: Timer) -> Self {
        Self {
            receiver: value.object,
            name: Cow::Borrowed("OnTimer"),
            args: vec![Value::Int(value.timer.0)],
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimerGameTime {
    pub object: ScriptObject,
    pub timer: TimerId,
}

impl From<TimerGameTime> for EventInvocation {
    fn from(value: TimerGameTime) -> Self {
        Self {
            receiver: value.object,
            name: Cow::Borrowed("OnTimerGameTime"),
            args: vec![Value::Int(value.timer.0)],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Init {
    pub object: ScriptObject,
}

impl From<Init> for EventInvocation {
    fn from(value: Init) -> Self {
        Self {
            receiver: value.object,
            name: Cow::Borrowed("OnInit"),
            args: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ItemAdded {
    pub object: ScriptObject,
    pub count: i32,
    pub base_item: ScriptObject,
    pub item_reference: ScriptObject,
    pub source_container: ScriptObject,
}
