pub mod classes;
pub mod compiler;
pub mod runtime;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::mem::transmute;
use std::sync::{Arc, RwLock};

use game_common::cell::UnsafeRefCell;
use game_common::collections::arena::{Arena, Key};
use openfo4_jit::{CompiledFunction, Type};
use openfo4_pex::{Class, Module};

pub type NativeFuncPtr =
    unsafe extern "C" fn(*const runtime::HostState, usize, *mut runtime::RawVal);

use crate::compiler::{ClassFieldLayout, CompilationResult, Context, NativeFunction};
use crate::runtime::{Func, HostState, IntoFunc, NativeFnPtr, call_host_trampoline};

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {
    registry: Registry,
    shared: Arc<EngineShared>,
    layouts: HashMap<String, Vec<ClassFieldLayout>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            shared: Arc::new(EngineShared::default()),
            layouts: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: &Module) {
        self.registry.register(module).unwrap();
    }

    pub fn register_native_function<F, T, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Params, Returns>,
    {
        let _lock = self.shared.state_lock.write();
        let mut state = unsafe { self.shared.state.borrow_mut() };

        let ptr = call_host_trampoline as *const u8;
        let index = f.into_func(&mut state);

        let mut signature = F::signature();
        // Our VM context pointer.
        signature.params.insert(0, Type::Pointer);
        // VM error
        signature.returns.push(Type::I32);

        self.registry.define_native_function(
            class,
            func,
            NativeFunction {
                ptr,
                index,
                signature,
            },
        );
    }

    pub fn compile(&mut self, class: &str) -> Instance {
        let instance = self.registry.compile(self.shared.clone(), class).unwrap();

        for (_, (f, _)) in &instance.functions {
            self.layouts.extend(f.class_layouts.clone());
        }

        instance
    }

    pub fn get_class_layout(&self, class: &str) -> Option<&[ClassFieldLayout]> {
        self.layouts
            .get(&class.to_lowercase())
            .map(|v| v.as_slice())
    }
}

#[derive(Default)]
struct EngineShared {
    state_lock: RwLock<()>,
    state: UnsafeRefCell<HostState>,
}

#[derive(Clone, Debug)]
pub struct ClassId(pub String);

#[derive(Debug)]
pub struct Registry {
    ctx: Context,
    native_functions: Vec<(String, String, NativeFunction)>,
    classes: HashMap<String, Class>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            native_functions: Vec::new(),
            ctx: Context::new(),
        }
    }

    pub fn register(&mut self, module: &Module) -> Result<(), Error> {
        self.ctx.register_module(module).unwrap();

        for class in &module.classes {
            self.classes
                .insert(class.name.to_lowercase(), class.clone());
        }

        Ok(())
    }

    pub fn define_native_function(&mut self, class: &str, func: &str, f: NativeFunction) {
        // self.native_functions
        //     .push((class.to_lowercase(), func.to_lowercase(), f));

        self.ctx.define_native_function(class, func, f);
    }

    fn compile(&mut self, engine: Arc<EngineShared>, class: &str) -> Result<Instance, Error> {
        let mut functions = HashMap::new();

        let class = self.classes.get(&class.to_lowercase()).unwrap();

        for s in &class.states {
            for f in &s.functions {
                if functions.contains_key(&f.name.to_lowercase()) {
                    continue;
                }

                let res = self.ctx.compile(&class.name, &f.name).unwrap();

                let func = unsafe { Func::new(&res) };

                functions.insert(f.name.to_lowercase(), (res, func));
            }
        }

        Ok(Instance { engine, functions })
    }
}

/// An compiled instance of a script.
#[derive(Clone)]
pub struct Instance {
    engine: Arc<EngineShared>,
    functions: HashMap<String, (CompilationResult, Func)>,
}

impl Instance {
    pub fn get_func(&self, name: &str) -> Option<&Func> {
        let (_, func) = self.functions.get(&name.to_lowercase())?;
        Some(func)
    }
}

#[derive(Clone, Debug)]
pub struct ScriptInstance {
    functions: HashMap<String, CompilationResult>,
}

impl ScriptInstance {
    pub fn get_func(&self, name: &str) -> Option<Func> {
        let res = self.functions.get(&name.to_lowercase())?;

        Some(Func {
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(res.function.as_ptr()) },
            signature: res.function.signature().clone(),
            stack_size: res.stack_size,
        })
    }
}

#[derive(Clone, Debug)]
pub struct InstanceField {
    pub class: String,
    pub name: String,
    pub offset: usize,
}
