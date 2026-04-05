mod builtin;
mod constant;
mod function;
mod function_props;
mod runtime;
mod trampoline;

use std::collections::VecDeque;
use std::sync::Arc;

use game_common::string::Lowercase;
use game_tracing::trace_span;
use hashbrown::{HashMap, HashSet};
use openfo4_jit::{Compiler, FuncId, Signature, Type as JitType};
use openfo4_pex::{Class, Function, Instruction, Module, Struct, Value};
use slab::Slab;
use thiserror::Error;

use crate::compiler::codegen::constant::ConstantData;
use crate::compiler::codegen::function_props::{FunctionProps, function_properties};
// use crate::compiler::codegen::layout::{ClassLayout, compute_struct_layout};
use crate::compiler::codegen::runtime::RuntimeFunctions;
use crate::compiler::resources::{
    self, CompositeType, CompositeTypeMember, Name, PropertyType, Resources, TypeId, TypeInfo,
    TypeProperty,
};
use crate::compiler::{CompilationResult, NativeFunction};
use crate::engine::Options;

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
}

impl CodegenPass {
    pub fn new() -> Self {
        let mut compiler = Compiler::new();

        Self {
            data: CodegenData::new(&mut compiler),
            compiler,
            classes_updated: false,
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

            // FIXME: Handle multi states
            let mut functions = HashMap::new();
            for state in &class.states {
                for func in &state.functions {
                    let id = FunctionId(self.data.functions.insert(FunctionDef {
                        signature: None,
                        func_id: None,
                        function: func.clone(),
                        native: None,
                        trampoline_func_id: None,
                        props: None,
                        constants: Arc::default(),
                        class: Lowercase::new(&*class.name),
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
                    FunctionId(self.data.functions.insert(FunctionDef {
                        signature: None,
                        func_id: None,
                        function: func.clone(),
                        native: None,
                        trampoline_func_id: None,
                        props: None,
                        constants: Arc::default(),
                        class: Lowercase::new(&*class.name),
                    }))
                });

                let setter = prop.setter.as_ref().map(|func| {
                    FunctionId(self.data.functions.insert(FunctionDef {
                        signature: None,
                        func_id: None,
                        function: func.clone(),
                        native: None,
                        trampoline_func_id: None,
                        props: None,
                        constants: Arc::default(),
                        class: Lowercase::new(&*class.name),
                    }))
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

        let resolve_ty_and_default_value = |ty: &Type| -> (TypeId, Option<Vec<u8>>) {
            match ty {
                Type::None => (self.data.resources.primitives.none, None),
                Type::Bool => (self.data.resources.primitives.bool, Some(vec![0; 4])),
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
                Type::Bool => (4, 4),
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
                    name: Some(Name::Struct {
                        field: Lowercase::new(&*member.name),
                    }),
                    default_value,
                    size: member_size,
                });
                field_offsets.insert(Lowercase::new(&*member.name), offset);

                offset += member_size;
                align = align.max(member_align);
            }

            self.data.resources.types.define(
                id,
                resources::Type::Composite(CompositeType {
                    members,
                    size: offset,
                    align,
                }),
            );

            self.data.resources.infos.lock().insert(
                id,
                TypeInfo {
                    classes: Vec::new(),
                    properties,
                    name: Some(name.to_owned()),
                },
            );

            struct_field_offsets.insert(name.to_owned(), field_offsets);
        }

        for (name, fields) in struct_field_offsets {
            self.data.structs.get_mut(&name).unwrap().field_offsets = fields;
        }

        let mut class_field_offsets: HashMap<Lowercase<String>, HashMap<Lowercase<String>, usize>> =
            HashMap::new();

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

                match parent {
                    resources::Type::Composite(ty) => ty.clone(),
                    _ => unreachable!(),
                }
            } else {
                // First member is for RTTI.
                CompositeType {
                    members: vec![CompositeTypeMember {
                        ty: self.data.resources.primitives.pointer,
                        offset: 0,
                        name: None,
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

            let header_ptr = self
                .data
                .resources
                .constants
                .alloc(bytemuck::cast_slice(&class_ids), align_of::<usize>());

            base.members[0].default_value =
                Some(header_ptr.expose_provenance().to_ne_bytes().to_vec());

            let mut field_offsets = if let Some(parent) = &class.parent {
                class_field_offsets.get(parent).unwrap().clone()
            } else {
                HashMap::new()
            };
            // +1 because the first member is the TypeId header.
            let first_field_index = field_offsets.len() + 1;

            let class_raw = class.class.as_ref().unwrap();
            for var in &class_raw.variables {
                let ty = self.data.resolve_ty(&Lowercase::new(&*var.definition.ty));
                let (member_size, member_align) = get_size_align(&ty);
                let (ty, mut default_value) = resolve_ty_and_default_value(&ty);

                match &var.value {
                    Value::Bool(v) => {
                        default_value = Some(u32::from(*v).to_ne_bytes().to_vec());
                    }
                    Value::Int(v) => {
                        default_value = Some(v.to_ne_bytes().to_vec());
                    }
                    Value::Float(v) => {
                        default_value = Some(v.to_ne_bytes().to_vec());
                    }
                    Value::String(v) => {
                        let ptr = self.data.resources.constants.alloc(v.as_bytes(), 1);
                        default_value = Some(ptr.expose_provenance().to_ne_bytes().to_vec());
                    }
                    Value::Null | Value::Ident(_) => {}
                };

                base.size = (base.size + member_align - 1) & !(member_align - 1);

                base.members.push(CompositeTypeMember {
                    ty,
                    offset: base.size,
                    name: Some(Name::Class {
                        class: Lowercase::new(&*class_raw.name),
                        field: Lowercase::new(&*var.definition.name),
                    }),
                    default_value,
                    size: member_size,
                });
                field_offsets.insert(Lowercase::new(&*var.definition.name), base.size);

                base.size += member_size;
                base.align = base.align.max(member_align);
            }

            self.data
                .resources
                .types
                .define(id, resources::Type::Composite(base));

            class_field_offsets.insert(name.to_owned(), field_offsets);

            // Properties
            let mut properties = if let Some(parent) = &class.parent {
                let parent_id = self.data.classes[parent].type_id;

                self.data
                    .resources
                    .infos
                    .lock()
                    .get(&parent_id)
                    .unwrap()
                    .clone()
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

            self.data.resources.infos.lock().insert(
                id,
                TypeInfo {
                    classes: class_ids,
                    properties,
                    name: Some(name.to_owned()),
                },
            );
        }

        for (class, _) in classes {
            let fields = class_field_offsets.remove(&class).unwrap();
            let class = self.data.classes.get_mut(&class).unwrap();
            class.field_offsets = fields;
        }
    }

    pub fn compile_fn(
        &mut self,
        class: &str,
        f: &str,
        options: &Options,
    ) -> Result<CompilationResult, Error> {
        let _span = trace_span!("CodegenPass::compile_function").entered();

        let class = Lowercase::new(class);
        let f = Lowercase::new(f);

        self.compute_type_layouts();

        let root_func_id = self.data.get_function_id(&class, &f).unwrap();

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
            let class = &self.data.classes.get(&class).unwrap();
            let id = function::declare_function(
                &self.data,
                class,
                func,
                &mut self.compiler,
                options.use_native_stubs,
            )?;

            let props = function_properties(&self.data, func);

            let func = &mut self.data.functions[func_id.0];
            func.func_id = Some(id);
            func.props = Some(props);
        }

        for func_id in &funcs_to_compile {
            let func = &self.data.functions[func_id.0];
            let class = &self.data.classes.get(&func.class).unwrap();
            let res = function::translate_function(&self.data, class, func, &mut self.compiler);

            // TODO: FIXME: Don't leak this here.
            // Constants must be kept alive until all function instances are dropped.
            // NOTE THAT THIS MUST INCLUDE ALL CALLED FUNCTIONS RECURSIVELY.
            core::mem::forget(res.constants);

            let func = &mut self.data.functions[func_id.0];
            func.signature = Some(res.signature.clone());
        }

        let func = &mut self.data.functions[root_func_id.0];
        let trampoline = trampoline::generate_host_to_guest_trampoline(
            &mut self.compiler,
            func.func_id.unwrap(),
            func.signature.as_ref().unwrap(),
        );
        func.trampoline_func_id = Some(trampoline);

        let props = func.props.as_ref().unwrap();

        let compiled = self.compiler.compile_function(trampoline);

        Ok(CompilationResult {
            function: compiled,
            stack_size: props.local_stack_size,
            signature: func.signature.clone().unwrap(),
        })
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
            "bool" => JitType::I32,
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
    functions: HashMap<Lowercase<String>, FunctionId>,
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
    class: Lowercase<String>,
    signature: Option<Signature>,
    func_id: Option<FuncId>,
    function: Function,
    native: Option<NativeFunction>,
    trampoline_func_id: Option<FuncId>,
    props: Option<FunctionProps>,
    constants: Arc<Vec<ConstantData>>,
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

    for instruction in &function.function.instructions {
        match instruction {
            Instruction::CallMethod(op) => {
                let Value::Ident(object) = &op.object else {
                    continue;
                };

                let object = Lowercase::new(object.as_str());
                let class_name = find_var(&object);
                let function_name = match &op.function_name {
                    Value::Ident(v) => v,
                    _ => unreachable!(),
                };

                let function_name = Lowercase::new(function_name.as_str());

                let func = data.get_function_id(&class_name, &function_name).unwrap();

                callees.push(func);
            }
            Instruction::CallParent(op) => {
                let parent = current_class.parent.as_ref().unwrap();

                let Value::Ident(func_name) = &op.function else {
                    unreachable!();
                };
                let func_name = Lowercase::new(func_name.as_str());

                let func = data.get_function_id(parent, &func_name).unwrap();
                callees.push(func);
            }
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(a), Value::Ident(b)) => (a, b),
                    _ => unreachable!(),
                };
                let class_name = Lowercase::new(class_name.as_str());
                let function_name = Lowercase::new(function_name.as_str());

                let func = data.get_function_id(&class_name, &function_name).unwrap();

                callees.push(func);
            }
            Instruction::PropertyGet(op) => {
                let Value::Ident(object) = &op.value else {
                    unreachable!()
                };

                let Value::Ident(prop_name) = &op.property else {
                    unreachable!()
                };

                let object = Lowercase::new(object.as_str());
                let class_name = find_var(&object);
                let prop_name = Lowercase::new(prop_name.as_str());

                if let Some(func) = data.get_property_getter_id(&class_name, &prop_name) {
                    callees.push(func);
                }
            }
            Instruction::PropertySet(op) => {
                let Value::Ident(object) = &op.object else {
                    unreachable!()
                };

                let Value::Ident(prop_name) = &op.property else {
                    unreachable!()
                };

                let object = Lowercase::new(object.as_str());
                let class_name = find_var(&object);
                let prop_name = Lowercase::new(prop_name.as_str());

                if let Some(func) = data.get_property_setter_id(&class_name, &prop_name) {
                    callees.push(func);
                }
            }
            _ => {}
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
