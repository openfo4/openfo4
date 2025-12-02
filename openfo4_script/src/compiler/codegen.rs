mod function;
mod layout;
mod trampoline;
mod types;

use std::collections::{HashMap, HashSet};

use game_common::collections::arena::{Arena, Key};
use openfo4_jit::{CompiledFunction, Compiler, FunctionId, Signature, Type as JitType};
use openfo4_pex::{Function, Instruction, Module, Value};
use thiserror::Error;

use crate::compiler::NativeFunction;
use crate::compiler::codegen::function::FunctionResult;
use crate::compiler::codegen::types::Types;

#[derive(Clone, Debug, Error)]
#[error("codegen error")]
pub struct Error;

#[derive(Clone, Debug)]
pub struct CodegenPass {
    types: Arena<JitType>,
    primitives: Primitives,
    data: CodegenData,
}

impl CodegenPass {
    pub fn new() -> Self {
        let mut types = Arena::new();

        let none = types.insert(JitType::I32);
        let bool = types.insert(JitType::I32);
        let int = types.insert(JitType::I32);
        let float = types.insert(JitType::F32);
        let ptr = types.insert(JitType::Pointer);

        Self {
            types,
            primitives: Primitives {
                none,
                bool,
                int,
                float,
                ptr,
            },
            data: CodegenData::new(),
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
                        },
                    );
                }
            }

            self.data.classes.insert(
                class.name.to_lowercase(),
                ClassDef {
                    name: class.name.to_lowercase(),
                    functions,
                    parent,
                    fields,
                },
            );
        }
    }

    pub fn define_native_function(&mut self, class: &str, function: &str, func: NativeFunction) {
        let def = self.data.classes.get_mut(&class.to_lowercase()).unwrap();
        let func_def = def.functions.get_mut(&function.to_lowercase()).unwrap();
        func_def.native = Some(func);
    }

    pub fn compile_fn(&mut self, class: &str, f: &str) -> Result<CompiledFunction, Error> {
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

        let def = self.data.get_function_mut(class, &f).unwrap();

        if def.trampoline_func_id.is_none() {
            let trampoline = trampoline::generate_host_to_guest_trampoline(
                &mut compiler,
                def.func_id.unwrap(),
                &def.signature.as_ref().unwrap(),
            );
            def.trampoline_func_id = Some(trampoline);
        }

        Ok(compiler.compile_function(def.trampoline_func_id.unwrap()))
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
}

impl CodegenData {
    fn new() -> Self {
        Self {
            types: Types::default(),
            classes: HashMap::new(),
        }
    }

    fn resolve_ty(&self, ty: &str) -> JitType {
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

#[derive(Clone, Debug)]
struct Primitives {
    none: Key,
    bool: Key,
    int: Key,
    float: Key,
    ptr: Key,
}

#[derive(Clone, Debug, Default)]
struct ClassDef {
    name: String,
    functions: HashMap<String, FunctionDef>,
    parent: Option<String>,
    fields: HashMap<String, String>,
}

#[derive(Clone, Debug)]
struct FunctionDef {
    signature: Option<Signature>,
    func_id: Option<FunctionId>,
    function: Function,
    native: Option<NativeFunction>,
    trampoline_func_id: Option<FunctionId>,
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
