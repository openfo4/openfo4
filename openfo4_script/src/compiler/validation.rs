use std::collections::{HashMap, HashSet};

use indexmap::IndexSet;
use openfo4_pex::{Class, Function, Instruction, Module, Property, Value};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("undefined type: {0}")]
    UndefinedType(String),
    #[error("classes {0:?} cause inheritance cycle")]
    InheritanceCycle(Vec<String>),
    #[error("class {class} inherits from non-class type {parent}")]
    ClassParentIsNotClassType { class: String, parent: String },
    #[error("duplicate variable name {variable} in class {class}")]
    DuplicateClassVariableName { class: String, variable: String },
    #[error("cannot assign {value:?} to variable of type {ty}")]
    IncomptabileAssignment { ty: String, value: Value },
    #[error("unknown variable {variable} in function {class}.{function}")]
    UnknownVariable {
        class: String,
        function: String,
        variable: String,
    },
    #[error("duplicate state name {state} in class {class}")]
    DuplicateStateName { class: String, state: String },
    #[error("default state {class} for class {default_state} does not exist")]
    NoDefaultState {
        class: String,
        default_state: String,
    },
    #[error("mismatching function signatures accross states")]
    MismatchingFunctionSignaturesAcrossStates,
    #[error("invalid operands: expected {expected:?}, found {found:?}")]
    InvalidOperands {
        expected: Vec<Type>,
        found: Vec<Type>,
    },
    #[error("branch offset goes out of bounds")]
    BranchOffsetOutOfBounds,
    #[error("undefined function {class}.{function}")]
    UndefinedFunction { class: String, function: String },
}

#[derive(Clone, Debug, Default)]
pub struct ValidationPass {
    classes: HashMap<String, Class>,
    structs: HashMap<String, Type>,
    unvalidated_classes: IndexSet<String>,
}

impl ValidationPass {
    pub fn register_module(&mut self, module: &Module) {
        for class in &module.classes {
            self.classes
                .insert(class.name.to_lowercase(), class.clone());

            for st in &class.structs {
                let name = format!("{}#{}", class.name.to_lowercase(), st.name.to_lowercase());

                let mut members = Vec::new();
                for member in &st.members {
                    members.push((member.name.to_lowercase(), member.ty.to_lowercase()));
                }

                self.structs
                    .insert(name.to_lowercase(), Type::Struct(members));
            }

            self.unvalidated_classes.insert(class.name.to_lowercase());
        }
    }

    fn resolve_ty(&self, mut ty: &str) -> Result<Type, Error> {
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
            name if self.structs.contains_key(name) => self.structs[name].clone(),
            name if self.classes.contains_key(name) => Type::Class(name.to_owned()),
            _ => return Err(Error::UndefinedType(ty.to_owned())),
        };

        for _ in 0..array_depth {
            typ = Type::Array(Box::new(typ));
        }

