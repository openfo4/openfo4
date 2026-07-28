use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

use crossbeam::queue::SegQueue;
use futures::stream::FuturesUnordered;
use futures_lite::StreamExt;
use game_tasks::TaskPool;
use game_tracing::trace_span;
use openfo4_script::runtime::{Context, Val};
use slab::Slab;
use tracing::instrument;

use crate::systems::scripting::classes::{BorrowedState, SharedBorrowCell, State};
use crate::systems::scripting::events::queue::{EventQueue, EventQueueProducer};
use crate::systems::scripting::invocations::{EventInvocation, Value};
use crate::systems::scripting::loader::{ScriptLoader, ScriptObjectId};
use crate::systems::scripting::objects::ScriptObjects;
use crate::systems::scripting::registration::{EventSender, RegistrationSet};

pub struct RunnerHandle {
    pub events: EventQueueProducer,
    pub cell: Arc<SharedBorrowCell<BorrowedState>>,
    tick: Arc<event_listener::Event>,
    queue: Arc<SegQueue<(EventInvocation, Arc<AtomicBool>)>>,
}

impl RunnerHandle {
    pub fn tick(&self) {
        self.tick.notify(1);
    }

    pub fn push_invocation(&mut self, event: EventInvocation) -> Invocation {
        let is_done = Arc::new(AtomicBool::new(false));
        self.queue.push((event, is_done.clone()));
        Invocation { is_done }
    }
}

pub struct Runner {
    events: EventQueue,
    objects: Arc<ScriptObjects>,
    registration: Arc<RegistrationSet>,
    pool: Arc<TaskPool>,
    loader: Arc<ScriptLoader>,
    tasks: FuturesUnordered<Task>,
    queue: VecDeque<ScheduledTask>,
    state: State,
    tick: Arc<event_listener::Event>,
    invocation_queue: Arc<SegQueue<(EventInvocation, Arc<AtomicBool>)>>,
    invocations: Slab<InvocationState>,
}

impl Runner {
    pub fn new(state: State, pool: Arc<TaskPool>) -> Self {
        Self {
            events: EventQueue::default(),
            objects: state.objects.clone(),
            registration: state.registration.clone(),
            pool,
            loader: state.loader.clone(),
            queue: VecDeque::new(),
            tasks: FuturesUnordered::new(),
            state,
            tick: Arc::default(),
            invocation_queue: Arc::default(),
            invocations: Slab::new(),
        }
    }

    pub fn handle(&self) -> RunnerHandle {
        RunnerHandle {
            events: self.events.producer(),
            cell: self.state.borrowed.clone(),
            tick: self.tick.clone(),
            queue: self.invocation_queue.clone(),
        }
    }

    #[instrument(name = "Runner::run", skip_all)]
    pub async fn run(&mut self) {
        loop {
            // FIXME: Ideally we would run fully asynchronously from the "main"
            // threada and not need to sync with it to make progress.
            self.tick.listen().await;

            self.pump_events();
            self.launch_tasks();

            let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
            while let Poll::Ready(Some(invocation)) = self.tasks.poll_next(&mut cx) {
                if let Some(invocation) = invocation {
                    let state = self.invocations.get_mut(invocation.0).unwrap();
                    state.num_running -= 1;
                    if state.num_running == 0 {
                        state.is_done.store(true, Ordering::Release);
                        self.invocations.remove(invocation.0);
                    }
                }
            }

            if let Some(state) = self.state.borrowed.try_borrow() {
                let world = unsafe { &mut *state.world };
                let events = unsafe { &mut *state.events };

                let ready = self.state.effects.lock().run(
                    &self.pool,
                    world,
                    events,
                    &self.loader.esp_loader,
                    &self.loader,
                );

                let mut wait = self.state.wait_for_effects.lock();
                for id in ready {
                    let event = wait.remove(&id).unwrap();
                    event.notify(event.total_listeners());
                }
            }
        }
    }

