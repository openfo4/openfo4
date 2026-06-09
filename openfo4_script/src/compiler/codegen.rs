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
use openfo4_jit::{Compiler, CompilerConfig, FuncId, Signature, Type as JitType};
use openfo4_pex::{
    Class, Function, Instruction, Module, Property, Struct, Value, Variable, VariableDefinition,
};
use slab::Slab;
use thiserror::Error;

// use crate::compiler::codegen::layout::{ClassLayout, compute_struct_layout};
use crate::compiler::codegen::runtime::RuntimeFunctions;
use crate::compiler::debug::DebugInfo;
use crate::compiler::resources::{
    self, CompositeType, CompositeTypeMember, PropertyType, Resources, TypeId, TypeInfo,
    TypeProperty, TypeRepr,
};
use crate::compiler::{CompilationResult, CompiledClass, NativeFunction};
use crate::engine::Options;
use crate::runtime::ArrayObject;

pub const MAX_INSTRUCTIONS_BEFORE_PREEMPT: usize = 1024;

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
}

impl CodegenPass {
    pub fn new(config: CompilerConfig) -> Self {
        let mut compiler = Compiler::new(config.clone());

        Self {
            data: CodegenData::new(&mut compiler),
            compiler,
            classes_updated: false,
            enable_debug: config.debug,
        }
    }

    pub fn resources(&self) -> &Arc<Resources> {
        &self.data.resources
    }

    pub fn register_module(&mut self, module: &Module) {
        for class in &module.classes {
            self.classes_updated = true;

            let parent = if class.parent.is_empty() || class.parent.eq_ignore_ascii_case("none") {
                None
            } else {
                Some(Lowercase::new(&*class.parent))
            };

            let mut fields = HashMap::new();
            for var in &class.variables {
                fields.insert(
                    Lowercase::new(&*var.definition.name),
                    Lowercase::new(&*var.definition.ty),
                );
            }

            let mut functions = IndexMap::new();
            for state in &class.states {
                for func in &state.functions {
                    let id = FunctionId(self.data.functions.insert(FunctionDef {
                        signature: None,
                        func_id: None,
                        function: func.clone(),
                        native: None,
                        trampoline_func_id: None,
                        class: Lowercase::new(&*class.name),
                        class_name_debug: class.name.clone(),
                        name: Lowercase::new(&*func.name),
                        psc_signature: None,
                        debug: None,
                        states: HashMap::new(),
                    }));

                    functions.insert(Lowercase::new(&*func.name), id);
                }
            }

            for st in &class.structs {
                let name = format!("{}#{}", class.name, st.name);
                self.data.structs.insert(
                    Lowercase::new(&*name),
                    StructDef {
                        s: st.clone(),
                        field_offsets: HashMap::new(),
                        ty: TypeId::DANGLING,
                    },
                );
            }

            let mut properties = HashMap::new();
            for prop in &class.properties {
                let variable = prop.variable.as_ref().map(|v| Lowercase::new(v.as_str()));

                let getter = prop.getter.as_ref().map(|func| {
                    FunctionId(
                        self.data.functions.insert(FunctionDef {
                            signature: None,
                            func_id: None,
                            function: func.clone(),
                            native: None,
                            trampoline_func_id: None,
                            class: Lowercase::new(&*class.name),
                            class_name_debug: class.name.clone(),
                            name: Lowercase::new(&*func.name),
                            psc_signature: None,
                            debug: None,
                            // Getters/Setters cannot have states.
                            states: [(StateId::Default, FunctionStateImpl::Body(func.clone()))]
                                .into_iter()
                                .collect(),
                        }),
                    )
                });

                let setter = prop.setter.as_ref().map(|func| {
                    FunctionId(
                        self.data.functions.insert(FunctionDef {
                            signature: None,
                            func_id: None,
                            function: func.clone(),
                            native: None,
                            trampoline_func_id: None,
                            class: Lowercase::new(&*class.name),
                            class_name_debug: class.name.clone(),
                            name: Lowercase::new(&*func.name),
                            psc_signature: None,
                            debug: None,
                            // Getters/Setters cannot have states.
                            states: [(StateId::Default, FunctionStateImpl::Body(func.clone()))]
                                .into_iter()
                                .collect(),
                        }),
                    )
                });

                properties.insert(
                    Lowercase::new(&*prop.name),
                    ClassProperty {
                        variable,
                        getter,
                        setter,
                    },
                );
            }

            self.data.classes.insert(
                Lowercase::new(&*class.name),
                ClassDef {
                    name: Lowercase::new(&*class.name),
                    functions,
                    parent,
                    fields,
                    type_id: TypeId::DANGLING,
                    properties,
                    class: Some(class.clone()),
                    field_offsets: HashMap::new(),
                },
            );
        }
    }

