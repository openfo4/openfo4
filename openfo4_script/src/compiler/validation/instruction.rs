use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_pex::{Function, Instruction, Value};
use thiserror::Error;

use crate::compiler::validation::{ClassDef, GenericType, OperandKey, Type, ValidationPass};

#[derive(Clone, Debug, Error)]
pub enum InvalidInstructionError {
    #[error("unknown variable {0}")]
    UnknownVariable(String),
    #[error("invalid type {found:?} for {operand}, expected one of {expected:?}")]
    InvalidType {
        operand: OperandKey,
        found: Type,
        expected: Vec<GenericType>,
    },
    #[error("invalid type {found:?} for {operand}, cannot be coerced to {expected:?}")]
    InvalidCoersion {
        operand: OperandKey,
        found: Type,
        expected: Vec<GenericType>,
    },
    #[error("invalid type {found:?} for {operand}, cannot be cast into {expected:?}")]
    InvalidCast {
        operand: OperandKey,
        found: Type,
        expected: Vec<GenericType>,
    },
    #[error("unknown member {member} for struct {st}")]
    UnknownStructMember { st: String, member: String },
    #[error("unknown property {property} in class {class}")]
    UnknownClassProperty { class: String, property: String },
    #[error("invalid parameter count for call: expected {expected}, found {found}")]
    InvalidParameterCount { expected: usize, found: usize },
    #[error("cannot call_static non static function")]
    CallStaticOnNonStatic,
    #[error("cannot call method on static function")]
    CallMethodOnStatic,
    #[error("cannot call parent from static function")]
    CallParentFromStatic,
    #[error("cannot call parent from top-level class")]
    CallParentOnRoot,
    #[error("cannot call undefined function {class}.{func}")]
    UndefinedFunction { class: String, func: String },
}

