mod builtin;
mod function;
mod function_props;
mod layout;
mod trampoline;
mod types;

use std::collections::{HashMap, HashSet};

use openfo4_jit::{Compiler, FunctionId, Signature, Type as JitType};
use openfo4_pex::{Function, Instruction, Module, Struct, Value};
use thiserror::Error;

use crate::compiler::codegen::function::FunctionResult;
use crate::compiler::codegen::function_props::{FunctionProps, function_properties};
use crate::compiler::codegen::layout::ClassLayout;
use crate::compiler::codegen::types::Types;
use crate::compiler::{ClassFieldLayout, CompilationResult, FieldType, NativeFunction};

pub const MAX_INSTRUCTIONS_BEFORE_PREEMPT: usize = 1024;

#[derive(Clone, Debug, Error)]
#[error("codegen error")]
pub struct Error;

#[derive(Clone, Debug)]
pub struct CodegenPass {
    data: CodegenData,
    next_type_id: usize,
}

impl CodegenPass {
    pub fn new() -> Self {
        Self {
            data: CodegenData::new(),
            next_type_id: 0,
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
                    functions.insert(
                        func.name.to_lowercase(),
                        FunctionDef {
                            signature: None,
                            func_id: None,
                            function: func.clone(),
                            native: None,
                            trampoline_func_id: None,
                            props: None,
                        },
                    );
                }
            }

            for st in &class.structs {
                let name = format!("{}#{}", class.name, st.name);
                self.data.structs.insert(name.to_lowercase(), st.clone());
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
                },
            );
            self.next_type_id += 1;
        }
    }

    pub fn define_native_function(&mut self, class: &str, function: &str, func: NativeFunction) {
        let def = self.data.classes.get_mut(&class.to_lowercase()).unwrap();
        let func_def = def.functions.get_mut(&function.to_lowercase()).unwrap();
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
    }

    pub fn compile_fn(&mut self, class: &str, f: &str) -> Result<CompilationResult, Error> {
        self.compute_type_layouts();

        let mut compiler = Compiler::new();

        // The compile the function we must also compile all functions that
        // are called.
        let mut queue = vec![(class.to_lowercase(), f.to_lowercase())];
        let mut dependencies = HashMap::<(String, String), HashSet<(String, String)>>::new();
        while let Some((class, function)) = queue.pop() {
            let def = self.data.get_function(&class, &function).unwrap();

            // Function is already compiled.
            if def.func_id.is_some() {
                continue;
            }

            let callees = find_function_callees(&self.data.classes, &class, &def.function);

            let mut depends_on = HashSet::new();
            for (callee_class, callee_func) in callees {
                let def = self.data.get_function(&callee_class, &callee_func).unwrap();

                if !def.func_id.is_some() {
                    depends_on.insert((callee_class.clone(), callee_func.clone()));
                    queue.push((callee_class, callee_func));
                }
            }

            if depends_on.is_empty() {
                {
                    let current_class = self.data.classes.get(&class).unwrap();
                    let func = self.data.get_function(&class, &function).unwrap();
                    let props = function_properties(&self.data, current_class, &func.function);
                    self.data.get_function_mut(&class, &function).unwrap().props = Some(props);
                }

                // Compile the function.
                let res = self.compile_function_internal(&class, &function, &mut compiler);

                self.data
                    .get_function_mut(&class, &function)
                    .unwrap()
                    .func_id = Some(res.id);

                self.data
                    .get_function_mut(&class, &function)
                    .unwrap()
                    .signature = Some(res.signature);

                // Mark all dependend callers as ready.
                dependencies.retain(|(class_name, func_name), depends| {
                    depends.remove(&(class.clone(), function.clone()));
                    if depends.is_empty() {
                        queue.push((class_name.clone(), func_name.clone()));
                        false
                    } else {
                        true
                    }
                });

                continue;
            }

            dependencies.insert((class, function), depends_on);
        }

        let mut class_layouts = HashMap::new();
        for class in self.data.classes.values() {
            let mut class_fields = Vec::new();
            let mut next_class = Some(&class.name);
            while let Some(class_name) = next_class.take() {
                let class = self.data.get_class(class_name).unwrap();
                let layout = class.layout.as_ref().unwrap();

                for (field_name, field_offset) in &layout.field_offsets {
                    let field_ty = self.data.resolve_ty(&class.fields.get(field_name).unwrap());

                    class_fields.push(ClassFieldLayout {
                        class: class_name.to_lowercase(),
                        name: field_name.to_lowercase(),
                        offset: *field_offset,
                        ty: match field_ty {
                            Type::Bool => FieldType::Bool,
                            Type::Int => FieldType::I32,
                            Type::Float => FieldType::F32,
                            _ => FieldType::Pointer,
                        },
                    });
                }

                next_class = class.parent.as_ref();
            }

            class_layouts.insert(class.name.to_lowercase(), class_fields);
        }

        let def = self.data.get_function_mut(class, &f).unwrap();

        if def.trampoline_func_id.is_none() {
            let trampoline = trampoline::generate_host_to_guest_trampoline(
                &mut compiler,
                def.func_id.unwrap(),
                &def.signature.as_ref().unwrap(),
            );
            def.trampoline_func_id = Some(trampoline);
        }

        let props = def.props.as_ref().unwrap();

        Ok(CompilationResult {
            function: compiler.compile_function(def.trampoline_func_id.unwrap()),
            stack_size: props.stack_size,
            class_layouts,
        })
    }

    fn compile_function_internal(
        &self,
        class: &str,
        function_name: &str,
        compiler: &mut Compiler,
    ) -> FunctionResult {
        tracing::debug!("comiling {}.{}", class, function_name);

        let current_class = self.data.classes.get(class).unwrap();
        let function = self.data.get_function(class, function_name).unwrap();

        println!("{}.{}", class, function_name);
        println!("{}", function.function.text());

        let res = function::translate_function(&self.data, current_class, &function, compiler);

        if function.function.is_native {
            return res;
        }

        let compiled = compiler.compile_function(res.id);

        println!("{}", compiled.text().unwrap());

        res
    }
}

