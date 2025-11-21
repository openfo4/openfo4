pub mod classes;
pub mod compiler;
pub mod runtime;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::mem::transmute;

use game_common::collections::arena::{Arena, Key};
use openfo4_jit::{CompiledFunction, Type};
use openfo4_pex::{Class, Module};

pub type NativeFuncPtr =
    unsafe extern "C" fn(*const runtime::HostState, usize, *mut runtime::RawVal);

use crate::compiler::{Context, NativeFunction};
use crate::runtime::{Func, HostState, IntoFunc, NativeFnPtr, call_host_trampoline};

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {
    registry: Registry,
    pub state: HostState,
    instances: HashMap<String, ScriptInstance>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            state: HostState::default(),
            instances: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: &Module) {
        self.registry.register(module).unwrap();
    }

    pub fn register_native_function<F, T, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Params, Returns>,
    {
        let ptr = call_host_trampoline as *const u8;
        let index = f.into_func(&mut self.state);

        let mut signature = F::signature();
        // Our VM context pointer.
        signature.params.insert(0, Type::Pointer);

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

    pub fn compile(&mut self, class: &str) -> &ScriptInstance {
        let instance = self.registry.compile(class).unwrap();
        self.instances.insert(class.to_owned(), instance);
        self.instances.get(class).unwrap()
    }
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

    pub fn compile(&mut self, class: &str) -> Result<ScriptInstance, Error> {
        let mut functions = HashMap::new();

        let class = self.classes.get(&class.to_lowercase()).unwrap();

        for s in &class.states {
            for f in &s.functions {
                if functions.contains_key(&f.name.to_lowercase()) {
                    continue;
                }

                dbg!(&f.name);
                let fn_ptr = self.ctx.compile(&class.name, &f.name).unwrap();

                functions.insert(f.name.to_lowercase(), fn_ptr);
            }
        }

        Ok(ScriptInstance { functions })
    }
}

#[derive(Clone, Debug)]
pub struct ScriptInstance {
    functions: HashMap<String, CompiledFunction>,
}

impl ScriptInstance {
    pub fn get_func(&self, name: &str) -> Option<Func> {
        let func = self.functions.get(&name.to_lowercase())?;

        Some(Func {
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(func.as_ptr()) },
            signature: func.signature().clone(),
        })
    }
}
