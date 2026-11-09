use std::fmt::Debug;
use std::sync::Arc;

use async_lock::Mutex;
use hashbrown::{HashMap, HashSet};
use openfo4_common::string::Lowercase;
use openfo4_jit::{BackendKind, CompilerConfig, Type};
use openfo4_pex::{Class, Module};
use parking_lot::RwLock;

use crate::compiler::resources::{self, Resources, TypeId, TypeInfo, TypeRepr};
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
    classes: parking_lot::Mutex<HashMap<Lowercase<String>, Class>>,
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
            classes: parking_lot::Mutex::new(HashMap::new()),
            registered_native_funcs: HashSet::new(),
        }
    }

    /// Registers the given [`Module`] for the compiler of this `Engine`.
    ///
    /// This function may block until parallel compilations are done.
    pub async fn register(&self, module: &Module) {
        // Lock ctx, then classes.
        let mut ctx = self.ctx.lock().await;
        let mut classes = self.classes.lock();

        ctx.register_module(module).unwrap();

        for class in &module.classes {
            classes.insert(Lowercase::new(class.name.as_str()), class.clone());
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

        // Lock ctx, then classes.
        let mut ctx = self.ctx.lock().await;
        let classes = self.classes.lock();

        ctx.compute_type_layouts()?;

        let class = classes.get(&class_name).unwrap();

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

    pub fn get_type(&self, mut name: &str) -> Option<TypeRef<'_>> {
        let mut array_depth = 0;
        while let Some(next) = name.strip_suffix("[]") {
            name = next;
            array_depth += 1;
        }

        if array_depth > 1 {
            todo!("nested arays are not yet supported");
        }

        let name = Lowercase::new(name);

        let types = self.resources.papyrus.types.lock();
        let mut id = *types.get(&name)?;
        drop(types);

        if array_depth > 0 {
            id = *self.resources.papyrus.type_to_array_ptr.lock().get(&id)?;
        }

        self.get_type_by_id(id)
    }

    pub fn get_type_by_id(&self, id: TypeId) -> Option<TypeRef<'_>> {
        let ty = self.resources.types.get(id)?;
        let mut inner_ty = id;
        let mut next = ty.clone();
        while let TypeRepr::Pointer(inner) = next.repr {
            next = self.resources.types.get(inner).unwrap();
            inner_ty = inner;
        }

        Some(TypeRef {
            engine: self,
            id,
            inner_ty,
            ty,
        })
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

#[derive(Clone, Debug)]
pub struct TypeRef<'a> {
    engine: &'a Engine,
    id: TypeId,
    /// If `id` is a pointer this is the type the pointer references.
    inner_ty: TypeId,
    ty: Arc<resources::Type>,
    // elem_id: TypeId,
    // elem_ty: Arc<resources::Type>,
    // depth: usize,
}

impl TypeRef<'_> {
    pub fn id(&self) -> TypeId {
        self.id
    }

    pub fn info(&self) -> &TypeInfo {
        &self.ty.info
    }

    /// Returns the type of the elements of an array type.
    ///
    /// If this type is not an array `None` is returned.
    pub fn elem_ty(&self) -> Option<Self> {
        let id = *self
            .engine
            .resources
            .papyrus
            .arrays_to_elem
            .lock()
            .get(&self.inner_ty)?;

        self.engine.get_type_by_id(id)
    }
}
