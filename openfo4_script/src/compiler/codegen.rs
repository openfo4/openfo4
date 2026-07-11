// mod builtin;

mod function;
mod hir;
mod pex;
mod runtime;
mod trace;
mod trampoline;

use std::collections::VecDeque;
use std::sync::Arc;

use game_common::string::Lowercase;
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use indexmap::IndexMap;
use openfo4_jit::{Compiler, CompilerConfig, FuncId, Signature};
use openfo4_pex::{Class, Instruction, Module, Struct, Value};
use thiserror::Error;

use crate::compiler::codegen::pex::layout;
use crate::compiler::codegen::pex::registry::{
    ClassDef, ClassId, FunctionDef, FunctionId, FunctionStateImpl, Registry, StructId, Type,
};
use crate::compiler::codegen::runtime::RuntimeFunctions;
use crate::compiler::resources::{Resources, TypeId, TypeRepr};
use crate::compiler::{CompilationResult, CompiledClass, NativeFunction};
use crate::engine::Options;
use crate::{RegisterNativeError, UnknownFunctionError};

#[derive(Clone, Debug, Error)]
#[error("codegen error")]
pub enum Error {
    #[error("undefined native function: {0}")]
    UndefNativeFunc(String),
    #[error(
        "native function signature missmatch for {name}: host definition: {host:?}, script definition: {script:?}"
    )]
    NativeFuncSignatureMissmatch {
        name: String,
        script: Signature,
        host: Signature,
    },
}

pub struct CodegenPass {
    data: CodegenData,
    compiler: Compiler,
    classes_updated: bool,
    enable_debug: bool,
    define_classes: HashMap<ClassId, Class>,
    define_structs: HashMap<StructId, Struct>,
}

impl CodegenPass {
    pub fn new(config: CompilerConfig) -> Self {
        let mut compiler = Compiler::new(config.clone());

        Self {
            data: CodegenData::new(&mut compiler),
            compiler,
            classes_updated: false,
            enable_debug: config.debug,
            define_classes: HashMap::new(),
            define_structs: HashMap::new(),
        }
    }

    pub fn resources(&self) -> &Arc<Resources> {
        &self.data.resources
    }

    pub fn register_module(&mut self, module: &Module) {
        for class in &module.classes {
            let name = Lowercase::new(&*class.name);
            let id = self.data.registry.declare_class(name);
            self.define_classes.insert(id, class.clone());

            for st in &class.structs {
                let mut st = st.clone();
                st.name = format!("{}#{}", class.name, st.name);
                let name = Lowercase::new(&*st.name);
                let id = self.data.registry.declare_struct(name);
                self.define_structs.insert(id, st.clone());
            }

            self.classes_updated = true;
        }
    }

    fn register_defined_types(&mut self) {
        let mut queue: VecDeque<_> = self
            .define_classes
            .iter()
            .map(|(_, class)| Lowercase::new(&*class.name))
            .collect();
        let mut classes_by_name: HashMap<_, _> = self
            .define_classes
            .drain()
            .map(|(id, class)| (Lowercase::new(&*class.name), (id, class)))
            .collect();

        while let Some(name) = queue.pop_front() {
            if self.data.registry.class_by_name(&name).is_some() {
                continue;
            }

            let (_, class) = classes_by_name.get(&name).unwrap();

            // We must register the parent class before any subclasses.
            if !class.parent.is_empty() && !class.parent.eq_ignore_ascii_case("none") {
                let parent = Lowercase::new(&*class.parent);
                if self.data.registry.class_by_name(&parent).is_none() {
                    queue.push_front(name);
                    queue.push_front(parent);
                    continue;
                }
            }

            let (id, class) = classes_by_name.remove(&name).unwrap();
            self.data.registry.define_class(id, class);
        }

        for (id, st) in self.define_structs.drain() {
            self.data.registry.define_struct(id, st);
        }
    }

    pub fn define_native_function(
        &mut self,
        class: &str,
        function: &str,
        func: NativeFunction,
    ) -> Result<(), RegisterNativeError> {
        let class = Lowercase::new(class);
        let function = Lowercase::new(function);

        let class = self.data.registry.class_by_name(&class).unwrap();
        let function = class.functions.get(&function).unwrap_or_else(|| {
            panic!("unknown native function: {}.{}", class.debug_name, function);
        });

        if self.data.native_funcs.insert(function, func).is_some() {
            panic!("cannot redefined native functions");
        }

        Ok(())
    }