    fn pump_events(&mut self) {
        let _span = trace_span!("Runner::pump_events").entered();

        while let Some((event, is_done)) = self.invocation_queue.pop() {
            let scripts = self.objects.get_by_id(event.receiver.id()).unwrap();

            let entry = self.invocations.vacant_entry();
            let key = InvocationId(entry.key());

            let mut num_running = 0;
            for script in scripts.iter() {
                self.queue.push_back(ScheduledTask {
                    object: event.receiver.id(),
                    invocation_id: Some(key),
                    class: script.name().to_owned(),
                    func: event.name.to_string(),
                    args: event.args.clone(),
                });
                num_running += 1;
            }

            if num_running != 0 {
                entry.insert(InvocationState {
                    num_running,
                    is_done,
                });
            }
        }

        let dispatches = self.events.process(&self.objects, &self.pool, &self.loader);

        for dispatch in dispatches {
            tracing::debug!(
                "dispatching event {:?} for {:?}",
                dispatch.kind,
                dispatch.receiver,
            );

            // Dispatch one event for every script object.
            let scripts = self.objects.get_by_id(dispatch.receiver).unwrap();
            for script in scripts.iter() {
                self.registration.for_each(
                    &dispatch.kind,
                    &EventSender::Object(script.get().object.addr()),
                    |receiver, event| {
                        let receiver = self.objects.get_by_addr(receiver).unwrap();

                        if receiver.get().object.addr() == script.get().object.addr() {
                            self.queue.push_back(ScheduledTask {
                                object: receiver.id(),
                                class: receiver.name().to_owned(),
                                invocation_id: None,
                                func: event.kind.func_name().to_owned(),
                                args: dispatch.args.to_vec(),
                            });
                        } else {
                            let mut args = dispatch.args.to_vec();
                            args.insert(0, Value::ObjectRef(script.get().object.clone()));

                            self.queue.push_back(ScheduledTask {
                                object: receiver.id(),
                                class: receiver.name().to_owned(),
                                invocation_id: None,
                                func: event.kind.remote_func_name().to_owned(),
                                args,
                            });
                        }
                    },
                );

                for alias in self
                    .state
                    .aliases
                    .get_alises_for_object(script.get().object.addr())
                {
                    let alias = self.objects.get_by_addr(alias).unwrap();

                    let mut args = dispatch.args.to_vec();
                    args.insert(0, Value::ObjectRef(script.get().object.clone()));

                    self.queue.push_back(ScheduledTask {
                        object: alias.id(),
                        class: alias.name().to_owned(),
                        invocation_id: None,
                        func: dispatch.kind.remote_ref_alias_func_name().to_owned(),
                        args,
                    });
                }
            }
        }
    }

    fn launch_tasks(&mut self) {
        let _span = trace_span!("Runner::launch_tasks").entered();

        while let Some(task) = self.queue.pop_front() {
            let object = self.objects.get_by_id(task.object).unwrap();
            let script = object.get(&task.class).unwrap();

            let instance = script.get().instance.clone();
            if instance.get_func(&task.func).is_none() {
                continue;
            }

            let mut stack = self.loader.engine.create_stack();

            let mut params = task
                .args
                .into_iter()
                .map(|value| match value {
                    Value::Bool(v) => Val::Bool(v),
                    Value::Int(v) => Val::I32(v),
                    Value::ObjectRef(object) => Val::Object(object.addr()),
                    Value::ObjectAddr(addr) => Val::Object(addr),
                })
                .collect::<Vec<_>>();
            params.insert(0, Val::Object(script.get().object.addr()));

            tracing::debug!(
                "calling {}.{} ({:?})",
                task.class,
                task.func,
                script.get().object.addr()
            );

            let engine = Arc::clone(&self.loader.engine);
            let state = self.state.clone();

            let future = Box::pin(async move {
                let mut ctx = Context::new(&engine, &mut stack, state);

                let func = instance.get_func(&task.func).unwrap();

                if let Err(err) = func.call_async(&mut ctx, &params).await {
                    tracing::error!("script returned with error: {:?}", err);
                }

                task.invocation_id
            });

            self.tasks.push(future);
        }
    }
}

#[derive(Clone, Debug)]
struct ScheduledTask {
    // id: TaskId,
    object: ScriptObjectId,
    invocation_id: Option<InvocationId>,
    class: String,
    func: String,
    args: Vec<Value>,
}

type Task = Pin<Box<dyn Future<Output = Option<InvocationId>> + Send + Sync + 'static>>;

#[derive(Clone, Debug)]
struct InvocationState {
    /// The number of script functions that are still being executed.
    num_running: usize,
    is_done: Arc<AtomicBool>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct InvocationId(usize);

#[derive(Debug)]
pub struct Invocation {
    is_done: Arc<AtomicBool>,
}

impl Invocation {
    pub fn is_done(&self) -> bool {
        self.is_done.load(Ordering::Acquire)
    }
}
