use std::collections::{HashMap, HashSet};

use openfo4_pex::{Class, Function, Instruction, Module, Value};
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
    #[error("invalid instruction operands")]
    InvalidInstructionOperands,
    #[error("duplicate state name {state} in class {class}")]
    DuplicateStateName { class: String, state: String },
    #[error("default state {class} for class {default_state} does not exist")]
    NoDefaultState {
        class: String,
        default_state: String,
    },
    #[error("mismatching function signatures accross states")]
    MismatchingFunctionSignaturesAcrossStates,
}

#[derive(Clone, Debug, Default)]
pub struct ValidationPass {
    classes: HashMap<String, Class>,
}

impl ValidationPass {
    pub fn register_module(&mut self, module: &Module) {
        for class in &module.classes {
            self.classes
                .insert(class.name.to_lowercase(), class.clone());
        }
    }

    fn resolve_ty(&self, mut ty: &str) -> Result<Type, Error> {
        if ty.contains("#") {
            todo!();
        }

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
                    returns: (!matches!(func.return_ty.to_lowercase().as_str(), "" | "none"))
                        .then(|| self.resolve_ty(&func.return_ty))
                        .transpose()?,
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

    fn validate_function(&self, class: &Class, func: &Function) -> Result<(), Error> {
        let mut variables = HashMap::new();

        for var in &class.variables {
            let ty = self.resolve_ty(&var.definition.ty)?;
            variables.insert(var.definition.name.to_lowercase(), ty);
        }

        for param in &func.params {
            let ty = self.resolve_ty(&param.ty)?;
            variables.insert(param.name.to_lowercase(), ty);
        }

        for local in &func.locals {
            let ty = self.resolve_ty(&local.ty)?;
            variables.insert(local.name.to_lowercase(), ty);
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
                    return Err(Error::InvalidInstructionOperands);
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
                    assert_types_eq(&[src, dst], Type::Bool)?;
                }
                Instruction::Store(op) => {
                    let src = variable_type(&op.src)?;
                    let dst = variable_type(&op.dst)?;

                    if src != dst {
                        return Err(Error::InvalidInstructionOperands);
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), Error> {
        for class in self.classes.values() {
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FuncSignature {
    pub is_static: bool,
    pub is_native: bool,
    pub params: Vec<Type>,
    pub returns: Option<Type>,
}
