use std::fmt::Debug;
use std::sync::Arc;

use async_lock::Mutex;
use game_common::string::Lowercase;
use hashbrown::{HashMap, HashSet};
use openfo4_jit::{BackendKind, CompilerConfig, Type};
use openfo4_pex::{Class, Module};
use parking_lot::RwLock;

use crate::compiler::resources::{Resources, TypeId, TypeInfo};
use crate::compiler::{CompilationResult, CompilerOptions, Context, NativeFunction};
use crate::runtime::func::Func;
use crate::runtime::{HostState, IntoFunc, ObjectPool, Stack};
use crate::{RegisterNativeError, compiler};

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {
    /// Compilation context.
    ctx: Mutex<Context>,
    resources: Arc<Resources>,
    pub(crate) shared: Arc<EngineShared>,
    instances: RwLock<HashMap<Lowercase<String>, Arc<InstanceShared>>>,
    classes: HashMap<Lowercase<String>, Class>,
    registered_native_funcs: HashSet<(Lowercase<String>, Lowercase<String>)>,
}

impl Engine {
    pub fn new(options: CompilerOptions) -> Self {
        let ctx = Context::new(CompilerConfig {
            validation: options.validation,
            debug: options.debug,
            backend: BackendKind::default(),
        });
        let pool = ObjectPool::new(Arc::clone(ctx.resources()));

        Self {
            resources: Arc::clone(ctx.resources()),
            ctx: Mutex::new(ctx),
            shared: Arc::new(EngineShared {
                state: HostState { pool },
            }),
            instances: RwLock::new(HashMap::new()),
            classes: HashMap::new(),
            registered_native_funcs: HashSet::new(),
        }
    }

    /// Registers the given [`Module`] for the compiler of this `Engine`.
    pub fn register(&mut self, module: &Module) {
        self.ctx.get_mut().register_module(module).unwrap();

        for class in &module.classes {
            self.classes
                .insert(Lowercase::new(class.name.as_str()), class.clone());
        }
    }

    /// Registers a native host function that can be called from the guest.
    ///
    /// # Panics
    ///
    /// Panics if the same a native function with the same `class` and `func` was already registered
    /// with this `Engine`.
    pub fn register_native_function<F, T, Marker, Params, Returns>(
        &mut self,
        class: &str,
        func: &str,
        f: F,
    ) -> Result<(), RegisterNativeError>
    where
        F: IntoFunc<T, Marker, Params, Returns>,
    {
        {
            let class = Lowercase::new(class);
            let func = Lowercase::new(func);

            if !self
                .registered_native_funcs
                .insert((class.clone(), func.clone()))
            {
                panic!("native function {}.{} already registered", class, func);
            }
        }

        let sig = self
            .ctx
            .get_mut()
            .compute_function_signature(class, func)
            .map_err(|err| RegisterNativeError::UnknownFunction(err))?;
        let name = format!("{}.{}", class, func);

        let host_func = f.into_func(sig, name);

        let mut signature = F::signature();
        // Our VM context pointer.
        signature.params.insert(0, Type::Pointer);
        // VM error
        signature.returns.insert(0, Type::I32);

        self.ctx.get_mut().define_native_function(
            class,
            func,
            NativeFunction {
                func: host_func,
                signature,
            },
        )
    }

    pub async fn compile(
        &self,
        class: &str,
        options: &Options,
    ) -> Result<Instance, compiler::Error> {
        let class_name = Lowercase::new(class);

        if let Some(shared) = self.instances.read().get(&class_name) {
            return Ok(Instance {
                shared: shared.clone(),
            });
        }

        let mut ctx = self.ctx.lock().await;

        ctx.compute_type_layouts()?;

        let class = self.classes.get(&class_name).unwrap();

        let mut funcs = HashMap::new();

        let res = ctx.compile(&class.name, options)?;
        for (name, res) in res.functions {
            let func = unsafe { Func::new(&res, Arc::downgrade(&self.shared)) };
            funcs.insert(name, (res, func));
        }

        let shared = Arc::new(InstanceShared { funcs });

        let mut instances = self.instances.write();

        if let Some(shared) = instances.get(&class_name) {
            return Ok(Instance {
                shared: shared.clone(),
            });
        }

        instances.insert(class_name, shared.clone());
        Ok(Instance { shared })
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
        if self.instances.read().get(&name).is_none() && is_class(&name) {
            return None;
        }

        self.resources.string_to_type.lock().get(&name).copied()
    }

    pub fn none_type(&self) -> TypeId {
        self.resources.primitives.none
    }

    pub fn bool_type(&self) -> TypeId {
        self.resources.primitives.bool
    }

    pub fn int_type(&self) -> TypeId {
        self.resources.primitives.int
    }

    pub fn float_type(&self) -> TypeId {
        self.resources.primitives.float
    }

    pub fn ptr_type(&self) -> TypeId {
        self.resources.primitives.pointer
    }

    pub fn type_info(&self, id: TypeId) -> TypeInfo {
        self.resources.types.get(id).unwrap().info.clone()
    }

    /// Creates a new empty [`Stack`] for function suspension and resumption.
    ///
    /// The returned [`Stack`] is bound to this `Engine`.
    pub fn create_stack(&self) -> Stack {
        Stack::new_with_engine(Arc::clone(&self.shared))
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
pub struct InstanceField {
    pub class: String,
    pub name: String,
    pub offset: usize,
}