        Ok(typ)
    }

    fn validate_class(&self, class: &Class) -> Result<(), Error> {
        // - If the class inherits from another type it must exist.
        // - The parent type must be class.
        // - The class hierarchy must not form a cycle.
        if !class.parent.is_empty() && class.parent.to_lowercase() != "none" {
            let parent = self.resolve_ty(&class.parent)?;
            let Type::Class(parent) = parent else {
                return Err(Error::ClassParentIsNotClassType {
                    class: class.name.to_owned(),
                    parent: class.parent.to_owned(),
                });
            };

            let mut cur_class = parent;
            loop {
                let parent = self.classes.get(&cur_class).unwrap();

                if parent.name == class.name {
                    // TODO: Fill class names.
                    return Err(Error::InheritanceCycle(vec![]));
                }

                if parent.parent.is_empty() || parent.parent.to_lowercase() == "none" {
                    break;
                }

                // Note: We ignore any errors here. When the parent class is resolved it
                // will already produce an error.
                let Ok(Type::Class(ty)) = self.resolve_ty(&parent.parent) else {
                    break;
                };

                cur_class = ty;
            }
        }

        // - Each variable name must not occur more than once.
        // - Each variable type must exist.
        // - If an initial value exists, it must be compatible.
        let mut variable_names = HashSet::new();
        for var in &class.variables {
            if !variable_names.insert(&var.definition.name) {
                return Err(Error::DuplicateClassVariableName {
                    class: class.name.clone(),
                    variable: var.definition.name.clone(),
                });
            }

            let ty = self.resolve_ty(&var.definition.ty)?;
            let is_ok = match (ty, &var.value) {
                (Type::None, Value::Null) => true,
                (Type::Bool, Value::Null | Value::Bool(_)) => true,
                (Type::Int, Value::Null | Value::Int(_)) => true,
                (Type::Float, Value::Null | Value::Float(_)) => true,
                (Type::String, Value::Null | Value::String(_)) => true,
                (Type::Array(_), Value::Null) => true,
                (Type::Class(_), Value::Null) => true,
                (Type::Struct(_), Value::Null) => true,
                _ => false,
            };

            if !is_ok {
                return Err(Error::IncomptabileAssignment {
                    ty: var.definition.ty.clone(),
                    value: var.value.clone(),
                });
            }
        }

        // - If a function with the same name exists in multiple states,
        //   all must have the same signature.
        // - Each state name must no occur more than once.
        let mut state_names = HashSet::new();
        let mut function_signatures = HashMap::new();

        for state in &class.states {
            if !state_names.insert(&state.name) {
                return Err(Error::DuplicateStateName {
                    class: class.name.clone(),
                    state: state.name.clone(),
                });
            }

            for func in &state.functions {
                let signature = FuncSignature {
                    is_static: func.is_static,
                    is_native: func.is_native,
                    params: func
                        .params
                        .iter()
                        .map(|var| self.resolve_ty(&var.ty))
                        .collect::<Result<_, _>>()?,
                    returns: self.resolve_ty(&func.return_ty)?,
                };

                match function_signatures.get(&func.name) {
                    Some(sig) if sig != &signature => {
                        return Err(Error::MismatchingFunctionSignaturesAcrossStates);
                    }
                    Some(_) => {}
                    None => {
                        function_signatures.insert(&func.name, signature);
                    }
                }

                self.validate_function(class, func)?;
            }
        }

        if !state_names.contains(&class.default_state) {
            return Err(Error::NoDefaultState {
                class: class.name.clone(),
                default_state: class.default_state.clone(),
            });
        }

        Ok(())
    }

    /// Checks whether `src` is assignable to `dst`.
    fn is_compatible(&self, src: &Type, dst: &Type) -> bool {
        match (src, dst) {
            (Type::None, Type::None) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Int, Type::Int) => true,
            (Type::Float, Type::Float) => true,
            (Type::String, Type::String) => true,
            // Storing an object is possible if src > dst,
            // i.e. dst is a parent of src.
            // Try to upcast until we have a match.
            (Type::Class(src), Type::Class(dst)) => {
                let mut src = src.to_lowercase();
                let dst = dst.to_lowercase();

                loop {
                    if src == *dst {
                        return true;
                    }

                    let class = self.classes.get(&src).unwrap();
                    src = class.parent.to_lowercase();
                }
            }
            // None can be assigned to any object.
            (Type::None, Type::Class(_)) => true,
            (Type::Struct(src), Type::Struct(dst)) if src == dst => true,
            // None can be assigned to any struct.
            (Type::None, Type::Struct(_)) => true,
            (Type::None, Type::Array(_)) => true,
            (Type::Array(src), Type::Array(dst)) if src == dst => true,
            (Type::Var, Type::Var) => true,
            _ => false,
        }
    }

    fn function_signature(
        &self,
        class: &str,
        func: &str,
        visit_parents: bool,
    ) -> Result<FuncSignature, Error> {
        let search_class = class;
        let search_function = func;

        let mut class_name = class.to_lowercase();

        let func = loop {
            let class = self.classes.get(&class_name.to_lowercase()).unwrap();

            if let Some(func) = class.states.iter().find_map(|state| {
                state
                    .functions
                    .iter()
                    .find(|f| f.name.to_lowercase() == func.to_lowercase())
            }) {
                break func;
            }

            class_name = class.parent.to_lowercase();

            if !visit_parents || class_name.is_empty() {
                return Err(Error::UndefinedFunction {
                    class: search_class.to_owned(),
                    function: search_function.to_owned(),
                });
            }
        };

        let mut params = Vec::new();
        for param in &func.params {
            let ty = self.resolve_ty(&param.ty)?;
            params.push(ty);
        }

        let returns = self.resolve_ty(&func.return_ty)?;

        Ok(FuncSignature {
            is_static: func.is_static,
            is_native: func.is_native,
            params,
            returns,
        })
    }

    fn get_property(&self, class: &str, prop: &str) -> Result<&Property, Error> {
        let mut class_name = class.to_lowercase();
        loop {
            let class = self.classes.get(&class_name.to_lowercase()).unwrap();

            if let Some(prop) = class
                .properties
                .iter()
                .find(|p| p.name.to_lowercase() == prop.to_lowercase())
            {
                return Ok(prop);
            }

            class_name = class.parent.to_lowercase();

            if class_name.is_empty() {
                todo!();
            }
        }
    }

    fn is_castable(&self, mut src: Type, mut dst: Type) -> bool {
        if src == dst {
            return true;
        }

        loop {
            match (src, dst) {
                (Type::Int, Type::Float) => return true,
                (Type::Float, Type::Int) => return true,
                (Type::Class(src), Type::Class(dst)) => {
                    // Try downcasting.
                    let mut class_name = src.to_lowercase();
                    loop {
                        if class_name == dst.to_lowercase() {
                            return true;
                        }

                        let class = self.classes.get(&class_name).unwrap();
                        class_name = class.parent.to_lowercase();
                        if class_name.is_empty() {
                            break;
                        }
                    }

                    // Try upcasting.
                    let mut class_name = dst.to_lowercase();
                    loop {
                        if class_name == src.to_lowercase() {
                            return true;
                        }

                        let class = self.classes.get(&class_name).unwrap();
                        class_name = class.parent.to_lowercase();
                        if class_name.is_empty() {
                            break;
                        }
                    }

                    todo!();
                }
                (
                    Type::Int
                    | Type::Float
                    | Type::String
                    | Type::Class(_)
                    | Type::Array(_)
                    | Type::Struct(_),
                    Type::Bool,
                ) => return true,
                (Type::Bool, Type::Int | Type::Float | Type::String) => return true,
                (
                    Type::Int | Type::Float | Type::Class(_) | Type::Array(_) | Type::Struct(_),
                    Type::String,
                ) => return true,
                (Type::Array(src_elem), Type::Array(dst_elem)) => {
                    src = *src_elem;
                    dst = *dst_elem;
                }
                // Var is a opaque runtime value and can be casted
                // into anything.
                (Type::Var, _) => return true,
                (_, Type::Var) => return true,
                // Creating a null pointer.
                (Type::None, Type::Class(_) | Type::Array(_) | Type::String) => return true,
                (src, dst) => {
                    todo!("{:?} {:?}", src, dst);
                }
            }
        }
    }

    fn validate_function(&self, class: &Class, func: &Function) -> Result<(), Error> {
        let mut variables = HashMap::new();

        // A class can access any fields defined by its parents,
        // however if both child and parent have the same field defined
        // the child field takes precedence.
        {
            let mut current_class = class;
            loop {
                for var in &current_class.variables {
                    let name = var.definition.name.to_lowercase();
                    if variables.contains_key(&name) {
                        continue;
                    }

                    let ty = self.resolve_ty(&var.definition.ty)?;
                    variables.insert(name, ty);
                }

                let parent = current_class.parent.to_lowercase();
                if matches!(parent.as_str(), "" | "none") {
                    break;
                }

                current_class = self.classes.get(&parent).unwrap();
            }
        }

        for param in &func.params {
            let ty = self.resolve_ty(&param.ty)?;
            variables.insert(param.name.to_lowercase(), ty);
        }

        for local in &func.locals {
            let ty = self.resolve_ty(&local.ty)?;
            variables.insert(local.name.to_lowercase(), ty);
        }

        // Non-static functions have a magic "self" variable for the self object.
        // The type is equal to the class of the function.
        if !func.is_static {
            let ty = self.resolve_ty(&class.name)?;
            variables.insert("self".to_owned(), ty);
        }

        let variable_type = |value: &Value| -> Result<Type, Error> {
            match value {
                Value::Null => Ok(Type::None),
                Value::Bool(_) => Ok(Type::Bool),
                Value::Int(_) => Ok(Type::Int),
                Value::Float(_) => Ok(Type::Float),
                Value::String(_) => Ok(Type::String),
                Value::Ident(name) => {
                    let name = name.to_lowercase();
                    match &*name {
                        // Magic variable that holds the current state as a string.
                        "::state" => Ok(Type::String),
                        _ => variables
                            .get(&name)
                            .cloned()
                            .ok_or_else(|| Error::UnknownVariable {
                                class: class.name.clone(),
                                function: func.name.clone(),
                                variable: name.clone(),
                            }),
                    }
                }
            }
        };

        let assert_types_eq = |types: &[Type], expect: Type| -> Result<(), Error> {
            for ty in types {
                if ty != &expect {
                    return Err(Error::InvalidOperands {
                        expected: std::iter::repeat_n(expect, types.len()).collect(),
                        found: types.to_vec(),
                    });
                }
            }

            Ok(())
        };

        for instruction in &func.instructions {
            match instruction {
                Instruction::Nop => {}
                // - lhs, rhs and dst must be integers.
                Instruction::IAdd(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Int)?;
                }
                Instruction::ISub(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Int)?;
                }
                Instruction::IMul(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Int)?;
                }
                Instruction::IDiv(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Int)?;
                }
                Instruction::IMod(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Int)?;
                }
                Instruction::INeg(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[src, dst], Type::Int)?;
                }
                Instruction::FAdd(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Float)?;
                }
                Instruction::FSub(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Float)?;
                }
                Instruction::FMul(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Float)?;
                }
                Instruction::FDiv(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::Float)?;
                }
                Instruction::FNeg(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[src, dst], Type::Float)?;
                }
                Instruction::Not(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;

                    match (&src, &dst) {
                        (Type::Bool, Type::Bool) => {}
                        (Type::Int, Type::Bool) => {}
                        (Type::String, Type::Bool) => {}
                        (Type::Class(_), Type::Bool) => {}
                        (Type::Struct(_), Type::Bool) => {}
                        (Type::Array(_), Type::Bool) => {}
                        _ => {
                            todo!();
                        }
                    }
                }
                Instruction::Store(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;

                    if !self.is_compatible(&src, &dst) {
                        dbg!(&src, &dst);
                        todo!();
                    }
                }
                Instruction::CmpEq(op)
                | Instruction::CmpLt(op)
                | Instruction::CmpLe(op)
                | Instruction::CmpGt(op)
                | Instruction::CmpGe(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;

                    if !self.is_compatible(&lhs, &rhs) && !self.is_compatible(&rhs, &lhs) {
                        todo!();
                    }

                    assert_types_eq(&[dst], Type::Bool)?;
                }
                Instruction::Branch(offset) => {
                    assert!(matches!(offset, Value::Int(_)))
                }
                Instruction::BranchIfTrue(op) | Instruction::BranchIfFalse(op) => {
                    let condition = variable_type(&op.condition)?;

                    assert!(self.is_castable(condition, Type::Bool));
                    assert!(matches!(op.offset, Value::Int(_)));
                }
                Instruction::CallMethod(op) => {
                    let object = variable_type(&op.object)?;
                    let dst = variable_type(&op.return_assignment)?;

                    let Type::Class(class) = object else {
                        todo!();
                    };

                    let Value::Ident(func) = &op.function_name else {
                        todo!();
                    };

                    let sig = self.function_signature(&class, func, true)?;

                    assert!(!sig.is_static);

                    assert_eq!(op.args.len(), sig.params.len());
                    for (param, arg) in sig.params.iter().zip(&op.args) {
                        let arg = variable_type(&arg)?;
                        assert!(self.is_compatible(&arg, param));
                    }

                    assert!(self.is_compatible(&sig.returns, &dst));
                }
                Instruction::CallParent(op) => {
                    let dst = variable_type(&op.return_assignment)?;

                    let Value::Ident(func) = &op.function else {
                        todo!();
                    };

                    let sig = self.function_signature(&class.parent, func, true)?;

                    assert!(!sig.is_static);

                    assert_eq!(op.args.len(), sig.params.len());
                    for (param, arg) in sig.params.iter().zip(&op.args) {
                        let arg = variable_type(&arg)?;
                        assert!(self.is_compatible(&arg, param));
                    }

                    assert!(self.is_compatible(&sig.returns, &dst));
                }
                Instruction::CallStatic(op) => {
                    let (Value::Ident(class), Value::Ident(func)) = (&op.class, &op.function)
                    else {
                        todo!();
                    };

                    // Static function do not recognise parent classes.
                    let sig = self.function_signature(class, func, false)?;

                    assert!(sig.is_static);

                    assert_eq!(op.args.len(), sig.params.len());
                    for (parm, arg) in sig.params.iter().zip(&op.args) {
                        let arg = variable_type(&arg)?;
                        assert!(self.is_compatible(&arg, parm));
                    }

                    let dst = variable_type(&op.return_assignment)?;
                    assert!(self.is_compatible(&sig.returns, &dst));
                }
                Instruction::Return(value) => {
                    let value = variable_type(value)?;
                    let return_ty = self.resolve_ty(&func.return_ty)?;

                    if !self.is_compatible(&value, &return_ty) {
                        todo!();
                    }
                }
                Instruction::StringConcat(op) => {
                    let lhs = variable_type(&op.lhs)?;
                    let rhs = variable_type(&op.rhs)?;
                    let dst = variable_type(&op.dst)?;
                    assert_types_eq(&[lhs, rhs, dst], Type::String)?;
                }
                Instruction::PropertyGet(op) => {
                    let value = variable_type(&op.value)?;
                    let dst = variable_type(&op.return_assignment)?;

                    let Type::Class(class) = value else {
                        todo!();
                    };

                    let Value::Ident(property) = &op.property else {
                        todo!();
                    };

                    let prop = self.get_property(&class, &property)?;
                    let ty = self.resolve_ty(&prop.ty)?;

                    assert_eq!(ty, dst);
                }
                Instruction::PropertySet(op) => {
                    let object = variable_type(&op.object)?;
                    let value = variable_type(&op.value)?;

                    let Type::Class(class) = object else {
                        todo!();
                    };

                    let Value::Ident(property) = &op.property else {
                        todo!();
                    };

                    let prop = self.get_property(&class, &property)?;
                    let ty = self.resolve_ty(&prop.ty)?;

                    assert_eq!(ty, value);
                }
                Instruction::ArrayCreate(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;

                    // src is the array length.
                    assert!(matches!(src, Type::Int));
                    assert!(matches!(dst, Type::Array(_)));
                }
                Instruction::ArrayLength(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;

                    assert!(matches!(src, Type::Array(_)));
                    assert!(matches!(dst, Type::Int));
                }
                Instruction::ArrayGet(op) => {
                    let array = variable_type(&op.array)?;
                    let index = variable_type(&op.index)?;
                    let dst = variable_type(&op.return_assignment)?;

                    let Type::Array(element) = array else {
                        todo!();
                    };

                    assert_eq!(*element, dst);
                    assert!(matches!(index, Type::Int));
                }
                Instruction::ArraySet(op) => {
                    let index = variable_type(&op.index)?;
                    let value = variable_type(&op.value)?;
                    let array = variable_type(&op.array)?;

                    let Type::Array(element) = array else {
                        todo!();
                    };

                    assert_eq!(*element, value);
                    assert!(matches!(index, Type::Int));
                }
                Instruction::ArrayFind(op) | Instruction::ArrayRFind(op) => {
                    let array = variable_type(&op.array_value)?;
                    let start_index = variable_type(&op.start_index)?;
                    let dst = variable_type(&op.return_index)?;
                    let value = variable_type(&op.value)?;

                    let Type::Array(element) = array else {
                        todo!();
                    };

                    assert_eq!(*element, value);
                    assert!(matches!(start_index, Type::Int));
                    assert!(matches!(dst, Type::Int));
                }
                Instruction::Is(op) => {
                    let object = variable_type(&op.object)?;
                    let dst = variable_type(&op.return_assignment)?;

                    let Value::Ident(class) = &op.class else {
                        todo!();
                    };

                    assert!(self.classes.contains_key(&class.to_lowercase()));
                    assert!(matches!(object, Type::Class(_)));
                    assert!(matches!(dst, Type::Bool));
                }
                Instruction::Cast(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;

                    assert!(self.is_castable(src, dst));
                }
                Instruction::StructCreate(op) => {
                    let dst = variable_type(&op.return_assignment)?;
                    assert!(matches!(dst, Type::Struct(_)));
                }
                Instruction::StructGet(op) => {
                    let struct_value = variable_type(&op.struct_value)?;
                    let dst = variable_type(&op.return_assignment)?;

                    let Value::Ident(member_name) = &op.member_name else {
                        todo!();
                    };

                    let Type::Struct(members) = struct_value else {
                        todo!();
                    };

                    let Some((_, ty)) = members
                        .iter()
                        .find(|(name, _)| name.to_lowercase() == member_name.to_lowercase())
                    else {
                        todo!();
                    };
                    let ty = self.resolve_ty(&ty)?;

                    assert_eq!(ty, dst);
                }
                Instruction::StructSet(op) => {
                    let struct_value = variable_type(&op.struct_value)?;
                    let value = variable_type(&op.value)?;

                    let Value::Ident(member_name) = &op.member_name else {
                        todo!();
                    };

                    let Type::Struct(members) = struct_value else {
                        todo!();
                    };

                    let Some((_, ty)) = members
                        .iter()
                        .find(|(name, _)| name.to_lowercase() == member_name.to_lowercase())
                    else {
                        todo!();
                    };
                    let ty = self.resolve_ty(&ty)?;

                    assert_eq!(ty, value);
                }
                Instruction::ArrayFindStruct(op) | Instruction::ArrayRFindStruct(op) => {
                    let array = variable_type(&op.array_value)?;
                    let start_index = variable_type(&op.start_index)?;
                    let dst = variable_type(&op.return_index)?;
                    let value = variable_type(&op.value)?;

                    let Value::String(member_name) = &op.member_name else {
                        todo!();
                    };

                    let Type::Array(element) = array else {
                        todo!();
                    };
                    let Type::Struct(members) = *element else {
                        todo!();
                    };

                    let Some((_, ty)) = members
                        .iter()
                        .find(|(name, _)| name.to_lowercase() == member_name.to_lowercase())
                    else {
                        todo!();
                    };
                    let ty = self.resolve_ty(&ty)?;
                    assert_eq!(ty, value);
                    assert!(matches!(start_index, Type::Int));
                    assert!(matches!(dst, Type::Int));
                }
                Instruction::ArrayAdd(op) => {
                    let array = variable_type(&op.array)?;
                    let value = variable_type(&op.value)?;
                    let count = variable_type(&op.count)?;

                    let Type::Array(element) = array else {
                        todo!();
                    };

                    assert_eq!(*element, value);
                    assert!(matches!(count, Type::Int))
                }
                Instruction::ArrayInsert(op) => {
                    let array = variable_type(&op.array)?;
                    let index = variable_type(&op.index)?;
                    let value = variable_type(&op.value)?;

                    let Type::Array(element) = array else {
                        todo!();
                    };

                    assert_eq!(&*element, &value);
                    assert!(matches!(index, Type::Int));
                }
                Instruction::ArrayPop(op) => {
                    let ty = variable_type(&op.array)?;
                    assert!(matches!(ty, Type::Array(_)));
                }
                Instruction::ArrayRemove(op) => {
                    let array = variable_type(&op.array_value)?;
                    let index = variable_type(&op.index)?;
                    let count = variable_type(&op.count)?;

                    assert!(matches!(array, Type::Array(_)));
                    assert!(matches!(index, Type::Int));
                    assert!(matches!(count, Type::Int));
                }
                Instruction::ArrayClear(op) => {
                    let ty = variable_type(&op.array)?;
                    assert!(matches!(ty, Type::Array(_)));
                }
            }
        }

        Ok(())
    }

    pub fn validate(&mut self) -> Result<(), Error> {
        while let Some(class_name) = self.unvalidated_classes.pop() {
            let class = self.classes.get(&class_name).unwrap();
            self.validate_class(&class)?;

            for state in &class.states {
                for func in &state.functions {
                    self.validate_function(class, func)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FuncSignature {
    pub is_static: bool,
    pub is_native: bool,
    pub params: Vec<Type>,
    pub returns: Type,
}
