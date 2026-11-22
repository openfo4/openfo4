use std::borrow::Cow;

use openfo4_script::runtime::object::ObjectHandle;

use crate::systems::scripting::loader::ScriptObject;
use crate::systems::scripting::state::TimerId;

#[derive(Clone, Debug)]
pub struct EventInvocation {
    /// The receiver of the script invocation.
    pub receiver: ScriptObject,
    // Event handler function name.
    pub name: Cow<'static, str>,
    pub args: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f32),
    Object(ObjectHandle),
    Null,
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&ObjectHandle> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<ObjectHandle> for Value {
    fn from(value: ObjectHandle) -> Self {
        Self::Object(value)
    }
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
pub struct QuestFragmentSetStage {
    pub quest: ScriptObject,
    pub func_name: String,
}

impl From<QuestFragmentSetStage> for EventInvocation {
    fn from(value: QuestFragmentSetStage) -> Self {
        Self {
            receiver: value.quest,
            name: Cow::Owned(value.func_name),
            args: Vec::new(),
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

#[derive(Clone, Debug)]
pub struct AliasInit {
    pub object: ScriptObject,
}

impl From<AliasInit> for EventInvocation {
    fn from(value: AliasInit) -> Self {
        Self {
            receiver: value.object,
            name: Cow::Borrowed("OnAliasInit"),
            args: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AliasShutdown {
    pub object: ScriptObject,
}

impl From<AliasShutdown> for EventInvocation {
    fn from(value: AliasShutdown) -> Self {
        Self {
            receiver: value.object,
            name: Cow::Borrowed("OnAliasShutdown"),
            args: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct QuestShutdown {
    pub object: ScriptObject,
}

impl From<QuestShutdown> for EventInvocation {
    fn from(value: QuestShutdown) -> Self {
        Self {
            receiver: value.object,
            name: Cow::Borrowed("OnQuestShutdown"),
            args: Vec::new(),
        }
    }
}