    pub fn define_native_function(&mut self, class: &str, function: &str, func: NativeFunction) {
        let class = Lowercase::new(class);
        let function = Lowercase::new(function);

        let Some(func_def) = self
            .data
            .get_function_mut(class.as_str(), function.as_str())
        else {
            panic!("unknown native function: {class}.{function}");
        };

        func_def.native = Some(func);
    }

    pub fn compute_type_layouts(&mut self) {
        if !self.classes_updated {
            return;
        }
        self.classes_updated = false;

        self.update_class_states();

        let resolve_ty_and_default_value = |ty: &Type| -> (TypeId, Option<Vec<u8>>) {
            match ty {
                Type::None => (self.data.resources.primitives.none, None),
                Type::Bool => (self.data.resources.primitives.bool, Some(vec![0; 1])),
                Type::Int => (
                    self.data.resources.primitives.int,
                    Some(vec![0; size_of::<i32>()]),
                ),
                Type::Float => (
                    self.data.resources.primitives.float,
                    Some(vec![0; size_of::<f32>()]),
                ),
                Type::String | Type::Struct(_) | Type::Class(_) | Type::Var | Type::Array(_) => (
                    self.data.resources.primitives.pointer,
                    Some(vec![0; size_of::<usize>()]),
                ),
            }
        };

        let resolve_property_type = |ty: &Type| -> PropertyType {
            match ty {
                Type::None => PropertyType::Inline(self.data.resources.primitives.none),
                Type::Bool => PropertyType::Inline(self.data.resources.primitives.bool),
                Type::Int => PropertyType::Inline(self.data.resources.primitives.int),
                Type::Float => PropertyType::Inline(self.data.resources.primitives.float),
                Type::String => PropertyType::Pointer(self.data.resources.primitives.string),
                Type::Class(name) | Type::Struct(name) => {
                    let s = self.data.resources.string_to_type.lock();
                    let id = s.get(name).unwrap();
                    PropertyType::Pointer(*id)
                }
                Type::Array(ty) => {
                    // FIXME: For now we only support one level of nesting.
                    // The game currently never supports instantiating nested arrays,
                    // however we should still be able to implement that.
                    let id = match &**ty {
                        Type::None => self.data.resources.primitives.none,
                        Type::Bool => self.data.resources.primitives.bool,
                        Type::Int => self.data.resources.primitives.int,
                        Type::Float => self.data.resources.primitives.float,
                        Type::String => self.data.resources.primitives.string,
                        Type::Class(name) | Type::Struct(name) => {
                            let s = self.data.resources.string_to_type.lock();
                            *s.get(name).unwrap()
                        }
                        Type::Var => todo!(),
                        Type::Array(_) => todo!(),
                    };

                    PropertyType::ArrayPointer(id)
                }
                Type::Var => todo!(),
            }
        };

        let get_size_align = |ty: &Type| -> (usize, usize) {
            match ty {
                Type::None => (0, 1),
                Type::Bool => (1, 1),
                Type::Int => (size_of::<i32>(), align_of::<i32>()),
                Type::Float => (size_of::<f32>(), align_of::<f32>()),
                Type::String | Type::Struct(_) | Type::Class(_) | Type::Var | Type::Array(_) => {
                    (size_of::<usize>(), align_of::<usize>())
                }
            }
        };

        let mut structs = HashMap::new();
        let mut struct_field_offsets = HashMap::new();

        for (name, st) in &mut self.data.structs {
            let id = self.data.resources.types.declare();
            structs.insert(name.to_owned(), id);
            self.data
                .resources
                .string_to_type
                .lock()
                .insert(name.to_owned(), id);

            st.ty = id;
        }

        let mut classes = HashMap::new();
        for (name, class) in &mut self.data.classes {
            let id = self.data.resources.types.declare();
            classes.insert(name.clone(), id);
            self.data
                .resources
                .string_to_type
                .lock()
                .insert(name.to_owned(), id);

            class.type_id = id;
        }

        for (name, st) in &self.data.structs {
            let id = structs[name];

            let mut members = Vec::new();
            let mut offset = 0;
            let mut align = 1;

            let mut field_offsets = HashMap::new();
            let mut properties = HashMap::new();

            for member in &st.s.members {
                let ty_name = Lowercase::new(&*member.ty);
                let ty = self.data.resolve_ty(&ty_name);
                let (member_size, member_align) = get_size_align(&ty);
                let (ty_id, default_value) = resolve_ty_and_default_value(&ty);

                offset = (offset + member_align - 1) & !(member_align - 1);

                let index = members.len();
                properties.insert(
                    (name.to_owned(), Lowercase::new(&*member.name)),
                    TypeProperty {
                        field: index,
                        ty: resolve_property_type(&ty),
                    },
                );

                members.push(CompositeTypeMember {
                    ty: ty_id,
                    offset,
                    default_value,
                    size: member_size,
                });
                field_offsets.insert(Lowercase::new(&*member.name), offset);

                offset += member_size;
                align = align.max(member_align);
            }

            self.data.resources.types.define(
                id,
                resources::Type {
                    repr: TypeRepr::Composite(CompositeType {
                        members,
                        size: offset,
                        align,
                    }),
                    info: TypeInfo {
                        name: Some(name.clone()),
                        classes: vec![id],
                        properties,
                    },
                },
            );

            struct_field_offsets.insert(name.to_owned(), field_offsets);
        }

        for (name, fields) in struct_field_offsets {
            self.data.structs.get_mut(&name).unwrap().field_offsets = fields;
        }

        let mut class_field_offsets: HashMap<
            Lowercase<String>,
            (usize, HashMap<Lowercase<String>, usize>),
        > = HashMap::new();

        let mut queue = self.data.classes.keys().collect::<VecDeque<_>>();
        while let Some(name) = queue.pop_front() {
            let class = &self.data.classes[name];
            let id = classes[name];

            let mut base = if let Some(parent) = &class.parent {
                let parent = classes[parent];
                let Some(parent) = self.data.resources.types.get(parent) else {
                    queue.push_back(name);
                    continue;
                };

                match parent.repr {
                    TypeRepr::Composite(ty) => ty.clone(),
                    _ => unreachable!(),
                }
            } else {
                // First member is for RTTI.
                CompositeType {
                    members: vec![CompositeTypeMember {
                        ty: self.data.resources.primitives.pointer,
                        offset: 0,
                        default_value: None,
                        size: size_of::<usize>(),
                    }],
                    align: align_of::<usize>(),
                    size: size_of::<usize>(),
                }
            };

            let mut class_ids = vec![id];
            let mut next_parent = &class.parent;
            while let Some(parent) = next_parent {
                let class = &self.data.classes[parent];
                let id = classes[parent];

                class_ids.push(id);
                next_parent = &class.parent;
            }

            // TypeIds need to be sorted such that superclasses come before their subclasses.
            class_ids.reverse();

            // This will be set after functions are compiled.
            base.members[0].default_value = Some([0; size_of::<usize>()].to_vec());

            let (first_field_index, mut field_offsets) = if let Some(parent) = &class.parent {
                class_field_offsets.get(parent).unwrap().clone()
            } else {
                // +1 because the first member is the TypeId header.
                (1, HashMap::new())
            };

            let mut new_fields = 0;

            let class_raw = class.class.as_ref().unwrap();
            for var in &class_raw.variables {
                let ty = self.data.resolve_ty(&Lowercase::new(&*var.definition.ty));
                let (member_size, member_align) = get_size_align(&ty);
                let (ty, mut default_value) = resolve_ty_and_default_value(&ty);

                match &var.value {
                    Value::Bool(v) => {
                        default_value = Some(vec![u8::from(*v)]);
                    }
                    Value::Int(v) => {
                        default_value = Some(v.to_ne_bytes().to_vec());
                    }
                    Value::Float(v) => {
                        default_value = Some(v.to_ne_bytes().to_vec());
                    }
                    Value::String(v) => {
                        let ptr = self.data.resources.constants.alloc(v.as_bytes(), 1);

                        let string_ptr = self.data.resources.constants.alloc(
                            ArrayObject {
                                ptr: ptr.cast_mut(),
                                len: v.len(),
                            }
                            .as_bytes(),
                            align_of::<ArrayObject>(),
                        );

                        default_value = Some(string_ptr.expose_provenance().to_ne_bytes().to_vec());
                    }
                    Value::Null | Value::Ident(_) => {}
                };

                base.size = (base.size + member_align - 1) & !(member_align - 1);

                base.members.push(CompositeTypeMember {
                    ty,
                    offset: base.size,
                    default_value,
                    size: member_size,
                });
                field_offsets.insert(Lowercase::new(&*var.definition.name), base.size);
                new_fields += 1;

                base.size += member_size;
                base.align = base.align.max(member_align);
            }

            // Properties
            let mut properties = if let Some(parent) = &class.parent {
                let parent_id = self.data.classes[parent].type_id;

                self.data
                    .resources
                    .types
                    .get(parent_id)
                    .unwrap()
                    .info
                    .properties
            } else {
                HashMap::new()
            };

            for prop in &class_raw.properties {
                // Properties without associated variable are not accessible externally.
                let Some(variable) = &prop.variable else {
                    continue;
                };

                let position = class_raw
                    .variables
                    .iter()
                    .position(|v| v.definition.name.to_lowercase() == variable.to_lowercase())
                    .unwrap();

                let var = &class_raw.variables[position];
                let ty = self.data.resolve_ty(&Lowercase::new(&*var.definition.ty));

                properties.insert(
                    (name.to_owned(), Lowercase::new(&*prop.name)),
                    TypeProperty {
                        field: position + first_field_index,
                        ty: resolve_property_type(&ty),
                    },
                );
            }

            self.data.resources.types.define(
                id,
                resources::Type {
                    repr: TypeRepr::Composite(base),
                    info: TypeInfo {
                        classes: class_ids,
                        properties,
                        name: Some(name.to_owned()),
                    },
                },
            );

            class_field_offsets.insert(
                name.to_owned(),
                (first_field_index + new_fields, field_offsets),
            );
        }

        for (class, _) in classes {
            let (_, fields) = class_field_offsets.remove(&class).unwrap();
            let class = self.data.classes.get_mut(&class).unwrap();
            class.field_offsets = fields;
        }
    }

