use std::borrow::Borrow;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, LazyLock};
use std::task::Poll;

use futures::FutureExt;
use futures_lite::future::block_on;
use hashbrown::HashMap;
use openfo4_audio::AudioManager;
use openfo4_audio::backend::{DefaultBackend, StubBackend};
use openfo4_core::path::DataPathBuf;
use openfo4_core::record::{ModuleId, RecordId};
use openfo4_core_pipeline::entities::Entities;
use openfo4_esp::{FormId, Record, RecordData, RecordHeaderFlags};
use openfo4_game::config::Config;
use openfo4_game::loader::FileLoader;
use openfo4_game::shared::SharedState;
use openfo4_game::state::GameState;
use openfo4_game::state::esp::{EspLoader, EspLoaderBuilder};
use openfo4_game::systems::scripting::invocations::{EventInvocation, Value};
use openfo4_game::systems::scripting::loader::ScriptObject;
use openfo4_game::time::Time;
use openfo4_gizmos::Gizmos;
use openfo4_script::runtime::object::ObjectHandle;
use openfo4_tasks::TaskPool;
use pscc::Compiler;

pub struct TestRunnerBuilder {
    compiler: ScriptCompiler,
    esp: StaticEspLoaderBuilder,
}

impl TestRunnerBuilder {
    pub fn add_record<T>(&mut self, data: T) -> RecordId
    where
        T: Into<RecordData>,
    {
        self.esp.insert(data)
    }

    pub fn build(self) -> TestRunner {
        let entities = Entities::dummy();
        let shared = Arc::new(SharedState { entities });
        let pool = Arc::new(TaskPool::new(1));
        let gizmos = Gizmos::dummy();

        let file_loader = create_file_loader(self.compiler);
        let loader = self.esp.build();

        let audio = AudioManager::new(DefaultBackend::Stub(StubBackend));

        let game = block_on(GameState::new(
            shared,
            pool,
            file_loader,
            loader,
            Config::default(),
            audio,
        ));

        TestRunner {
            game,
            time: Time::new(),
            gizmos,
        }
    }
}

pub struct TestRunner {
    game: GameState,
    time: Time,
    gizmos: Gizmos,
}

impl TestRunner {
    pub fn builder() -> TestRunnerBuilder {
        TestRunnerBuilder {
            compiler: ScriptCompiler::new(),
            esp: StaticEspLoaderBuilder::default(),
        }
    }

    pub fn load(&mut self, id: RecordId) -> ScriptObject {
        block_on(self.game.executor.loader().load(id))
            .unwrap()
            .object
    }

    pub fn script(&mut self, object: &ScriptObject, class: &str) -> ObjectHandle {
        let scripts = self.game.executor.objects().get_by_id(object.id()).unwrap();
        scripts.get(class).unwrap().get().object.clone()
    }

    pub fn call<T, R>(&mut self, receiver: R, func: &str, args: T) -> Vec<Value>
    where
        T: Into<Vec<Value>>,
        R: Borrow<ScriptObject>,
    {
        let mut invocation = self.game.executor.invoke(EventInvocation {
            receiver: receiver.borrow().clone(),
            name: func.to_owned().into(),
            args: args.into(),
        });

        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        loop {
            match invocation.poll_unpin(&mut cx) {
                Poll::Pending => {
                    self.time.update();
                    self.game.update(&self.time, &self.gizmos);
                }
                Poll::Ready(res) => return res.unwrap(),
            }
        }
    }

    #[track_caller]
    pub fn call_assert<T, U, R>(&mut self, receiver: R, func: &str, args: T, expect: U)
    where
        T: Into<Vec<Value>>,
        U: Into<Value>,
        R: Borrow<ScriptObject>,
    {
        let res = self.call(receiver, func, args);
        assert_eq!(
            res,
            vec![expect.into()],
            "unexpected return value in function {func}"
        );
    }
}

fn create_file_loader(scripts: ScriptCompiler) -> Arc<FileLoader> {
    let mut loader = FileLoader::new::<_, Vec<PathBuf>>(vec![]);
    for (filename, module) in &scripts.modules {
        loader.add_memory(
            DataPathBuf::new(&*format!("scripts/{filename}.pex")),
            module.write(),
        );
    }

    Arc::new(loader)
}

const MODULE_ID: ModuleId = ModuleId(NonZeroU32::MIN);

#[derive(Clone, Debug, Default)]
struct StaticEspLoaderBuilder {
    records: HashMap<RecordId, Record>,
    next_id: u32,
}

impl StaticEspLoaderBuilder {
    pub fn insert<T>(&mut self, data: T) -> RecordId
    where
        T: Into<RecordData>,
    {
        let form = FormId(self.next_id + 1);
        self.next_id += 1;

        self.records.insert(
            RecordId::new(MODULE_ID, form),
            Record {
                id: form,
                flags: RecordHeaderFlags::empty(),
                data: data.into(),
            },
        );

        RecordId::new(MODULE_ID, form)
    }

    pub fn build(self) -> Arc<EspLoader> {
        let mut builder = EspLoaderBuilder::new();
        builder.set_hardcoded(self.records);
        Arc::new(builder.finalize())
    }
}

fn workspace_dir() -> &'static Path {
    static DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        let path = Command::new(env!("CARGO"))
            .arg("locate-project")
            .arg("--workspace")
            .arg("--message-format=plain")
            .output()
            .unwrap()
            .stdout;

        let path = Path::new(std::str::from_utf8(&path).unwrap().trim());
        path.parent().unwrap().to_owned()
    });

    &*DIR
}

struct ScriptCompiler {
    modules: HashMap<String, pex::Module>,
}

impl ScriptCompiler {
    fn new() -> Self {
        let srcdir = workspace_dir().join("assets/papyrus");

        let mut compiler = Compiler::new();
        let mut files = Vec::new();

        for entry in std::fs::read_dir(srcdir).unwrap() {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_file() {
                compiler.add_file_source(entry.path()).unwrap();
                files.push(entry.path());
            }
        }

        let mut modules = HashMap::new();
        for path in files {
            let module = compiler.compile_file(&path).unwrap();
            modules.insert(
                path.file_stem().unwrap().to_str().unwrap().to_owned(),
                module,
            );
        }

        Self { modules }
    }
}

pub trait DefaultExt: Default {
    fn default_with<F>(f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        let mut value = Self::default();
        f(&mut value);
        value
    }
}

impl<T: Default> DefaultExt for T {}