    pub fn compute_type_layouts(&mut self) {
        self.register_defined_types();
        if !self.classes_updated {
            return;
        }
        self.classes_updated = false;

        for (id, st) in self.data.registry.structs() {
            let ty_id = self.data.resources.types.declare();
            self.data
                .resources
                .string_to_type
                .lock()
                .insert(st.name.to_owned(), ty_id);

            self.data.struct_types.insert(id, ty_id);
        }

        let mut class_queue = VecDeque::new();
        for (id, class) in &mut self.data.registry.classes() {
            let ty_id = self.data.resources.types.declare();
            self.data
                .resources
                .string_to_type
                .lock()
                .insert(class.name.clone(), ty_id);

            self.data.class_types.insert(id, ty_id);
            class_queue.push_back(id);
        }

        for (id, st) in self.data.registry.structs() {
            let ty_id = self.data.struct_types[&id];

            let (ty, offsets) =
                layout::compute_struct_layout(&self.data.resources, &self.data.registry, ty_id, st);

            self.data.resources.types.define(ty_id, ty);
            self.data.struct_field_offsets.insert(id, offsets);
        }

        while let Some(id) = class_queue.pop_front() {
            let class = self.data.registry.class(id).unwrap();
            let ty_id = self.data.class_types[&id];

            let (parent_ty, parent_offsets) = if let Some(parent) = class.parent {
                let (Some(ty), Some(offsets)) = (
                    self.data.class_types.get(&parent),
                    self.data.class_field_offsets.get(&parent),
                ) else {
                    class_queue.push_front(id);
                    class_queue.push_front(parent);
                    continue;
                };

                (Some(*ty), Some(offsets.clone()))
            } else {
                (None, None)
            };

            let (ty, offsets) = layout::compute_class_layout(
                &self.data.resources,
                &self.data.registry,
                class,
                ty_id,
                parent_ty,
                parent_offsets,
            );

            self.data.resources.types.define(ty_id, ty);
            self.data.class_field_offsets.insert(id, offsets);
        }
    }

    fn build_vtable_for_class(&self, class: &ClassDef) -> Vec<usize> {
        let mut vtable = Vec::new();
        for func_id in class.functions.new.values() {
            let addr = self.data.function_addrs[func_id];
            vtable.push(addr);
        }

        vtable
    }

    fn build_class_headers(&mut self) {
        let mut classes_to_build = HashSet::new();
        for (id, _) in &self.data.function_addrs {
            let func = self.data.registry.function(*id).unwrap();
            // Static functions don't need any vtable.
            if func.is_static {
                continue;
            }

            if self.data.class_headers.contains_key(&func.class) {
                continue;
            }

            classes_to_build.insert(func.class);
        }

        // Classes without functions will never get visited be the above loop.
        // While we don't need a vtable for these classes, we still need to their
        // header.
        // We simply visit any functionless classes whenever the parent is visited.
        for (_, class) in self.data.registry.classes() {
            if !class.functions.new.is_empty() || !class.functions.overriden.is_empty() {
                continue;
            }

            if self.data.class_headers.contains_key(&class.id) {
                continue;
            }

            // Note that this will still work correctly when the parent is also functionless,
            // since parents will get yielded after child classes in the iterator.
            if let Some(parent) = class.parent {
                if !self.data.class_headers.contains_key(&parent)
                    && !classes_to_build.contains(&parent)
                {
                    continue;
                }
            }

            classes_to_build.insert(class.id);
        }

        let mut classes_to_build: VecDeque<_> = classes_to_build.into_iter().collect();
        while let Some(id) = classes_to_build.pop_front() {
            let class = self.data.registry.class(id).unwrap();

            tracing::debug!("building vtable for {}", class.debug_name);

            if self.data.class_headers.contains_key(&id) {
                continue;
            }

            // We must build the parent header first.
            if let Some(parent) = class.parent
                && !self.data.class_headers.contains_key(&parent)
            {
                classes_to_build.push_front(id);
                classes_to_build.push_front(parent);
                continue;
            }

            let mut root = class;
            let mut vtables = IndexMap::new();
            vtables.insert(class.id, self.build_vtable_for_class(class));
            while let Some(parent) = root.parent {
                root = self.data.registry.class(parent).unwrap();
                vtables.insert(root.id, self.build_vtable_for_class(root));
            }
            vtables.reverse();

            for func in class.functions.overriden.values() {
                let def_class = self.data.registry.class(func.def_class).unwrap();
                let vtable = &mut vtables[&func.def_class];

                let index = def_class
                    .functions
                    .new
                    .values()
                    .position(|f| *f == func.def_function)
                    .unwrap();
                let addr = self.data.function_addrs[&func.func];
                vtable[index] = addr;
            }

            let mut data: Vec<usize> = vec![vtables.len()];

            for (id, vtable) in vtables {
                let ty = self.data.class_types[&id];
                data.push(ty.as_bits());

                let vtable_ptr = self
                    .data
                    .resources
                    .constants
                    .alloc(bytemuck::cast_slice(&vtable), align_of::<usize>());
                data.push(vtable_ptr.addr())
            }

            let type_id = self.data.class_types[&class.id];

            let header_ptr = self
                .data
                .resources
                .constants
                .alloc(bytemuck::cast_slice(&data), align_of::<usize>());
            self.data.class_headers.insert(id, data);

            // Update the "default value" for the class header ptr.
            let mut ty =
                Arc::make_mut(&mut self.data.resources.types.get(type_id).unwrap()).clone();
            match &mut ty.repr {
                TypeRepr::Composite(ty) => {
                    ty.members[0].default_value =
                        Some(bytemuck::bytes_of(&header_ptr.addr()).to_vec());
                }
                _ => unreachable!(),
            }
            self.data.resources.types.define(type_id, ty);
        }
    }