    fn update_class_states(&mut self) {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut fn_queue = VecDeque::new();

        for class in self.data.classes.keys() {
            queue.push_back(class);
        }

        while let Some(name) = queue.pop_front() {
            if visited.contains(name) {
                continue;
            }

            let class = self.data.classes.get(name).unwrap();
            if let Some(parent) = &class.parent
                && !visited.contains(parent)
            {
                queue.push_front(name);
                queue.push_front(parent);
                continue;
            }

            visited.insert(name);

            for (_, id) in &class.functions {
                fn_queue.push_back(*id);
            }
        }

        let mut create_state_for_class = Vec::new();

        while let Some(id) = fn_queue.pop_front() {
            let func = &self.data.functions[id.0];
            let class = self.data.classes.get(&func.class).unwrap();

            let mut states = HashMap::new();

            // Inherit the states from the parent class function.
            if let Some(parent) = &class.parent {
                let parent = self.data.classes.get(parent).unwrap();
                if let Some(id) = parent.functions.get(&func.name) {
                    states = self.data.functions[id.0]
                        .states
                        .keys()
                        .map(|state_id| (state_id.clone(), FunctionStateImpl::Call(*id)))
                        .collect();
                }
            }

            let pex_class = class.class.as_ref().unwrap();
            for state in &pex_class.states {
                let Some(f) = state
                    .functions
                    .iter()
                    .find(|f| f.name == func.function.name)
                else {
                    continue;
                };

                let id = if state.name == pex_class.default_state {
                    StateId::Default
                } else {
                    StateId::Named(state.name.clone())
                };

                states.insert(id, FunctionStateImpl::Body(f.clone()));
            }

            // If more than one state exists, we must ensure the builtin
            // ::state variable exists for the object.
            if states.len() > 1 {
                create_state_for_class.push(class.name.clone());
            }

            let func = &mut self.data.functions[id.0];
            func.states = states;
        }

        drop(visited);

        for class in create_state_for_class {
            let mut class = self.data.classes.get_mut(&class).unwrap();
            while let Some(parent) = class.parent.clone() {
                class = self.data.classes.get_mut(&parent).unwrap();
            }

            if class.fields.contains_key(Lowercase::from_static("::state")) {
                continue;
            }

            class
                .fields
                .insert(Lowercase::new("::state"), Lowercase::new("string"));
            class.properties.insert(
                Lowercase::new("::state"),
                ClassProperty {
                    variable: Some(Lowercase::new("::state")),
                    getter: None,
                    setter: None,
                },
            );

            // FIXME: We should remove the entire original pex class to avoid
            // needing this double definition.
            class.class.as_mut().unwrap().variables.push(Variable {
                definition: VariableDefinition {
                    name: "::state".to_owned(),
                    ty: "string".to_owned(),
                },
                is_const: false,
                value: Value::String("".to_owned()),
            });
            class.class.as_mut().unwrap().properties.push(Property {
                name: "::state".to_owned(),
                ty: "string".to_owned(),
                variable: Some("::state".to_owned()),
                getter: None,
                setter: None,
            });
        }
    }

