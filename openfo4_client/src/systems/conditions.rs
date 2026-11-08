mod compiler;

use std::borrow::Cow;
use std::sync::Arc;
use std::task::{Poll, Waker};

use async_lock::Mutex;
use futures::FutureExt;
use hashbrown::HashMap;
use openfo4_core::ecs::World;
use openfo4_core::events::EventBus;
use openfo4_core::record::RecordId;
use openfo4_esp::conditions::Condition;
use openfo4_pex::{Class, Module, Property, State, Value, Variable, VariableDefinition};
use tracing::instrument;

use crate::state::esp::EspLoader;
use crate::systems::conditions::compiler::build_func;
use crate::systems::quests::QuestManager;
use crate::systems::scripting::invocations::EventInvocation;
use crate::systems::scripting::loader::{ScriptLoader, ScriptObject, ScriptObjectId};
use crate::systems::scripting::{Executor, invocations};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct ConditionId {
    record: RecordId,
    index: usize,
}

/// Compiler for conditions that are attached on ESP records.
#[derive(Debug)]
pub struct ConditionCompiler {
    esp_loader: Arc<EspLoader>,
    loader: Arc<ScriptLoader>,
    conditions: Mutex<HashMap<ConditionId, CompiledCondition>>,
}

impl ConditionCompiler {
    pub fn new(esp_loader: Arc<EspLoader>, loader: Arc<ScriptLoader>) -> Self {
        Self {
            esp_loader,
            loader,
            conditions: Mutex::new(HashMap::new()),
        }
    }

    /// Compile the given set of conditions.
    ///
    /// The [`RecordId`] is the record from which the [`Condition`]s have been loaded from.
    /// The `index` value is an arbitrary value that can be used to distinguish multiple condition
    /// sets on a single record.
    #[instrument(name = "ConditionCompiler::compile", skip_all)]
    pub async fn compile(
        &self,
        record: RecordId,
        index: usize,
        conditions: &[Condition],
    ) -> CompiledCondition {
        let id = ConditionId { record, index };

        // Most condition expressions found on records are completely
        // empty. We should avoid creating a runtime function for them.
        if conditions.is_empty() {
            return CompiledCondition {
                inner: Inner::Empty,
            };
        }

        let mut conditions_set = self.conditions.lock().await;

        if let Some(object) = conditions_set.get(&id) {
            return object.clone();
        }

        let res = build_func(conditions);

        let name = format!("Condition_{}", record);
        let mut class = Class {
            name: name.clone(),
            parent: "".to_owned(),
            default_state: "".to_owned(),
            properties: Vec::new(),
            states: vec![State {
                name: "".to_owned(),
                functions: vec![res.func],
            }],
            structs: Vec::new(),
            variables: Vec::new(),
        };

        let mut global_values = Vec::new();
        for (index, global) in res.globals.into_iter().enumerate() {
            class.variables.push(Variable {
                definition: VariableDefinition {
                    name: global.ident.clone(),
                    ty: global.ty.to_owned(),
                },
                is_const: true,
                value: Value::Null,
            });

            let id = self.esp_loader.resolve_form(global.id, record).unwrap();

            // FIXME: Ideally we would remove the specific conditions that depend
            // on the value that failed to load and replace them with a `false`.
            let Some(res) = self.loader.load(id).await else {
                tracing::warn!("record {id} failed to load for condition used in {record}");
                continue;
            };

            let scripts = self.loader.objects().get_by_id(res.object.id()).unwrap();
            let script = scripts.get(global.ty).unwrap();

            let prop = format!("p{index}");
            class.properties.push(Property {
                name: prop.clone(),
                getter: None,
                setter: None,
                ty: global.ty.to_owned(),
                variable: Some(global.ident),
            });

            global_values.push((prop, script.get().object.clone()));
        }

        let module = Module {
            classes: vec![class],
        };

        let engine = self.loader.engine();
        engine.register(&module).await;
        let object = self.loader.instantiate_default(&name).await;

        let scripts = self.loader.objects().get_by_id(object.id()).unwrap();
        let script = scripts.get(&name).unwrap();

        for (prop, value) in global_values {
            script.get().object.set(&name, &prop, value.addr()).unwrap();
        }

        let object = CompiledCondition {
            inner: Inner::Object(object),
        };

        conditions_set.insert(ConditionId { record, index }, object.clone());

        object
    }
}

#[derive(Clone, Debug)]
pub struct CompiledCondition {
    inner: Inner,
}

#[derive(Clone, Debug)]
enum Inner {
    Empty,
    Object(ScriptObject),
}

impl CompiledCondition {
    /// Evaluate the condition.
    pub fn eval(
        &self,
        executor: &mut Executor,
        world: &mut World,
        events: &mut EventBus,
        quests: &mut QuestManager,
        subject: ScriptObjectId,
        target: Option<ScriptObjectId>,
    ) -> bool {
        let receiver = match &self.inner {
            Inner::Empty => return true,
            Inner::Object(obj) => obj,
        };

        let Some(subject) = executor
            .objects()
            .get_by_id(subject)
            .and_then(|s| s.get("ObjectReference"))
        else {
            return false;
        };

        let subject_obj = invocations::Value::Object(subject.get().object.clone());
        let mut target_obj = invocations::Value::Null;

        if let Some(target) = target {
            let Some(target) = executor
                .objects()
                .get_by_id(target)
                .and_then(|s| s.get("ObjectReference"))
            else {
                return false;
            };

            target_obj = invocations::Value::Object(target.get().object.clone());
        }

        let mut invocation = executor.invoke(EventInvocation {
            receiver: receiver.clone(),
            args: vec![subject_obj, target_obj],
            name: Cow::Borrowed("eval"),
        });

        let waker = Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        let returns = loop {
            match invocation.poll_unpin(&mut cx) {
                Poll::Ready(returns) => break returns,
                Poll::Pending => {
                    executor.update(world, events, quests);
                }
            }
        };

        returns
            .as_ref()
            .and_then(|vals| vals.first())
            .and_then(|val| val.as_bool())
            .unwrap_or(false)
    }
}