    pub fn compile(&mut self, class: &str, options: &Options) -> Result<CompiledClass, Error> {
        let _span = trace_span!("CodegenPass::compile").entered();

        tracing::debug!("compiling class {}", class);

        self.compute_type_layouts();

        let class_name = Lowercase::new(class);

        let class = self.data.registry.class_by_name(&class_name).unwrap();

        let mut functions = HashMap::new();
        let funcs: Vec<_> = class.functions.iter().collect();
        for id in funcs {
            let func = self.data.registry.function(id).unwrap();
            let Some(name) = &func.name else {
                continue;
            };
            let name = name.clone();

            let res = self.compile_function(id, options)?;
            functions.insert(name, res);
        }

        // Update vtable headers after functions are build.
        self.build_class_headers();

        Ok(CompiledClass { functions })
    }

    fn compile_function(
        &mut self,
        root_func_id: FunctionId,
        options: &Options,
    ) -> Result<CompilationResult, Error> {
        let _span = trace_span!("CodegenPass::compile_function").entered();

        // We already compiled the function.
        if let Some(res) = self.data.native_guest_funcs.get(&root_func_id) {
            return Ok(res.clone());
        }

        let mut funcs_to_compile = HashSet::<FunctionId>::new();
        let mut queue = vec![root_func_id];
        while let Some(func_id) = queue.pop() {
            // Function was already compiled.
            if self.data.functions.contains_key(&func_id) {
                continue;
            }

            if funcs_to_compile.contains(&func_id) {
                continue;
            }

            funcs_to_compile.insert(func_id);

            let func = self.data.registry.function(func_id).unwrap();

            let mut callees = find_function_callees(&self.data, &func);

            // We also need to consider special handling for the root function.
            // FIXME: Can we move this out of the loop somehow?
            if func_id == root_func_id {
                callees.insert(func_id);
            }

            for callee in callees {
                queue.push(callee);

                let func = self.data.registry.function(callee).unwrap();
                if func.is_static {
                    continue;
                }

                // While we don't actually need to call any of these functions,
                // calling any non-static function of a class requires us to build
                // a complete vtable with all function addresses, i.e. we must compile
                // all functions in that class.
                // This also includes all functions of the parent class.
                let mut class = self.data.registry.class(func.class).unwrap();
                loop {
                    for func_id in class.functions.new.values() {
                        queue.push(*func_id);
                    }

                    // We need to visit both our overriding function and the original
                    // function. The original function will be visited when the visit
                    // our parent and we don't need to push it here.
                    for func in class.functions.overriden.values() {
                        queue.push(func.func);
                    }

                    if let Some(parent) = class.parent {
                        class = self.data.registry.class(parent).unwrap();
                    } else {
                        break;
                    }
                }
            }
        }

        for &func_id in &funcs_to_compile {
            let func = self.data.registry.function(func_id).unwrap();
            let class = self.data.registry.class(func.class).unwrap();

            let id = function::declare_function(
                &self.data,
                class,
                func_id,
                func,
                &mut self.compiler,
                options.use_native_stubs,
            )?;
            self.data.functions.insert(func_id, id);
        }

        let mut results = Vec::new();
        for &id in &funcs_to_compile {
            let func = self.data.registry.function(id).unwrap();
            let class = self.data.registry.class(func.class).unwrap();

            let res = function::translate_function(
                &self.data,
                class,
                func,
                &mut self.compiler,
                self.enable_debug,
            );
            results.push(res);
        }

        // Note: We cannot merge this loop into the previous loop.
        // We need to translate all required functions before we start
        // compiling any of them, otherwise cycles would not resolve
        // correctly.
        for (id, res) in funcs_to_compile.into_iter().zip(results) {
            let func_id = self.data.functions[&id];
            let func = self.compiler.compile_function(func_id);

            self.data.function_addrs.insert(id, func.as_ptr().addr());

            if let Some(info) = &res.debug {
                super::debug::write_debug_output(info, &func);
            }
        }

        // Generate a trampoline function that converts from the internal
        // calling convention to the host natve calling convention.
        let func_id = self.data.functions[&root_func_id];
        let func = self.data.registry.function(root_func_id).unwrap();
        let sig = function::translate_function_signature(func);
        let trampoline_id =
            trampoline::generate_host_to_guest_trampoline(&mut self.compiler, func_id, &sig);
        let function = self.compiler.compile_function(trampoline_id);

        let psc_signature = function::translate_function_psc_signature(&self.data, func);
        let res = CompilationResult {
            function,
            signature: sig,
            psc_signature,
        };

        self.data
            .native_guest_funcs
            .insert(root_func_id, res.clone());

        Ok(res)
    }