    pub fn compile(&mut self, class: &str, options: &Options) -> Result<CompiledClass, Error> {
        let _span = trace_span!("CodegenPass::compile").entered();

        let class_name = Lowercase::new(class);

        self.compute_type_layouts();

        let class = self.data.classes.get(&class_name).unwrap();

        let mut functions = HashMap::new();

        let mut next_class_name = Some(&class.name);
        while let Some(class_name) = next_class_name {
            let class = self.data.get_class(&class_name).unwrap();

            for name in class.functions.keys() {
                // Function was overriden by subclass.
                if functions.contains_key(name) {
                    continue;
                }

                functions.insert(name.clone(), class_name.clone());
            }

            next_class_name = class.parent.as_ref();
        }

        let mut func_ptrs = HashMap::new();
        let mut final_funcs = HashMap::new();

        for (func, class) in functions {
            let (res, ptr) = self.compile_function(&class, &func, options)?;

            func_ptrs.insert(func.clone(), ptr);
            final_funcs.insert(func, res);
        }

        self.build_class_header(&class_name, &func_ptrs);

        Ok(CompiledClass {
            functions: final_funcs,
        })
    }

    fn compile_function(
        &mut self,
        class: &Lowercase<str>,
        func: &Lowercase<str>,
        options: &Options,
    ) -> Result<(CompilationResult, *const u8), Error> {
        let _span = trace_span!("CodegenPass::compile_function").entered();

        let root_func_id = self.data.get_function_id(class, func).unwrap();

        let mut funcs_to_compile = HashSet::<FunctionId>::new();
        let mut queue = vec![root_func_id];
        while let Some(func_id) = queue.pop() {
            let def = &self.data.functions[func_id.0];

            // Function was already compiled.
            if def.func_id.is_some() || funcs_to_compile.contains(&func_id) {
                continue;
            }

            funcs_to_compile.insert(func_id);

            let callees = find_function_callees(&self.data, &def);
            queue.extend(callees);
        }

        for func_id in &funcs_to_compile {
            let func = &self.data.functions[func_id.0];
            let class = &self.data.classes.get(class).unwrap();
            let id = function::declare_function(
                &self.data,
                class,
                func,
                &mut self.compiler,
                options.use_native_stubs,
            )?;

            let func = &mut self.data.functions[func_id.0];
            func.func_id = Some(id);
        }

        for func_id in &funcs_to_compile {
            let func = &self.data.functions[func_id.0];
            let class = &self.data.classes.get(&func.class).unwrap();
            let res = function::translate_function(
                &self.data,
                class,
                func,
                &mut self.compiler,
                self.enable_debug,
            );

            let func = &mut self.data.functions[func_id.0];
            func.signature = Some(res.signature.clone());
            func.psc_signature = Some(res.psc_signature.clone());
            func.debug = res.debug;
        }

        for func_id in &funcs_to_compile {
            let func = &self.data.functions[func_id.0];
            let compiled_func = self.compiler.compile_function(func.func_id.unwrap());
            if let Some(info) = &func.debug {
                super::debug::write_debug_output(info, &compiled_func);
            }
        }

        let func = self.data.get_function(class, func).unwrap();
        let compiled_func = self.compiler.compile_function(func.func_id.unwrap());

        let func = &mut self.data.functions[root_func_id.0];
        let trampoline = trampoline::generate_host_to_guest_trampoline(
            &mut self.compiler,
            func.func_id.unwrap(),
            func.signature.as_ref().unwrap(),
        );
        func.trampoline_func_id = Some(trampoline);

        let compiled_trampoline = self.compiler.compile_function(trampoline);

        Ok((
            CompilationResult {
                function: compiled_trampoline,
                signature: func.signature.clone().unwrap(),
                psc_signature: func.psc_signature.clone().unwrap(),
            },
            compiled_func.as_ptr(),
        ))
    }

