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
        let instance = self
            .registry
            .compile(ClassId(class.to_lowercase()))
            .unwrap();
        self.instances.insert(class.to_owned(), instance);
        self.instances.get(class).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct ClassId(pub String);

#[derive(Debug)]
pub struct Registry {
    classes: HashMap<String, Class>,
    native_functions: Vec<(String, String, NativeFunction)>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            native_functions: Vec::new(),
        }
    }

    pub fn register(&mut self, module: &openfo4_pex::Module) -> Result<Vec<ClassId>, Error> {
        let mut classes = Vec::new();

        for class in &module.classes {
            self.classes
                .insert(class.name.to_lowercase(), class.clone());

            classes.push(ClassId(class.name.to_lowercase()));
        }

        Ok(classes)
    }

    pub fn define_native_function(&mut self, class: &str, func: &str, f: NativeFunction) {
        self.native_functions
            .push((class.to_lowercase(), func.to_lowercase(), f));
    }

    pub fn compile(&self, class_id: ClassId) -> Result<ScriptInstance, Error> {
        let mut ctx = Context::new();

        for name in self.classes.keys() {
            ctx.declare_class(&name);
        }

        for class in self.classes.values() {
            if !class.parent.is_empty() {
                ctx.declare_parent_class(&class.name, &class.parent);
            }
        }

        for (name, class) in &self.classes {
            for var in &class.variables {
                ctx.declare_class_field(&name, var);
            }

            for state in &class.states {
                for func in &state.functions {
                    ctx.declare_class_function(&name, &func);
                }
            }
        }

        for (class, func, ptr) in &self.native_functions {
            ctx.define_native_function(&class, &func, ptr.clone());
        }

        let class = self.classes.get(&class_id.0).unwrap();

        let mut functions = HashMap::new();

        for s in &class.states {
            for f in &s.functions {
                if functions.contains_key(&f.name.to_lowercase()) {
                    continue;
                }

                dbg!(&f.name);
                let fn_ptr = ctx.compile_fn(&class_id.0, &f).unwrap();

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