    pub fn compute_function_signature(
        &self,
        class_str: &str,
        func_str: &str,
    ) -> Result<PscSignature, UnknownFunctionError> {
        let class = Lowercase::new(&*class_str);
        let func = Lowercase::new(&*func_str);
        let class =
            self.data
                .registry
                .class_by_name(&class)
                .ok_or_else(|| UnknownFunctionError {
                    class: class_str.to_owned(),
                    func: func_str.to_owned(),
                })?;
        let func = class
            .functions
            .get(&func)
            .ok_or_else(|| UnknownFunctionError {
                class: class_str.to_owned(),
                func: func_str.to_owned(),
            })?;
        let func = self.data.registry.function(func).unwrap();
        Ok(function::translate_function_psc_signature(&self.data, func))
    }
}

#[derive(Clone, Debug)]
struct CodegenData {
    resources: Arc<Resources>,
    runtime_funcs: RuntimeFunctions,
    registry: Registry,
    struct_types: HashMap<StructId, TypeId>,
    struct_field_offsets: HashMap<StructId, HashMap<Lowercase<String>, usize>>,
    class_types: HashMap<ClassId, TypeId>,
    class_field_offsets: HashMap<ClassId, HashMap<Lowercase<String>, usize>>,
    functions: HashMap<FunctionId, FuncId>,
    function_addrs: HashMap<FunctionId, usize>,
    native_funcs: HashMap<FunctionId, NativeFunction>,
    /// Functions defined in the guest that can be called from the host with `extern "C"` ABI.
    native_guest_funcs: HashMap<FunctionId, CompilationResult>,
    class_headers: HashMap<ClassId, Vec<usize>>,
}

impl CodegenData {
    fn new(compiler: &mut Compiler) -> Self {
        Self {
            resources: Arc::default(),
            runtime_funcs: RuntimeFunctions::new(compiler),
            registry: Registry::default(),
            struct_types: HashMap::new(),
            struct_field_offsets: HashMap::new(),
            class_types: HashMap::new(),
            class_field_offsets: HashMap::new(),
            functions: HashMap::new(),
            native_funcs: HashMap::new(),
            native_guest_funcs: HashMap::new(),
            function_addrs: HashMap::new(),
            class_headers: HashMap::new(),
        }
    }
}