    fn build_class_header(
        &mut self,
        class: &Lowercase<String>,
        ptrs: &HashMap<Lowercase<String>, *const u8>,
    ) {
        let mut words = Vec::new();

        let mut class_count = 0;
        let mut next_class_name = Some(class);
        while let Some(class_name) = next_class_name {
            let class = self.data.classes.get(class_name).unwrap();

            let mut vtable = Vec::new();
            for func in class.functions.keys() {
                let ptr = ptrs.get(func).unwrap();
                vtable.push(ptr.expose_provenance());
            }

            let vtable_ptr = self
                .data
                .resources
                .constants
                .alloc(bytemuck::cast_slice(&vtable), align_of::<*const u8>());

            words.push(vtable_ptr.expose_provenance());
            words.push(class.type_id.as_bits());

            class_count += 1;
            next_class_name = class.parent.as_ref();
        }

        words.push(class_count);
        words.reverse();

        let ptr = self
            .data
            .resources
            .constants
            .alloc(bytemuck::cast_slice(&words), align_of::<usize>());
        let ptr = ptr.expose_provenance();

        let class = self.data.classes.get(class).unwrap();

        // Update the "default value" for the class header ptr.
        let mut ty = self.data.resources.types.get(class.type_id).unwrap();
        match &mut ty.repr {
            TypeRepr::Composite(ty) => {
                ty.members[0].default_value = Some(bytemuck::bytes_of(&ptr).to_vec());
            }
            _ => unreachable!(),
        }
        self.data.resources.types.define(class.type_id, ty);
    }

