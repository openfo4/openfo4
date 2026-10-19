mod instruction;

use hashbrown::{HashMap, HashSet};
use indexmap::IndexSet;
use openfo4_common::string::Lowercase;
use openfo4_pex::{Class, Function, Instruction, Module, Value};
use openfo4_tracing::trace_span;
use std::fmt::{self, Display, Formatter};
use thiserror::Error;

use crate::compiler::validation::instruction::InvalidInstructionError;

const NONE: &Lowercase<str> = Lowercase::from_static("none");
const BOOL: &Lowercase<str> = Lowercase::from_static("bool");
const INT: &Lowercase<str> = Lowercase::from_static("int");
const FLOAT: &Lowercase<str> = Lowercase::from_static("float");
const STRING: &Lowercase<str> = Lowercase::from_static("string");
const VAR: &Lowercase<str> = Lowercase::from_static("var");
const SELF: &Lowercase<str> = Lowercase::from_static("self");
const STATE: &Lowercase<str> = Lowercase::from_static("::state");

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("undefined type: {0}")]
    UndefinedType(Lowercase<String>),
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
    #[error(transparent)]
    InvalidInstruction(InvalidInstruction),
}

#[derive(Clone, Debug, Error)]
#[error("invalid instruction in {class}.{function} at position {index}: {error}")]
pub struct InvalidInstruction {
    class: String,
    function: String,
    index: usize,
    inst: Instruction,
    error: InvalidInstructionError,
}

#[derive(Clone, Debug)]
pub enum OperandKey {
    Named(&'static str),
    Unnamed(usize),
}

impl Display for OperandKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(v) => Display::fmt(v, f),
            Self::Unnamed(v) => Display::fmt(v, f),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ValidationPass {
    classes: HashMap<Lowercase<String>, ClassDef>,
    structs: HashMap<Lowercase<String>, Type>,
    unvalidated_classes: IndexSet<Lowercase<String>>,
}

impl ValidationPass {
    pub fn register_module(&mut self, module: &Module) {
        for class in &module.classes {
            let parent = if class.parent.is_empty() || class.parent.eq_ignore_ascii_case("none") {
                None
            } else {
                Some(Lowercase::new(&*class.parent))
            };

            let mut functions = HashMap::new();
            for state in &class.states {
                for func in &state.functions {
                    // For now we will assume that functions with the same name
                    // have the same signature across all states.
                    // If this is not the case validation will fail when the class
                    // is being validated and the contents that we have written here
                    // don't matter, as they can only be read once the class was
                    // validated.
                    let name = Lowercase::new(&*func.name);
                    if functions.contains_key(&name) {
                        continue;
                    }

                    let params = func
                        .params
                        .iter()
                        .map(|param| Lowercase::new(&*param.ty))
                        .collect();
                    let returns = Lowercase::new(&*func.return_ty);
                    functions.insert(
                        name,
                        FuncSig {
                            params,
                            returns,
                            is_static: func.is_static,
                            is_native: func.is_native,
                        },
                    );
                }
            }

            let mut properties = HashMap::new();
            for prop in &class.properties {
                let name = Lowercase::new(&*prop.name);
                let ty = Lowercase::new(&*prop.ty);
                properties.insert(name, ty);
            }

            self.classes.insert(
                Lowercase::new(&*class.name),
                ClassDef {
                    parent,
                    class: class.clone(),
                    functions,
                    properties,
                },
            );

            for st in &class.structs {
                let name = Lowercase::new(&*format!("{}#{}", class.name, st.name));

                let mut members = HashMap::new();
                for member in &st.members {
                    let name = Lowercase::new(&*member.name);
                    let ty = Lowercase::new(&*member.ty);
                    members.insert(
                        name,
                        StructMember {
                            name: member.name.clone(),
                            ty,
                        },
                    );
                }

                self.structs.insert(
                    name,
                    Type::Struct(StructType {
                        name: st.name.clone(),
                        members,
                    }),
                );
            }

            self.unvalidated_classes
                .insert(Lowercase::new(&*class.name));
        }
    }