fn find_function_callees(data: &CodegenData, func: &FunctionDef) -> HashSet<FunctionId> {
    let mut callees = HashSet::new();

    let class_id = func.class;
    let class = data.registry.class(func.class).unwrap();

    for (_, state) in &func.states {
        match state {
            FunctionStateImpl::Body(body) => {
                let find_var_as_class = |var_name: &Lowercase<str>| -> ClassId {
                    match var_name.as_inner() {
                        "self" => return class_id,
                        _ => (),
                    }

                    for (name, ty) in body.params.iter().zip(&func.params) {
                        if name == var_name {
                            match ty {
                                Type::Class(id) => return *id,
                                _ => unreachable!(),
                            }
                        }
                    }

                    if let Some(ty) = body.locals.get(var_name) {
                        match ty {
                            Type::Class(id) => return *id,
                            _ => unreachable!(),
                        }
                    }

                    let mut class = class;
                    loop {
                        if let Some(field) = class.fields.get(var_name) {
                            match field.ty {
                                Type::Class(id) => return id,
                                _ => unreachable!(),
                            }
                        }

                        if let Some(parent) = class.parent {
                            class = data.registry.class(parent).unwrap();
                        } else {
                            break;
                        }
                    }

                    todo!()
                };

                for instruction in &body.instructions {
                    match instruction {
                        Instruction::CallMethod(op) => {
                            let Value::Ident(object) = &op.object else {
                                unreachable!()
                            };

                            let class = find_var_as_class(&Lowercase::new(&**object));
                            let func_name = Lowercase::new(&*op.function_name);

                            let parent = data.registry.class(class).unwrap();
                            let func = parent.functions.get_method(&func_name).unwrap();

                            callees.insert(func);
                        }
                        Instruction::CallParent(op) => {
                            let func_name = Lowercase::new(op.function.as_str());
                            let func = class.functions.get_parent(&func_name).unwrap();
                            callees.insert(func);
                        }
                        Instruction::CallStatic(op) => {
                            let class_name = Lowercase::new(op.class.as_str());
                            let func_name = Lowercase::new(op.function.as_str());

                            let class = data.registry.class_by_name(&class_name).unwrap();
                            let func = class.functions.get_static(&func_name).unwrap();

                            callees.insert(func);
                        }
                        Instruction::PropertyGet(op) => {
                            let Value::Ident(object) = &op.value else {
                                unreachable!()
                            };

                            let object = Lowercase::new(object.as_str());
                            let class = find_var_as_class(&object);
                            let prop_name = Lowercase::new(op.property.as_str());

                            let class =
                                get_parent_class_with_property(data, class, &prop_name).unwrap();
                            let class = data.registry.class(class).unwrap();
                            let prop = class.properties.get(&prop_name).unwrap();

                            if let Some(func) = prop.getter {
                                callees.insert(func);
                            }
                        }
                        Instruction::PropertySet(op) => {
                            let Value::Ident(object) = &op.object else {
                                unreachable!()
                            };

                            let object = Lowercase::new(object.as_str());
                            let class = find_var_as_class(&object);
                            let prop_name = Lowercase::new(op.property.as_str());

                            let class =
                                get_parent_class_with_property(data, class, &prop_name).unwrap();
                            let class = data.registry.class(class).unwrap();
                            let prop = class.properties.get(&prop_name).unwrap();

                            if let Some(func) = prop.setter {
                                callees.insert(func);
                            }
                        }
                        _ => {}
                    }
                }
            }
            FunctionStateImpl::Call(id) => {
                callees.insert(*id);
            }
        }
    }

    callees
}

fn get_parent_class_with_property(
    data: &CodegenData,
    mut class_id: ClassId,
    prop_name: &Lowercase<str>,
) -> Option<ClassId> {
    loop {
        let class = data.registry.class(class_id)?;

        if class.properties.get(prop_name).is_some() {
            return Some(class.id);
        }

        let parent = class.parent?;
        class_id = parent;
    }
}

#[derive(Clone, Debug, Default)]
pub struct PscSignature {
    pub params: Vec<TypeId>,
    pub returns: Vec<TypeId>,
}