    pub fn compute_function_signature(&self, class: &str, func: &str) -> PscSignature {
        let class = Lowercase::new(&*class);
        let func = Lowercase::new(&*func);
        let func = self.data.get_function(&class, &func).unwrap();
        let sig = function::translate_function_psc_signature(&self.data, func);
        sig
    }
}

#[derive(Clone, Debug)]
struct CodegenData {
    resources: Arc<Resources>,
    classes: HashMap<Lowercase<String>, ClassDef>,
    structs: HashMap<Lowercase<String>, StructDef>,
    functions: Slab<FunctionDef>,
    runtime_funcs: RuntimeFunctions,
}

impl CodegenData {
    fn new(compiler: &mut Compiler) -> Self {
        Self {
            resources: Arc::default(),
            classes: HashMap::new(),
            structs: HashMap::new(),
            functions: Slab::new(),
            runtime_funcs: RuntimeFunctions::new(compiler),
        }
    }

    fn resolve_ty(&self, mut ty: &Lowercase<str>) -> Type {
        let mut array_depth = 0;
        while let Some(p) = ty.strip_suffix("[]") {
            ty = p;
            array_depth += 1;
        }

        let mut typ = match ty.as_inner() {
            "none" => Type::None,
            "bool" => Type::Bool,
            "int" => Type::Int,
            "float" => Type::Float,
            "string" => Type::String,
            "var" => Type::Var,
            _ if self.structs.contains_key(ty) => Type::Struct(ty.to_owned()),
            _ if self.classes.contains_key(ty) => Type::Class(ty.to_owned()),
            _ => unreachable!(),
        };

        for _ in 0..array_depth {
            typ = Type::Array(Box::new(typ));
        }

        typ
    }