impl ValidationPass {
    pub fn validate_instruction(
        &self,
        class: &ClassDef,
        function: &Function,
        instruction: &Instruction,
        variables: &HashMap<Lowercase<String>, Type>,
    ) -> Result<(), InvalidInstructionError> {
        let variable_type = |value: &Value| -> Result<Type, InvalidInstructionError> {
            match value {
                Value::Null => Ok(Type::None),
                Value::Bool(_) => Ok(Type::Bool),
                Value::Int(_) => Ok(Type::Int),
                Value::Float(_) => Ok(Type::Float),
                Value::String(_) => Ok(Type::String),
                Value::Ident(name) => variables
                    .get(&Lowercase::new(&**name))
                    .cloned()
                    .ok_or_else(|| InvalidInstructionError::UnknownVariable(name.clone())),
            }
        };

        let extract_struct_member_type =
            |operand: OperandKey, ty: Type, name: &str| -> Result<Type, InvalidInstructionError> {
                let name_lower = Lowercase::new(name);

                match ty {
                    Type::Struct(st) => {
                        if let Some(member) = st.members.get(&name_lower) {
                            let ty = self.resolve_ty(&member.ty).unwrap();
                            return Ok(ty);
                        }

                        Err(InvalidInstructionError::UnknownStructMember {
                            st: st.name,
                            member: name.to_owned(),
                        })
                    }
                    _ => Err(InvalidInstructionError::InvalidType {
                        operand,
                        found: ty,
                        expected: vec![GenericType::Struct],
                    }),
                }
            };

        let extract_property_name = |operand: OperandKey,
                                     ty: Type,
                                     prop_name: &str|
         -> Result<Type, InvalidInstructionError> {
            let prop_name_lower = Lowercase::new(prop_name);

            match ty {
                Type::Class(name) => {
                    let mut class = self.classes.get(&name).unwrap();

                    loop {
                        if let Some(ty) = class.properties.get(&prop_name_lower) {
                            let ty = self.resolve_ty(ty).unwrap();
                            return Ok(ty);
                        }

                        if let Some(parent) = &class.parent {
                            // Unwrap cannot fail since the class is validated.
                            class = self.classes.get(parent).unwrap();
                        } else {
                            break;
                        }
                    }

                    Err(InvalidInstructionError::UnknownClassProperty {
                        class: class.class.name.clone(),
                        property: prop_name.to_owned(),
                    })
                }
                _ => Err(InvalidInstructionError::InvalidCast {
                    operand,
                    found: ty,
                    expected: vec![GenericType::Class],
                }),
            }
        };

        fn assert_type_eq(
            operand: OperandKey,
            found: Type,
            expected: Type,
        ) -> Result<(), InvalidInstructionError> {
            if found != expected {
                Err(InvalidInstructionError::InvalidType {
                    operand,
                    expected: vec![GenericType::Concrete(expected)],
                    found,
                })
            } else {
                Ok(())
            }
        }

        fn assert_type_generic(
            operand: OperandKey,
            found: Type,
            expected: &[GenericType],
        ) -> Result<(), InvalidInstructionError> {
            if expected.iter().all(|e| *e != found) {
                Err(InvalidInstructionError::InvalidType {
                    operand,
                    expected: expected.to_vec(),
                    found,
                })
            } else {
                Ok(())
            }
        }

        fn extract_elem_ty(
            operand: OperandKey,
            array: Type,
        ) -> Result<Type, InvalidInstructionError> {
            match array {
                Type::Array(elem) => Ok(*elem),
                _ => Err(InvalidInstructionError::InvalidType {
                    operand,
                    found: array,
                    expected: vec![GenericType::Array],
                }),
            }
        }

        let return_ty = self
            .resolve_ty(&Lowercase::new(&*function.return_ty))
            .unwrap();

        match instruction {
            Instruction::Nop => {}
            Instruction::IAdd(op)
            | Instruction::ISub(op)
            | Instruction::IMul(op)
            | Instruction::IDiv(op)
            | Instruction::IMod(op) => {
                let lhs = variable_type(&op.lhs)?;
                let rhs = variable_type(&op.rhs)?;
                let dst = variable_type(&op.dst)?;

                assert_type_eq(OperandKey::Named("lhs"), lhs, Type::Int)?;
                assert_type_eq(OperandKey::Named("rhs"), rhs, Type::Int)?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Int)?;
            }
            Instruction::INeg(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;

                assert_type_eq(OperandKey::Named("src"), src, Type::Int)?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Int)?;
            }
            Instruction::FAdd(op)
            | Instruction::FSub(op)
            | Instruction::FMul(op)
            | Instruction::FDiv(op) => {
                let lhs = variable_type(&op.lhs)?;
                let rhs = variable_type(&op.rhs)?;
                let dst = variable_type(&op.dst)?;

                assert_type_eq(OperandKey::Named("lhs"), lhs, Type::Float)?;
                assert_type_eq(OperandKey::Named("rhs"), rhs, Type::Float)?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Float)?;
            }
            Instruction::FNeg(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;

                assert_type_eq(OperandKey::Named("src"), src, Type::Float)?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Float)?;
            }
            Instruction::Not(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;

                assert_type_generic(
                    OperandKey::Named("src"),
                    src,
                    &[
                        GenericType::Concrete(Type::Bool),
                        GenericType::Concrete(Type::Int),
                        GenericType::Concrete(Type::String),
                        GenericType::Class,
                        GenericType::Struct,
                        GenericType::Array,
                    ],
                )?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Bool)?;
            }
            Instruction::Store(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;
                self.assert_coercible(OperandKey::Named("src"), &src, &dst)?
            }
            Instruction::CmpEq(op)
            | Instruction::CmpLt(op)
            | Instruction::CmpLe(op)
            | Instruction::CmpGt(op)
            | Instruction::CmpGe(op) => {
                let is_cmpeq = matches!(instruction, Instruction::CmpEq(_));
                let lhs = variable_type(&op.lhs)?;
                let rhs = variable_type(&op.rhs)?;

                for (operand, ty) in [
                    (OperandKey::Named("lhs"), lhs.clone()),
                    (OperandKey::Named("rhs"), rhs.clone()),
                ] {
                    if is_cmpeq {
                        assert_type_generic(
                            operand,
                            ty,
                            &[
                                GenericType::Concrete(Type::None),
                                GenericType::Concrete(Type::Bool),
                                GenericType::Concrete(Type::Int),
                                GenericType::Concrete(Type::Float),
                                GenericType::Concrete(Type::String),
                                GenericType::Concrete(Type::Var),
                                GenericType::Class,
                                GenericType::Struct,
                                GenericType::Array,
                            ],
                        )?;
                    } else {
                        assert_type_generic(
                            operand,
                            ty,
                            &[
                                GenericType::Concrete(Type::None),
                                GenericType::Concrete(Type::Bool),
                                GenericType::Concrete(Type::Int),
                                GenericType::Concrete(Type::Float),
                            ],
                        )?;
                    }
                }

                // Try coercing in both directions, but if neither work prefer emitting an
                // error for coercing rhs to lhs.
                if let Err(err) = self.assert_coercible(OperandKey::Named("rhs"), &lhs, &rhs)
                    && self
                        .assert_coercible(OperandKey::Named("lhs"), &rhs, &lhs)
                        .is_err()
                {
                    return Err(err);
                }
            }
            Instruction::Branch(_) => {}
            Instruction::BranchIfTrue(op) | Instruction::BranchIfFalse(op) => {
                let condition = variable_type(&op.condition)?;

                assert_type_generic(
                    OperandKey::Named("condition"),
                    condition,
                    &[
                        GenericType::Concrete(Type::Bool),
                        GenericType::Concrete(Type::Int),
                        GenericType::Concrete(Type::Float),
                        GenericType::Concrete(Type::String),
                        GenericType::Class,
                        GenericType::Struct,
                        GenericType::Array,
                    ],
                )?;
            }
            Instruction::CallMethod(op) => {
                let object = variable_type(&op.object)?;
                let Type::Class(class) = object else {
                    return Err(InvalidInstructionError::InvalidType {
                        operand: OperandKey::Named("object"),
                        found: object,
                        expected: vec![GenericType::Class],
                    });
                };

                let func = Lowercase::new(&*op.function_name);
                let Some(sig) = self.function_signature(&class, &func, true) else {
                    let class = self.classes.get(&class).unwrap();
                    return Err(InvalidInstructionError::UndefinedFunction {
                        class: class.class.name.clone(),
                        func: op.function_name.clone(),
                    });
                };

                if sig.is_static {
                    return Err(InvalidInstructionError::CallMethodOnStatic);
                }

                if sig.params.len() != op.args.len() {
                    return Err(InvalidInstructionError::InvalidParameterCount {
                        expected: sig.params.len(),
                        found: op.args.len(),
                    });
                }

                for (index, (arg, param)) in op.args.iter().zip(sig.params).enumerate() {
                    let arg = variable_type(arg)?;
                    self.assert_coercible(OperandKey::Unnamed(index), &arg, &param)?;
                }

                let dst = variable_type(&op.return_assignment)?;
                self.assert_coercible(OperandKey::Named("dst"), &sig.returns, &dst)?;
            }
            Instruction::CallParent(op) => {
                let Some(parent) = &class.parent else {
                    return Err(InvalidInstructionError::CallParentOnRoot);
                };

                let func = Lowercase::new(&*op.function);
                let Some(sig) = self.function_signature(parent, &func, true) else {
                    let parent = self.classes.get(parent).unwrap();
                    return Err(InvalidInstructionError::UndefinedFunction {
                        class: parent.class.name.clone(),
                        func: op.function.clone(),
                    });
                };

                if sig.is_static {
                    return Err(InvalidInstructionError::CallMethodOnStatic);
                }

                if function.is_static {
                    return Err(InvalidInstructionError::CallParentFromStatic);
                }

                if sig.params.len() != op.args.len() {
                    return Err(InvalidInstructionError::InvalidParameterCount {
                        expected: sig.params.len(),
                        found: op.args.len(),
                    });
                }

                for (index, (arg, param)) in op.args.iter().zip(sig.params).enumerate() {
                    let arg = variable_type(arg)?;
                    self.assert_coercible(OperandKey::Unnamed(index), &arg, &param)?;
                }

                let dst = variable_type(&op.return_assignment)?;
                self.assert_coercible(OperandKey::Named("dst"), &sig.returns, &dst)?;
            }
            Instruction::CallStatic(op) => {
                let class = Lowercase::new(&*op.class);
                let func = Lowercase::new(&*op.function);
                let Some(sig) = self.function_signature(&class, &func, false) else {
                    return Err(InvalidInstructionError::UndefinedFunction {
                        class: op.class.clone(),
                        func: op.function.clone(),
                    });
                };

                if !sig.is_static {
                    return Err(InvalidInstructionError::CallStaticOnNonStatic);
                }

                if sig.params.len() != op.args.len() {
                    return Err(InvalidInstructionError::InvalidParameterCount {
                        expected: sig.params.len(),
                        found: op.args.len(),
                    });
                }

                for (index, (arg, param)) in op.args.iter().zip(sig.params).enumerate() {
                    let arg = variable_type(arg)?;
                    self.assert_coercible(OperandKey::Unnamed(index), &arg, &param)?;
                }

                let dst = variable_type(&op.return_assignment)?;
                self.assert_coercible(OperandKey::Named("dst"), &sig.returns, &dst)?;
            }
            Instruction::PropertyGet(op) => {
                let object = variable_type(&op.value)?;
                let dst = variable_type(&op.return_assignment)?;

                let prop =
                    extract_property_name(OperandKey::Named("object"), object, &op.property)?;
                assert_type_eq(OperandKey::Named("dst"), dst, prop)?;
            }
            Instruction::PropertySet(op) => {
                let object = variable_type(&op.object)?;
                let value = variable_type(&op.value)?;

                let prop =
                    extract_property_name(OperandKey::Named("object"), object, &op.property)?;
                assert_type_eq(OperandKey::Named("value"), value, prop)?;
            }
            Instruction::Is(op) => {
                let object = variable_type(&op.object)?;
                let dst = variable_type(&op.return_assignment)?;

                assert_type_generic(OperandKey::Named("object"), object, &[GenericType::Class])?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Bool)?;
            }
            Instruction::Cast(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;

                if !self.is_castable(src, dst.clone()) {
                    return Err(InvalidInstructionError::InvalidCast {
                        operand: OperandKey::Named("dst"),
                        found: dst,
                        // TODO
                        expected: Vec::new(),
                    });
                }
            }
            Instruction::Return(value) => {
                let value = variable_type(value)?;
                self.assert_coercible(OperandKey::Named("value"), &value, &return_ty)?;
            }
            Instruction::StringConcat(op) => {
                let lhs = variable_type(&op.lhs)?;
                let rhs = variable_type(&op.rhs)?;
                let dst = variable_type(&op.dst)?;

                assert_type_eq(OperandKey::Named("lhs"), lhs, Type::String)?;
                assert_type_eq(OperandKey::Named("rhs"), rhs, Type::String)?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::String)?
            }
            Instruction::StructCreate(op) => {
                let dst = variable_type(&op.return_assignment)?;
                assert_type_generic(OperandKey::Named("dst"), dst, &[GenericType::Struct])?;
            }
            Instruction::StructGet(op) => {
                let struct_ = variable_type(&op.struct_value)?;
                let dst = variable_type(&op.return_assignment)?;

                let member = extract_struct_member_type(
                    OperandKey::Named("struct"),
                    struct_,
                    &op.member_name,
                )?;
                assert_type_eq(OperandKey::Named("dst"), dst, member)?;
            }
            Instruction::StructSet(op) => {
                let struct_ = variable_type(&op.struct_value)?;
                let value = variable_type(&op.value)?;

                let member = extract_struct_member_type(
                    OperandKey::Named("struct"),
                    struct_,
                    &op.member_name,
                )?;
                assert_type_eq(OperandKey::Named("value"), value, member)?;
            }
            Instruction::ArrayCreate(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;

                assert_type_eq(OperandKey::Named("src"), src, Type::Int)?;
                assert_type_generic(OperandKey::Named("dst"), dst, &[GenericType::Array])?;
            }
            Instruction::ArrayLength(op) => {
                let src = variable_type(&op.src)?;
                let dst = variable_type(&op.dst)?;

                assert_type_generic(OperandKey::Named("src"), src, &[GenericType::Array])?;
                assert_type_eq(OperandKey::Named("dst"), dst, Type::Int)?;
            }
            Instruction::ArrayGet(op) => {
                let array = variable_type(&op.array)?;
                let index = variable_type(&op.index)?;
                let dst = variable_type(&op.return_assignment)?;

                let elem = extract_elem_ty(OperandKey::Named("array"), array)?;
                assert_type_eq(OperandKey::Named("index"), index, Type::Int)?;
                assert_type_eq(OperandKey::Named("dst"), dst, elem)?;
            }
            Instruction::ArraySet(op) => {
                let array = variable_type(&op.array)?;
                let index = variable_type(&op.index)?;
                let value = variable_type(&op.value)?;

                let elem = extract_elem_ty(OperandKey::Named("array"), array)?;
                assert_type_eq(OperandKey::Named("index"), index, Type::Int)?;
                assert_type_eq(OperandKey::Named("value"), value, elem)?;
            }
            Instruction::ArrayAdd(op) => {
                let array = variable_type(&op.array)?;
                let count = variable_type(&op.count)?;
                let value = variable_type(&op.value)?;

                let elem = extract_elem_ty(OperandKey::Named("array"), array)?;
                assert_type_eq(OperandKey::Named("count"), count, Type::Int)?;
                assert_type_eq(OperandKey::Named("value"), value, elem)?;
            }
            Instruction::ArrayInsert(op) => {
                let array = variable_type(&op.array)?;
                let index = variable_type(&op.index)?;
                let value = variable_type(&op.value)?;

                let elem = extract_elem_ty(OperandKey::Named("array"), array)?;
                assert_type_eq(OperandKey::Named("index"), index, Type::Int)?;
                assert_type_eq(OperandKey::Named("value"), value, elem)?;
            }
            Instruction::ArrayRemove(op) => {
                let array = variable_type(&op.array_value)?;
                let index = variable_type(&op.index)?;
                let count = variable_type(&op.count)?;

                assert_type_generic(OperandKey::Named("array"), array, &[GenericType::Array])?;
                assert_type_eq(OperandKey::Named("index"), index, Type::Int)?;
                assert_type_eq(OperandKey::Named("count"), count, Type::Int)?;
            }
            Instruction::ArrayPop(op) => {
                let array = variable_type(&op.array)?;
                assert_type_generic(OperandKey::Named("array"), array, &[GenericType::Array])?;
            }
            Instruction::ArrayClear(op) => {
                let array = variable_type(&op.array)?;
                assert_type_generic(OperandKey::Named("array"), array, &[GenericType::Array])?;
            }
            Instruction::ArrayFind(op) | Instruction::ArrayRFind(op) => {
                let array = variable_type(&op.array_value)?;
                let start_index = variable_type(&op.start_index)?;
                let return_index = variable_type(&op.return_index)?;
                let value = variable_type(&op.value)?;

                let elem = extract_elem_ty(OperandKey::Named("array"), array)?;
                assert_type_eq(OperandKey::Named("start_index"), start_index, Type::Int)?;
                assert_type_eq(OperandKey::Named("return_index"), return_index, Type::Int)?;
                assert_type_eq(OperandKey::Named("value"), value, elem)?;
            }
            Instruction::ArrayFindStruct(op) | Instruction::ArrayRFindStruct(op) => {
                let array = variable_type(&op.array_value)?;
                let start_index = variable_type(&op.start_index)?;
                let return_index = variable_type(&op.return_index)?;
                let value = variable_type(&op.value)?;

                let elem = extract_elem_ty(OperandKey::Named("array"), array)?;
                let member =
                    extract_struct_member_type(OperandKey::Named("array"), elem, &op.member_name)?;
                assert_type_eq(OperandKey::Named("start_index"), start_index, Type::Int)?;
                assert_type_eq(OperandKey::Named("return_index"), return_index, Type::Int)?;
                assert_type_eq(OperandKey::Named("value"), value, member)?;
            }
        }

        Ok(())
    }

    /// Checks whether `src` is assignable to `dst`.
    fn assert_coercible(
        &self,
        operand: OperandKey,
        src_ty: &Type,
        dst_ty: &Type,
    ) -> Result<(), InvalidInstructionError> {
        if src_ty == dst_ty {
            return Ok(());
        }

        match (&src_ty, &dst_ty) {
            // None can be assigned to any object.
            (Type::None, Type::Class(_)) => Ok(()),
            (Type::None, Type::Struct(_)) => Ok(()),
            (Type::None, Type::Array(_)) => Ok(()),
            // Storing an object is possible if src > dst,
            // i.e. dst is a parent of src.
            // Try to upcast until we have a match.
            (Type::Class(src), Type::Class(dst)) => {
                let mut src = src;

                loop {
                    if src == dst {
                        return Ok(());
                    }

                    let Some(class) = self.classes.get(src) else {
                        break;
                    };

                    if let Some(parent) = &class.parent {
                        src = parent;
                    } else {
                        break;
                    }
                }

                Err(InvalidInstructionError::InvalidCast {
                    operand,
                    found: src_ty.clone(),
                    expected: vec![GenericType::Concrete(dst_ty.clone())],
                })
            }
            _ => Err(InvalidInstructionError::InvalidCoersion {
                operand,
                found: dst_ty.clone(),
                expected: vec![GenericType::Concrete(src_ty.clone())],
            }),
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
                    let mut class_name = &src;
                    loop {
                        if *class_name == dst {
                            return true;
                        }

                        let class = self.classes.get(class_name).unwrap();
                        if let Some(parent) = &class.parent {
                            class_name = parent;
                        } else {
                            break;
                        }
                    }

                    // Try upcasting.
                    let mut class_name = &dst;
                    loop {
                        if *class_name == src {
                            return true;
                        }

                        let class = self.classes.get(class_name).unwrap();
                        if let Some(parent) = &class.parent {
                            class_name = parent;
                        } else {
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
}