    fn resolve_ty(&self, mut ty: &Lowercase<str>) -> Result<Type, Error> {
        let mut array_depth = 0;
        while let Some(p) = ty.strip_suffix("[]") {
            ty = p;
            array_depth += 1;
        }
        let mut typ = match ty {
            s if s == NONE => Type::None,
            s if s == BOOL => Type::Bool,
            s if s == INT => Type::Int,
            s if s == FLOAT => Type::Float,
            s if s == STRING => Type::String,
            s if s == VAR => Type::Var,
            name if let Some(ty) = self.structs.get(name) => ty.clone(),
            name if self.classes.contains_key(name) => Type::Class(name.to_owned()),
            _ => return Err(Error::UndefinedType(ty.to_owned())),
        };

        for _ in 0..array_depth {
            typ = Type::Array(Box::new(typ));
        }

        Ok(typ)
    }

    fn validate_class(&self, class: &ClassDef) -> Result<(), Error> {
        // - If the class inherits from another type it must exist.
        // - The parent type must be class.
        // - The class hierarchy must not form a cycle.
        let mut visited = HashSet::new();
        let mut cur_class = class;
        while let Some(parent) = &cur_class.parent {
            {
                let parent = self.resolve_ty(parent)?;
                if !matches!(parent, Type::Class(_)) {
                    return Err(Error::ClassParentIsNotClassType {
                        class: cur_class.class.name.clone(),
                        parent: cur_class.class.parent.clone(),
                    });
                };
            }

            if !visited.insert(parent) {
                // TODO: Fill this out.
                return Err(Error::InheritanceCycle(vec![]));
            }

            // This unwrap cannot fail, if the above type resolved
            // to a Type::Class successfully.
            let class = self.classes.get(parent).unwrap();
            cur_class = class;
        }

        // - Each variable name must not occur more than once.
        // - Each variable type must exist.
        // - If an initial value exists, it must be compatible.
        let mut variable_names = HashSet::new();
        for var in &class.class.variables {
            let var_name = Lowercase::new(&*var.definition.name);
            let var_ty = Lowercase::new(&*var.definition.ty);

            if !variable_names.insert(var_name) {
                return Err(Error::DuplicateClassVariableName {
                    class: class.class.name.clone(),
                    variable: var.definition.name.clone(),
                });
            }

            let ty = self.resolve_ty(&var_ty)?;
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
        // - No state name may occur more than once.
        let mut state_names = HashSet::new();
        let mut function_signatures = HashMap::new();

        for state in &class.class.states {
            let state_name = Lowercase::new(&*state.name);

            if !state_names.insert(state_name) {
                return Err(Error::DuplicateStateName {
                    class: class.class.name.clone(),
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
                        .map(|var| self.resolve_ty(&Lowercase::new(&*var.ty)))
                        .collect::<Result<_, _>>()?,
                    returns: self.resolve_ty(&Lowercase::new(&*func.return_ty))?,
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

        if !state_names.contains(&Lowercase::new(&*class.class.default_state)) {
            return Err(Error::NoDefaultState {
                class: class.class.name.clone(),
                default_state: class.class.default_state.clone(),
            });
        }

        Ok(())
    }

    fn function_signature(
        &self,
        class: &Lowercase<str>,
        func: &Lowercase<str>,
        visit_parents: bool,
    ) -> Option<FuncSignature> {
        let mut class_name = class;

        let func = loop {
            let class = self.classes.get(class_name).unwrap();

            if let Some(func) = class.functions.get(func) {
                break func;
            }

            if let Some(parent) = &class.parent
                && visit_parents
            {
                class_name = parent;
                continue;
            }

            return None;
        };

        let mut params = Vec::new();
        for param in &func.params {
            let ty = self.resolve_ty(param).unwrap();
            params.push(ty);
        }

        let returns = self.resolve_ty(&func.returns).unwrap();

        Some(FuncSignature {
            is_static: func.is_static,
            is_native: func.is_native,
            params,
            returns,
        })
    }

    fn validate_function(&self, class: &ClassDef, func: &Function) -> Result<(), Error> {
        let mut variables = HashMap::new();

        // A class can access any fields defined by its parents,
        // however if both child and parent have the same field defined
        // the child field takes precedence.
        {
            let mut cur_class = class;
            loop {
                for var in &cur_class.class.variables {
                    let var_name = Lowercase::new(&*var.definition.name);
                    let var_ty = Lowercase::new(&*var.definition.ty);

                    if variables.contains_key(&var_name) {
                        continue;
                    }

                    let ty = self.resolve_ty(&var_ty)?;
                    variables.insert(var_name, ty);
                }

                if let Some(parent) = &cur_class.parent {
                    // Unwrap cannot fail if the class was validated successfully.
                    cur_class = self.classes.get(parent).unwrap();
                } else {
                    break;
                }
            }
        }

        for param in &func.params {
            let ty = self.resolve_ty(&Lowercase::new(&*param.ty))?;
            variables.insert(Lowercase::new(&*param.name), ty);
        }

        for local in &func.locals {
            let ty = self.resolve_ty(&Lowercase::new(&*local.ty))?;
            variables.insert(Lowercase::new(&*local.name), ty);
        }

        if !func.is_static {
            // Non-static functions have a magic "self" variable for the self object.
            // The type is equal to the class of the function.
            let ty = self.resolve_ty(&Lowercase::new(&*class.class.name))?;
            variables.insert(SELF.to_owned(), ty);

            // The builtin "::state" is only available in non-static functions.
            variables.insert(STATE.to_owned(), Type::String);
        }

        for (index, instruction) in func.instructions.iter().enumerate() {
            if let Err(error) = self.validate_instruction(class, func, instruction, &variables) {
                return Err(Error::InvalidInstruction(InvalidInstruction {
                    class: class.class.name.clone(),
                    function: func.name.clone(),
                    index,
                    error,
                    inst: instruction.clone(),
                }));
            }
        }

        Ok(())
    }

    pub fn validate(&mut self) -> Result<(), Error> {
        let _span = trace_span!("ValidationPass::validate").entered();

        while let Some(class_name) = self.unvalidated_classes.pop() {
            let class = self.classes.get(&class_name).unwrap();
            self.validate_class(class)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    None,
    Bool,
    Int,
    Float,
    String,
    Var,
    Array(Box<Type>),
    Class(Lowercase<String>),
    Struct(StructType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructType {
    pub name: String,
    pub members: HashMap<Lowercase<String>, StructMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructMember {
    pub name: String,
    pub ty: Lowercase<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncSignature {
    pub is_static: bool,
    pub is_native: bool,
    pub params: Vec<Type>,
    pub returns: Type,
}

#[derive(Clone, Debug)]
pub enum GenericType {
    Concrete(Type),
    /// Any array type.
    Array,
    /// Any struct type.
    Struct,
    /// Any class type.
    Class,
}

impl PartialEq<Type> for GenericType {
    fn eq(&self, other: &Type) -> bool {
        match (self, other) {
            (Self::Concrete(ty), other) => ty == other,
            (Self::Array, Type::Array(_)) => true,
            (Self::Struct, Type::Struct(_)) => true,
            (Self::Class, Type::Class(_)) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClassDef {
    parent: Option<Lowercase<String>>,
    class: Class,
    functions: HashMap<Lowercase<String>, FuncSig>,
    properties: HashMap<Lowercase<String>, Lowercase<String>>,
}

#[derive(Clone, Debug)]
struct FuncSig {
    is_static: bool,
    is_native: bool,
    params: Vec<Lowercase<String>>,
    returns: Lowercase<String>,
}