    fn resolve_ty_jit(&self, ty: &Lowercase<str>) -> JitType {
        match ty.as_inner() {
            "none" => JitType::I32,
            "bool" => JitType::I8,
            "int" => JitType::I32,
            "float" => JitType::F32,
            "string" => JitType::Pointer,
            "var" => JitType::Pointer,
            _ => JitType::Pointer,
        }
    }

    fn get_class(&self, class: &Lowercase<str>) -> Option<&ClassDef> {
        self.classes.get(class)
    }

    fn get_function_id(&self, class: &Lowercase<str>, func: &Lowercase<str>) -> Option<FunctionId> {
        let class = get_parent_class_with_function(&self.classes, class, func)?;
        self.classes.get(class)?.functions.get(func).copied()
    }

    fn get_function(&self, class: &Lowercase<str>, func: &Lowercase<str>) -> Option<&FunctionDef> {
        let id = self.get_function_id(class, func)?;
        Some(&self.functions[id.0])
    }

    fn get_function_mut(
        &mut self,
        class: &Lowercase<str>,
        func: &Lowercase<str>,
    ) -> Option<&mut FunctionDef> {
        let id = self.get_function_id(class, func)?;
        Some(&mut self.functions[id.0])
    }

    fn get_class_property(
        &self,
        class: &Lowercase<str>,
        property: &Lowercase<str>,
    ) -> Option<&ClassProperty> {
        let class = get_parent_class_with_property(&self.classes, class, property)?;
        let class = self.classes.get(class)?;
        class.properties.get(property)
    }

    fn get_property_getter_id(
        &self,
        class: &Lowercase<str>,
        property: &Lowercase<str>,
    ) -> Option<FunctionId> {
        let class = get_parent_class_with_property(&self.classes, class, property)?;
        let class = self.classes.get(class)?;
        let prop = class.properties.get(property)?;
        prop.getter
    }

    fn get_property_setter_id(
        &self,
        class: &Lowercase<str>,
        property: &Lowercase<str>,
    ) -> Option<FunctionId> {
        let class = get_parent_class_with_property(&self.classes, class, property)?;
        let class = self.classes.get(class)?;
        let prop = class.properties.get(property)?;
        prop.setter
    }
}

#[derive(Clone, Debug, Default)]
struct ClassDef {
    name: Lowercase<String>,
    functions: IndexMap<Lowercase<String>, FunctionId>,
    parent: Option<Lowercase<String>>,
    fields: HashMap<Lowercase<String>, Lowercase<String>>,
    /// Unique RTTI number stored in the header to identify objects of this class.
    type_id: TypeId,
    field_offsets: HashMap<Lowercase<String>, usize>,
    properties: HashMap<Lowercase<String>, ClassProperty>,
    class: Option<Class>,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    name: Lowercase<String>,
    class: Lowercase<String>,
    class_name_debug: String,
    signature: Option<Signature>,
    psc_signature: Option<PscSignature>,
    func_id: Option<FuncId>,
    function: Function,
    native: Option<NativeFunction>,
    trampoline_func_id: Option<FuncId>,
    debug: Option<Box<DebugInfo>>,
    states: HashMap<StateId, FunctionStateImpl>,
}

