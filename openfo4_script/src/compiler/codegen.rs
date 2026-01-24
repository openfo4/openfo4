mod builtin;
mod constant;
mod function;
mod function_props;
mod layout;
mod runtime;
mod trampoline;
mod types;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use game_common::string::Lowercase;
use openfo4_jit::{Compiler, FuncId, Signature, Type as JitType};
use openfo4_pex::{Class, Function, Instruction, Module, Struct, Value};
use slab::Slab;
use thiserror::Error;

use crate::compiler::codegen::constant::ConstantData;
use crate::compiler::codegen::function_props::{FunctionProps, function_properties};
use crate::compiler::codegen::layout::{ClassLayout, compute_struct_layout};
use crate::compiler::codegen::runtime::RuntimeFunctions;
use crate::compiler::codegen::types::Types;
use crate::compiler::{
    CompilationResult, FieldType, NativeFunction, ObjectField, ObjectProperty, TypeLayout,
};
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
    next_type_id: usize,
    compiler: Compiler,
}

impl CodegenPass {
    pub fn new() -> Self {
        let mut compiler = Compiler::new();

        Self {
            data: CodegenData::new(&mut compiler),
            next_type_id: 0,
            compiler,
        }
    }

    pub fn register_module(&mut self, module: &Module) {
        for class in &module.classes {
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
                self.data
                    .structs
                    .insert(Lowercase::new(&*name), StructDef { s: st.clone() });
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
                    type_id: self.next_type_id,
                    layout: None,
                    header: None,
                    properties,
                    class: Some(class.clone()),
                },
            );
            self.next_type_id += 1;
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
        let mut queue = Vec::new();
        let mut dependencies = HashMap::<_, Vec<_>>::new();

        for class in self.data.classes.values() {
            let Some(parent) = &class.parent else {
                queue.push(class.name.clone());
                continue;
            };

            dependencies
                .entry(parent.clone())
                .or_default()
                .push(class.name.clone());
        }

        while let Some(class_name) = queue.pop() {
            let class = self.data.classes.get(&class_name).unwrap();
            let layout = layout::compute_class_layout(&self.data, &class);
            let class = self.data.classes.get_mut(&class_name).unwrap();
            class.layout = Some(layout);

            if let Some(children) = dependencies.remove(&class_name) {
                queue.extend(children);
            }
        }

        let mut class_headers = Vec::new();

        // Create class header
        for class in self.data.classes.values() {
            if class.header.is_some() {
                continue;
            }

            let mut type_ids = Vec::new();

            let mut next_class = Some(&class.name);
            while let Some(class_name) = next_class {
                let class = self.data.get_class(class_name.as_str()).unwrap();
                type_ids.push(class.type_id);
                next_class = class.parent.as_ref();
            }

            type_ids.push(type_ids.len());
            // Reverse such that the root class ID is at the start and the leaf is at the bottom.
            type_ids.reverse();

            let header: Arc<[usize]> = type_ids.into_boxed_slice().into();
            class_headers.push((class.name.clone(), header));
        }

        for (class, header) in class_headers {
            let class = self.data.classes.get_mut(&class).unwrap();
            class.header = Some(header);
        }
    }

    pub fn compute_class_layouts(
        &mut self,
        class: &str,
    ) -> Result<HashMap<Lowercase<String>, TypeLayout>, Error> {
        self.compute_type_layouts();

        let mut class_layouts = HashMap::new();
        for class in self.data.classes.values() {
            let mut class_fields = Vec::new();
            let mut properties = Vec::new();

            let mut next_class = Some(&class.name);
            while let Some(class_name) = next_class.take() {
                let class = self.data.get_class(class_name).unwrap();
                let layout = class.layout.as_ref().unwrap();

                for (field_name, field_offset) in &layout.field_offsets {
                    let field_ty_name = class.fields.get(field_name).unwrap();
                    let field_ty = self.data.resolve_ty(&field_ty_name);

                    class_fields.push(ObjectField {
                        class: class_name.clone(),
                        name: field_name.clone(),
                        offset: *field_offset,
                        ty: match field_ty {
                            Type::Bool => FieldType::Bool,
                            Type::Int => FieldType::I32,
                            Type::Float => FieldType::F32,
                            _ => FieldType::Pointer,
                        },
                        ty_name: field_ty_name.clone(),
                    });
                }

                for (name, prop) in &class.properties {
                    let Some(field) = class_fields.iter().position(|v| {
                        v.class == class.name
                            && prop.variable.as_ref().is_some_and(|pv| pv == &v.name)
                    }) else {
                        // Properties without a backing variable cannot be accessed
                        // externally.
                        continue;
                    };

                    properties.push(ObjectProperty {
                        class: class.name.clone(),
                        name: name.clone(),
                        var: field,
                    });
                }

                next_class = class.parent.as_ref();
            }

            class_layouts.insert(
                class.name.clone(),
                TypeLayout {
                    header: class.header.clone(),
                    fields: class_fields,
                    properties,
                },
            );
        }

        for (name, st) in &self.data.structs {
            let layout = st.layout(&self.data);

            let mut fields = Vec::new();
            let mut properties = Vec::new();
            for (field_name, field_offset) in &layout.field_offsets {
                let member =
                    st.s.members
                        .iter()
                        .find(|f| &Lowercase::new(f.name.as_str()) == field_name)
                        .unwrap();
                let member_ty = Lowercase::new(member.ty.as_str());
                let field_ty = self.data.resolve_ty(&member_ty);

                let var = fields.len();
                fields.push(ObjectField {
                    class: Lowercase::new(""),
                    name: field_name.clone(),
                    offset: *field_offset,
                    ty: match field_ty {
                        Type::Bool => FieldType::Bool,
                        Type::Int => FieldType::I32,
                        Type::Float => FieldType::F32,
                        _ => FieldType::Pointer,
                    },
                    ty_name: member_ty,
                });

                properties.push(ObjectProperty {
                    class: Lowercase::new(""),
                    name: field_name.clone(),
                    var,
                });
            }

            class_layouts.insert(
                name.clone(),
                TypeLayout {
                    header: None,
                    fields,
                    properties,
                },
            );
        }

        Ok(class_layouts)
    }

    pub fn compile_fn(
        &mut self,
        class: &str,
        f: &str,
        options: &Options,
    ) -> Result<CompilationResult, Error> {
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
            core::mem::forget(res.layouts);

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
    types: Types,
    classes: HashMap<Lowercase<String>, ClassDef>,
    structs: HashMap<Lowercase<String>, StructDef>,
    functions: Slab<FunctionDef>,
    runtime_funcs: RuntimeFunctions,
}

impl CodegenData {
    fn new(compiler: &mut Compiler) -> Self {
        Self {
            types: Types::default(),
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
    type_id: usize,
    layout: Option<ClassLayout>,
    header: Option<Arc<[usize]>>,
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
}

impl StructDef {
    fn layout(&self, data: &CodegenData) -> ClassLayout {
        compute_struct_layout(data, self)
    }
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
