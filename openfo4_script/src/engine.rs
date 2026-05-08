use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use game_common::cell::UnsafeRefCell;
use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{CompilerConfig, Type};
use openfo4_pex::{Class, Module};

use crate::compiler::resources::{TypeId, TypeInfo};
use crate::compiler::{CompilationResult, CompilerOptions, Context, NativeFunction};
use crate::runtime::host_functions::call_host_trampoline;
use crate::runtime::{CallTable, Func, HostState, IntoFunc, ObjectPool};

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {
    /// Compilation context.
    ctx: Context,
    pub(crate) shared: Arc<EngineShared>,
    instances: HashMap<Lowercase<String>, Arc<InstanceShared>>,
    classes: HashMap<Lowercase<String>, Class>,
}

impl Engine {
    pub fn new(options: CompilerOptions) -> Self {
        let ctx = Context::new(CompilerConfig {
            validation: options.validation,
            debug: options.debug,
        });
        let pool = ObjectPool::new(Arc::clone(ctx.resources()));

        Self {
            ctx,
            shared: Arc::new(EngineShared {
                state: HostState {
                    call_table: UnsafeRefCell::new(CallTable::default()),
                    pool,
                },
                call_table_lock: RwLock::default(),
            }),
            instances: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: &Module) {
        self.ctx.register_module(module).unwrap();

        for class in &module.classes {
            self.classes
                .insert(Lowercase::new(class.name.as_str()), class.clone());
        }
    }

    pub fn register_native_function<F, T, Params, Returns>(&mut self, class: &str, func: &str, f: F)
    where
        F: IntoFunc<T, Params, Returns>,
    {
        let _lock = self.shared.call_table_lock.write();
        let mut call_table = unsafe { self.shared.state.call_table.borrow_mut() };

        let ptr = call_host_trampoline as *const u8;
        let index = f.into_func(&mut call_table);

        let mut signature = F::signature();
        // Our VM context pointer.
        signature.params.insert(0, Type::Pointer);
        // VM error
        signature.returns.push(Type::I32);

        self.ctx.define_native_function(
            class,
            func,
            NativeFunction {
                ptr,
                index,
                signature,
            },
        );
    }

    pub fn compile(&mut self, class: &str, options: &Options) -> Instance {
        let class_name = Lowercase::new(class);

        if let Some(shared) = self.instances.get(&class_name) {
            return Instance {
                shared: shared.clone(),
            };
        }

        self.ctx.compute_type_layouts();

        let class = self.classes.get(&class_name).unwrap();

        let mut funcs = HashMap::new();

        let res = self.ctx.compile(&class.name, options).unwrap();
        for (name, res) in res.functions {
            let func = unsafe { Func::new(&res) };
            funcs.insert(name, (res, func));
        }

        let shared = Arc::new(InstanceShared { funcs });

        self.instances.insert(class_name, shared.clone());
        Instance { shared }
    }

    pub fn object_pool(&self) -> &ObjectPool {
        &self.shared.state.pool
    }

    pub fn get_type(&self, name: &str) -> Option<TypeId> {
        let name = Lowercase::new(name);

        // FIXME: This check is best-effort only;
        // It is in theory possible for classes to contain an #.
        // We should let the compiler expose which types are save
        // to construct.
        fn is_class(name: &Lowercase<str>) -> bool {
            !matches!(name.as_inner(), "none" | "int" | "float" | "string" | "var")
                // Structs 
                && !name.as_inner().contains('#')
        }

        // Even if the class already has a TypeId, it is not
        // safe to construct it before compiling the class first.
        if self.instances.get(&name).is_none() && is_class(&name) {
            return None;
        }

        self.ctx
            .resources()
            .string_to_type
            .lock()
            .get(&name)
            .copied()
    }

    pub fn none_type(&self) -> TypeId {
        self.ctx.resources().primitives.none
    }

    pub fn bool_type(&self) -> TypeId {
        self.ctx.resources().primitives.bool
    }

    pub fn int_type(&self) -> TypeId {
        self.ctx.resources().primitives.int
    }

    pub fn float_type(&self) -> TypeId {
        self.ctx.resources().primitives.float
    }

    pub fn ptr_type(&self) -> TypeId {
        self.ctx.resources().primitives.pointer
    }

    pub fn type_info(&self, id: TypeId) -> TypeInfo {
        self.ctx.resources().infos.lock().get(&id).cloned().unwrap()
    }
}

impl Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Skip compilation of native functions.
    pub skip_native: bool,
    /// Skip compliation of empty functions.
    pub skip_empty: bool,
    /// Use stub functions for all undefined native functions.
    ///
    /// Exists purely for compile testing and may result in crashes when calling into functions
    /// which call stub functions.
    ///
    /// Default: `false`
    pub use_native_stubs: bool,
}

pub(crate) struct EngineShared {
    pub(crate) call_table_lock: RwLock<()>,
    pub(crate) state: HostState,
}

impl Debug for EngineShared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineShared").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ClassId(pub String);

/// An compiled instance of a script.
#[derive(Clone, Debug)]
pub struct Instance {
    shared: Arc<InstanceShared>,
}

impl Instance {
    pub fn get_func(&self, name: &str) -> Option<&Func> {
        let (_, func) = self.shared.funcs.get(&Lowercase::new(name))?;
        Some(func)
    }
}

#[derive(Debug)]
struct InstanceShared {
    funcs: HashMap<Lowercase<String>, (CompilationResult, Func)>,
}

#[derive(Clone, Debug)]
pub struct ScriptInstance {
    functions: HashMap<String, CompilationResult>,
}

impl ScriptInstance {
    pub fn get_func(&self, name: &str) -> Option<Func> {
        let res = self.functions.get(&name.to_lowercase())?;
        Some(unsafe { Func::new(res) })
    }
}

#[derive(Clone, Debug)]
pub struct InstanceField {
    pub class: String,
    pub name: String,
    pub offset: usize,
}