#[derive(Clone, Debug)]
struct ClassProperty {
    pub variable: Option<Lowercase<String>>,
    pub getter: Option<FunctionId>,
    pub setter: Option<FunctionId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(usize);

#[derive(Clone, Debug)]
struct StructDef {
    pub s: Struct,
    pub ty: TypeId,
    pub field_offsets: HashMap<Lowercase<String>, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum StateId {
    Default,
    Named(String),
}

#[derive(Clone, Debug)]
enum FunctionStateImpl {
    Call(FunctionId),
    Body(Function),
}

fn find_function_callees(data: &CodegenData, function: &FunctionDef) -> Vec<FunctionId> {
    let mut callees = Vec::new();

    let class = function.class.as_str();
    let current_class = data.classes.get(class).unwrap();

    let find_var = |var_name: &Lowercase<str>| -> Lowercase<String> {
        match var_name.as_inner() {
            "self" => return class.to_owned(),
            _ => (),
        }

        for param in &function.function.params {
            let name = Lowercase::new(param.name.as_str());
            if name == *var_name {
                return Lowercase::new(param.ty.as_str());
            }
        }

        for local in &function.function.locals {
            let name = Lowercase::new(local.name.as_str());
            if name == *var_name {
                return Lowercase::new(local.ty.as_str());
            }
        }

        let mut class = data.get_class(&class).unwrap();
        loop {
            if let Some(ty) = class.fields.get(var_name) {
                return ty.clone();
            }

            if let Some(parent) = &class.parent {
                class = data.get_class(&parent).unwrap();
            } else {
                break;
            }
        }

        todo!()
    };

    for (_, state) in &function.states {
        match state {
            FunctionStateImpl::Body(body) => {
                for instruction in &body.instructions {
                    match instruction {
                        Instruction::CallParent(op) => {
                            let parent = current_class.parent.as_ref().unwrap();

                            let func_name = Lowercase::new(op.function.as_str());

                            let func = data.get_function_id(parent, &func_name).unwrap();
                            callees.push(func);
                        }
                        Instruction::CallStatic(op) => {
                            let class_name = Lowercase::new(op.class.as_str());
                            let function_name = Lowercase::new(op.function.as_str());

                            let func = data.get_function_id(&class_name, &function_name).unwrap();

                            callees.push(func);
                        }
                        Instruction::PropertyGet(op) => {
                            let Value::Ident(object) = &op.value else {
                                unreachable!()
                            };

                            let object = Lowercase::new(object.as_str());
                            let class_name = find_var(&object);
                            let prop_name = Lowercase::new(op.property.as_str());

                            if let Some(func) = data.get_property_getter_id(&class_name, &prop_name)
                            {
                                callees.push(func);
                            }
                        }
                        Instruction::PropertySet(op) => {
                            let Value::Ident(object) = &op.object else {
                                unreachable!()
                            };

                            let object = Lowercase::new(object.as_str());
                            let class_name = find_var(&object);
                            let prop_name = Lowercase::new(op.property.as_str());

                            if let Some(func) = data.get_property_setter_id(&class_name, &prop_name)
                            {
                                callees.push(func);
                            }
                        }
                        _ => {}
                    }
                }
            }
            FunctionStateImpl::Call(id) => {
                callees.push(*id);
            }
        }
    }

    callees.dedup();
    callees
}

fn get_parent_class_with_function<'a>(
    classes: &'a HashMap<Lowercase<String>, ClassDef>,
    class_name: &Lowercase<str>,
    func_name: &Lowercase<str>,
) -> Option<&'a Lowercase<str>> {
    let class = classes.get(class_name)?;
    let mut class_name = class.name.as_str();

    loop {
        let class = classes.get(class_name)?;
        class_name = class.name.as_str();

        if class.functions.get(func_name).is_some() {
            return Some(class_name);
        }

        let Some(parent) = &class.parent else {
            return None;
        };

        class_name = parent.as_str();
    }
}

fn get_parent_class_with_property<'a>(
    classes: &'a HashMap<Lowercase<String>, ClassDef>,
    class_name: &Lowercase<str>,
    prop_name: &Lowercase<str>,
) -> Option<&'a Lowercase<str>> {
    let class = classes.get(class_name)?;
    let mut class_name = class.name.as_str();

    loop {
        let class = classes.get(class_name)?;
        class_name = class.name.as_str();

        if class.properties.get(prop_name).is_some() {
            return Some(class_name);
        }

        let Some(parent) = &class.parent else {
            return None;
        };

        class_name = parent.as_str();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Type {
    None,
    Bool,
    Int,
    Float,
    String,
    Var,
    Array(Box<Type>),
    Class(Lowercase<String>),
    Struct(Lowercase<String>),
}

#[derive(Clone, Debug, Default)]
pub struct PscSignature {
    pub params: Vec<TypeId>,
    pub returns: Vec<TypeId>,
}
