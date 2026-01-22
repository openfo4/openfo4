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

pub const MAX_INSTRUCTIONS_BEFORE_PREEMPT: usize = 1024;

#[derive(Clone, Debug, Error)]
#[error("codegen error")]
pub struct Error;

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
            let parent = (!matches!(class.parent.to_lowercase().as_str(), "" | "none"))
                .then(|| class.parent.clone());

            let mut fields = HashMap::new();
            for var in &class.variables {
                fields.insert(
                    var.definition.name.to_lowercase(),
                    var.definition.ty.to_lowercase(),
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
                        class: class.name.to_lowercase(),
                    }));

                    functions.insert(func.name.to_lowercase(), id);
                }
            }

            for st in &class.structs {
                let name = format!("{}#{}", class.name, st.name);
                self.data
                    .structs
                    .insert(name.to_lowercase(), StructDef { s: st.clone() });
            }

            let mut properties = HashMap::new();
            for prop in &class.properties {
                let variable = prop.variable.as_ref().map(|v| v.to_lowercase());

                let getter = prop.getter.as_ref().map(|func| {
                    FunctionId(self.data.functions.insert(FunctionDef {
                        signature: None,
                        func_id: None,
                        function: func.clone(),
                        native: None,
                        trampoline_func_id: None,
                        props: None,
                        constants: Arc::default(),
                        class: class.name.to_lowercase(),
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
                        class: class.name.to_lowercase(),
                    }))
                });

                properties.insert(
                    prop.name.to_lowercase(),
                    ClassProperty {
                        variable,
                        getter,
                        setter,
                    },
                );
            }

            self.data.classes.insert(
                class.name.to_lowercase(),
                ClassDef {
                    name: class.name.to_lowercase(),
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
        let Some(func_def) = self.data.get_function_mut(class, function) else {
            panic!("unknown native function: {class}.{function}");
        };

        func_def.native = Some(func);
    }

    pub fn compute_type_layouts(&mut self) {
        let mut queue = Vec::new();
        let mut dependencies = HashMap::<_, Vec<_>>::new();

        for class in self.data.classes.values() {
            let Some(parent) = &class.parent else {
                queue.push(class.name.to_lowercase());
                continue;
            };

            dependencies
                .entry(parent.to_lowercase())
                .or_default()
                .push(class.name.to_lowercase());
        }

        while let Some(class_name) = queue.pop() {
            let class = self.data.classes.get(&class_name.to_lowercase()).unwrap();
            let layout = layout::compute_class_layout(&self.data, &class);
            let class = self
                .data
                .classes
                .get_mut(&class_name.to_lowercase())
                .unwrap();
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
                let class = self.data.get_class(class_name).unwrap();
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
            let class = self.data.classes.get_mut(&class.to_lowercase()).unwrap();
            class.header = Some(header);
        }
    }

    pub fn compute_class_layouts(
        &mut self,
        class: &str,
    ) -> Result<HashMap<String, TypeLayout>, Error> {
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
                        class: class_name.to_lowercase(),
                        name: field_name.to_lowercase(),
                        offset: *field_offset,
                        ty: match field_ty {
                            Type::Bool => FieldType::Bool,
                            Type::Int => FieldType::I32,
                            Type::Float => FieldType::F32,
                            _ => FieldType::Pointer,
                        },
                        ty_name: field_ty_name.to_lowercase(),
                    });
                }

                for (name, prop) in &class.properties {
                    let Some(field) = class_fields.iter().position(|v| {
                        v.class.to_lowercase() == class.name.to_lowercase()
                            && prop
                                .variable
                                .as_ref()
                                .is_some_and(|pv| pv.to_lowercase() == v.name.to_lowercase())
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
                class.name.to_lowercase(),
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
                        .find(|f| f.name.to_lowercase() == field_name.to_lowercase())
                        .unwrap();
                let field_ty = self.data.resolve_ty(&member.ty);

                let var = fields.len();
                fields.push(ObjectField {
                    class: "".to_owned(),
                    name: field_name.to_lowercase(),
                    offset: *field_offset,
                    ty: match field_ty {
                        Type::Bool => FieldType::Bool,
                        Type::Int => FieldType::I32,
                        Type::Float => FieldType::F32,
                        _ => FieldType::Pointer,
                    },
                    ty_name: member.ty.to_lowercase(),
                });

                properties.push(ObjectProperty {
                    class: "".to_owned(),
                    name: field_name.clone(),
                    var,
                });
            }

            class_layouts.insert(
                name.to_lowercase(),
                TypeLayout {
                    header: None,
                    fields,
                    properties,
                },
            );
        }

        Ok(class_layouts)
    }

    pub fn compile_fn(&mut self, class: &str, f: &str) -> Result<CompilationResult, Error> {
        self.compute_type_layouts();

        let root_func_id = self.data.get_function_id(class, f).unwrap();

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

        dbg!(&funcs_to_compile);

        for func_id in &funcs_to_compile {
            let func = &self.data.functions[func_id.0];
            let class = &self.data.classes.get(&class.to_lowercase()).unwrap();
            let id = function::declare_function(&self.data, class, func, &mut self.compiler);
            dbg!(&id);

            let props = function_properties(&self.data, func);

            let func = &mut self.data.functions[func_id.0];
            func.func_id = Some(id);
            func.props = Some(props);
        }

        for func_id in &funcs_to_compile {
            let func = &self.data.functions[func_id.0];
            let class = &self.data.classes.get(&class.to_lowercase()).unwrap();
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
    classes: HashMap<String, ClassDef>,
    structs: HashMap<String, StructDef>,
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

    fn resolve_ty(&self, mut ty: &str) -> Type {
        let mut array_depth = 0;
        while let Some(p) = ty.strip_suffix("[]") {
            ty = p;
            array_depth += 1;
        }

        let mut typ = match &*ty.to_lowercase() {
            "none" => Type::None,
            "bool" => Type::Bool,
            "int" => Type::Int,
            "float" => Type::Float,
            "string" => Type::String,
            "var" => Type::Var,
            name if self.structs.contains_key(name) => Type::Struct(name.to_owned()),
            name if self.classes.contains_key(name) => Type::Class(name.to_owned()),
            _ => unreachable!(),
        };

        for _ in 0..array_depth {
            typ = Type::Array(Box::new(typ));
        }

        typ
    }

    fn resolve_ty_jit(&self, ty: &str) -> JitType {
        match &*ty.to_lowercase() {
            "none" => JitType::I32,
            "bool" => JitType::I32,
            "int" => JitType::I32,
            "float" => JitType::F32,
            "string" => JitType::Pointer,
            "var" => JitType::Pointer,
            _ => JitType::Pointer,
        }
    }

    fn get_class(&self, class: &str) -> Option<&ClassDef> {
        self.classes.get(&class.to_lowercase())
    }

    fn get_function_id(&self, class: &str, func: &str) -> Option<FunctionId> {
        let class = get_parent_class_with_function(&self.classes, class, func)?;
        self.classes
            .get(&class)?
            .functions
            .get(&func.to_lowercase())
            .copied()
    }

    fn get_function(&self, class: &str, func: &str) -> Option<&FunctionDef> {
        let id = self.get_function_id(class, func)?;
        Some(&self.functions[id.0])
    }

    fn get_function_mut(&mut self, class: &str, func: &str) -> Option<&mut FunctionDef> {
        let id = self.get_function_id(class, func)?;
        Some(&mut self.functions[id.0])
    }

    fn get_property_getter_id(&self, class: &str, property: &str) -> Option<FunctionId> {
        let class = self.classes.get(&class.to_lowercase())?;
        let prop = class.properties.get(&property.to_lowercase())?;
        prop.getter
    }

    fn get_property_setter_id(&self, class: &str, property: &str) -> Option<FunctionId> {
        let class = self.classes.get(&class.to_lowercase())?;
        let prop = class.properties.get(&property.to_lowercase())?;
        prop.setter
    }
}

#[derive(Clone, Debug, Default)]
struct ClassDef {
    name: String,
    functions: HashMap<String, FunctionId>,
    parent: Option<String>,
    fields: HashMap<String, String>,
    /// Unique RTTI number stored in the header to identify objects of this class.
    type_id: usize,
    layout: Option<ClassLayout>,
    header: Option<Arc<[usize]>>,
    properties: HashMap<String, ClassProperty>,
    class: Option<Class>,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    class: String,
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
    pub variable: Option<String>,
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

    let class = function.class.to_lowercase();
    let current_class = data.classes.get(&class).unwrap();

    let find_var = |var_name: &str| -> String {
        match &*var_name.to_lowercase() {
            "self" => return class.clone(),
            _ => (),
        }

        for param in &function.function.params {
            if param.name.to_lowercase() == var_name.to_lowercase() {
                return param.ty.to_lowercase();
            }
        }

        for local in &function.function.locals {
            if local.name.to_lowercase() == var_name.to_lowercase() {
                return local.ty.to_lowercase();
            }
        }

        let mut class = data.get_class(&class).unwrap();
        loop {
            if let Some(ty) = class.fields.get(&var_name.to_lowercase()) {
                return ty.to_lowercase();
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

                let class_name = find_var(&object);
                let function_name = match &op.function_name {
                    Value::Ident(v) => v,
                    _ => unreachable!(),
                };

                let func = data.get_function_id(&class_name, &function_name).unwrap();

                callees.push(func);
            }
            Instruction::CallParent(op) => {}
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(a), Value::Ident(b)) => (a, b),
                    _ => unreachable!(),
                };

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

                let class_name = find_var(&object);

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

                let class_name = find_var(&object);

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
    classes: &'a HashMap<String, ClassDef>,
    class_name: &str,
    func_name: &str,
) -> Option<String> {
    let mut class_name = class_name.to_lowercase();

    loop {
        let class = classes.get(&class_name)?;

        if class.functions.get(&func_name.to_lowercase()).is_some() {
            return Some(class_name);
        }

        let Some(parent) = &class.parent else {
            return None;
        };

        class_name = parent.to_lowercase();
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
    Class(String),
    Struct(String),
}