#[derive(Clone, Debug)]
struct CodegenData {
    types: Types,
    classes: HashMap<String, ClassDef>,
    structs: HashMap<String, Struct>,
}

impl CodegenData {
    fn new() -> Self {
        Self {
            types: Types::default(),
            classes: HashMap::new(),
            structs: HashMap::new(),
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
            name if self.structs.contains_key(name) => Type::Struct(
                self.structs[name]
                    .members
                    .iter()
                    .map(|v| (v.name.clone(), v.ty.clone()))
                    .collect(),
            ),
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

    fn get_function(&self, class: &str, func: &str) -> Option<&FunctionDef> {
        let class = get_parent_class_with_function(&self.classes, class, func)?;
        self.classes
            .get(&class)?
            .functions
            .get(&func.to_lowercase())
    }

    fn get_function_mut(&mut self, class: &str, func: &str) -> Option<&mut FunctionDef> {
        let class = get_parent_class_with_function(&self.classes, class, func)?;
        self.classes
            .get_mut(&class)?
            .functions
            .get_mut(&func.to_lowercase())
    }
}

#[derive(Clone, Debug, Default)]
struct ClassDef {
    name: String,
    functions: HashMap<String, FunctionDef>,
    parent: Option<String>,
    fields: HashMap<String, String>,
    /// Unique RTTI number stored in the header to identify objects of this class.
    type_id: usize,
    layout: Option<ClassLayout>,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    signature: Option<Signature>,
    func_id: Option<FunctionId>,
    function: Function,
    native: Option<NativeFunction>,
    trampoline_func_id: Option<FunctionId>,
    props: Option<FunctionProps>,
}

fn find_function_callees(
    classes: &HashMap<String, ClassDef>,
    class: &str,
    function: &Function,
) -> Vec<(String, String)> {
    let mut callees = Vec::new();

    let current_class = classes.get(class).unwrap();

    let find_var = |var_name: &str| -> String {
        match &*var_name.to_lowercase() {
            "self" => return class.to_lowercase(),
            _ => (),
        }

        for param in &function.params {
            if param.name.to_lowercase() == var_name.to_lowercase() {
                return param.ty.to_lowercase();
            }
        }

        for local in &function.locals {
            if local.name.to_lowercase() == var_name.to_lowercase() {
                return local.ty.to_lowercase();
            }
        }

        if let Some(ty) = current_class.fields.get(&var_name.to_lowercase()) {
            return ty.to_lowercase();
        }

        dbg!(&var_name);

        todo!()
    };

    for instruction in &function.instructions {
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

                callees.push((class_name, function_name.to_lowercase()));
            }
            Instruction::CallParent(op) => {}
            Instruction::CallStatic(op) => {
                let (class_name, function_name) = match (&op.class, &op.function) {
                    (Value::Ident(a), Value::Ident(b)) => (a, b),
                    _ => unreachable!(),
                };

                callees.push((class_name.to_lowercase(), function_name.to_lowercase()));
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
    Struct(Vec<(String, String)>),
}
