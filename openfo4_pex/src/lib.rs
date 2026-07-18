//! Parser for the PEX file format used in compiled scripts.

mod raw;
mod text;

const MAGIC_LE: [u8; 4] = [0xde, 0xc0, 0x57, 0xfa];
const MAGIC_BE: [u8; 4] = [0xfa, 0x57, 0xc0, 0xde];

const MAJOR_VERSION: u8 = 3;

use bytes::Buf;

use crate::raw::{DebugInfo, FunctionFlags, Header, Opcode, UserFlag};

#[derive(Clone, Debug)]
pub enum Error {
    Eof,
    BadMagic([u8; 4]),
    UnknownVersion(u8),
    Other(String),
    BadEnumValue(&'static str, u64),
    InvalidString(std::string::FromUtf8Error),
    InvalidStringPointer(u16),
    InvalidInstructionArgCount {
        op: Opcode,
        expected: usize,
        found: usize,
    },
    InvalidValue {
        op: Opcode,
        operand: &'static str,
        value: Value,
    },
}

#[derive(Clone, Debug)]
pub struct Module {
    pub classes: Vec<Class>,
}

impl Module {
    pub fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let mut magic = [0; 4];
        buf.try_copy_to_slice(&mut magic).map_err(|_| Error::Eof)?;

        let endian = match magic {
            MAGIC_LE => Endianess::Little,
            MAGIC_BE => Endianess::Big,
            _ => return Err(Error::BadMagic(magic)),
        };

        let mut reader = Reader { buf, endian };

        let header = Header::decode(&mut reader)?;

        if header.major_version != MAJOR_VERSION {
            return Err(Error::UnknownVersion(header.major_version));
        }

        let strings = Vec::<String>::decode(&mut reader)?;

        // If debug info is present we must read the entire debug info
        // because we don't know how long it is, so we cannot skip it.
        let has_debug_info = u8::decode(&mut reader)?;
        if has_debug_info != 0 {
            DebugInfo::decode(&mut reader)?;
        }

        // User flags are only used in the editor. They are not used
        // in the execution environment.
        Vec::<UserFlag>::decode(&mut reader)?;

        let raw_classes = Vec::<raw::Class>::decode(&mut reader)?;

        let mut classes = Vec::new();
        for class in raw_classes {
            let name = strings
                .get(usize::from(class.name))
                .ok_or(Error::InvalidStringPointer(class.name))?;

            let parent = strings
                .get(usize::from(class.parent_class_name))
                .ok_or(Error::InvalidStringPointer(class.parent_class_name))?;

            let mut structs = Vec::new();
            for st in &class.structs {
                let name = strings
                    .get(usize::from(st.name))
                    .ok_or(Error::InvalidStringPointer(st.name))?;

                let mut members = Vec::new();
                for member in &st.members {
                    let name = strings
                        .get(usize::from(member.name))
                        .ok_or(Error::InvalidStringPointer(member.name))?;

                    let ty = strings
                        .get(usize::from(member.type_name))
                        .ok_or(Error::InvalidStringPointer(member.type_name))?;

                    members.push(StructMember {
                        name: name.to_owned(),
                        ty: ty.to_owned(),
                    });
                }

                structs.push(Struct {
                    name: name.to_owned(),
                    members,
                });
            }

            let mut variables = Vec::new();
            for variable in &class.variables {
                let name = strings
                    .get(usize::from(variable.definition.name))
                    .ok_or(Error::InvalidStringPointer(variable.definition.name))?;

                let ty = strings
                    .get(usize::from(variable.definition.type_name))
                    .ok_or(Error::InvalidStringPointer(variable.definition.type_name))?;

                let value = Value::convert(&variable.value, &strings)?;

                variables.push(Variable {
                    definition: VariableDefinition {
                        name: name.to_owned(),
                        ty: ty.to_owned(),
                    },
                    is_const: variable.is_const,
                    value,
                });
            }

            let mut properties = Vec::new();
            for property in &class.properties {
                let name = strings
                    .get(usize::from(property.name))
                    .ok_or(Error::InvalidStringPointer(property.name))?;

                let ty = strings
                    .get(usize::from(property.type_name))
                    .ok_or(Error::InvalidStringPointer(property.type_name))?;

                let mut variable = None;
                if let Some(v) = property.autovar_name {
                    let var_name = strings
                        .get(usize::from(v))
                        .ok_or(Error::InvalidStringPointer(v))?;
                    variable = Some(var_name.to_owned());
                }

                let mut getter = None;
                if let Some(f) = &property.getter {
                    let function = convert_function(f, &strings)?;
                    getter = Some(function);
                }

                let mut setter = None;
                if let Some(f) = &property.setter {
                    let function = convert_function(f, &strings)?;
                    setter = Some(function);
                }

                properties.push(Property {
                    name: name.to_owned(),
                    ty: ty.to_owned(),
                    variable,
                    getter,
                    setter,
                });
            }

            let default_state = strings
                .get(usize::from(class.auto_state_name))
                .ok_or(Error::InvalidStringPointer(class.auto_state_name))?;

            let mut states = Vec::new();
            for state in &class.states {
                let name = strings
                    .get(usize::from(state.name))
                    .ok_or(Error::InvalidStringPointer(state.name))?;

                let mut functions = Vec::new();
                for function in &state.functions {
                    let name = strings
                        .get(usize::from(function.name))
                        .ok_or(Error::InvalidStringPointer(function.name))?;

                    let mut function = convert_function(&function.function, &strings)?;
                    function.name = name.to_owned();

                    functions.push(function);
                }

                states.push(State {
                    name: name.to_owned(),
                    functions,
                });
            }

            classes.push(Class {
                name: name.to_owned(),
                parent: parent.to_owned(),
                structs,
                states,
                default_state: default_state.to_owned(),
                variables,
                properties,
            });
        }

        Ok(Self { classes })
    }
}

fn convert_function(f: &raw::Function, strings: &[String]) -> Result<Function, Error> {
    let return_ty = strings
        .get(usize::from(f.return_type_name))
        .ok_or(Error::InvalidStringPointer(f.return_type_name))?;

    let mut params = Vec::new();
    for param in &f.params {
        let name = strings
            .get(usize::from(param.name))
            .ok_or(Error::InvalidStringPointer(param.name))?;

        let ty = strings
            .get(usize::from(param.type_name))
            .ok_or(Error::InvalidStringPointer(param.type_name))?;

        params.push(VariableDefinition {
            name: name.to_owned(),
            ty: ty.to_owned(),
        });
    }

    let mut locals = Vec::new();
    for local in &f.locals {
        let name = strings
            .get(usize::from(local.name))
            .ok_or(Error::InvalidStringPointer(local.name))?;

        let ty = strings
            .get(usize::from(local.type_name))
            .ok_or(Error::InvalidStringPointer(local.type_name))?;

        locals.push(VariableDefinition {
            name: name.to_owned(),
            ty: ty.to_owned(),
        });
    }

    let mut instructions = Vec::new();
    for instruction in &f.instructions {
        let instruction = Instruction::convert(instruction, strings)?;
        instructions.push(instruction);
    }

    Ok(Function {
        name: String::new(),
        is_static: f.flags.contains(FunctionFlags::STATIC),
        is_native: f.flags.contains(FunctionFlags::NATIVE),
        params,
        return_ty: return_ty.to_owned(),
        locals,
        instructions,
    })
}

#[derive(Clone, Debug)]
pub struct Class {
    /// The name of the class.
    pub name: String,
    /// The name of the parent class.
    pub parent: String,
    pub structs: Vec<Struct>,
    pub states: Vec<State>,
    pub default_state: String,
    pub variables: Vec<Variable>,
    pub properties: Vec<Property>,
}

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: String,
    pub members: Vec<StructMember>,
}

#[derive(Clone, Debug)]
pub struct StructMember {
    /// The unique identifier of the struct member.
    pub name: String,
    /// The type of the struct member.
    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct Property {
    /// The name of the property.
    pub name: String,
    /// The type of the property.
    pub ty: String,
    pub variable: Option<String>,
    /// Optional custom getter function.
    ///
    /// This function should be called when reading the property.
    pub getter: Option<Function>,
    /// Optional custom setter function.
    ///
    /// This function should be called when writing to the property.
    pub setter: Option<Function>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub name: String,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    /// Indicates whether the function is static.
    pub is_static: bool,
    /// Indicates whether the function is native.
    ///
    /// A native function contains no instructions and must be provided by the engine.
    pub is_native: bool,
    pub params: Vec<VariableDefinition>,
    pub return_ty: String,
    pub locals: Vec<VariableDefinition>,
    pub instructions: Vec<Instruction>,
}

impl Function {
    pub fn text(&self) -> String {
        text::for_function(self)
    }
}

#[derive(Clone, Debug)]
pub struct VariableDefinition {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub definition: VariableDefinition,
    /// Indicates whether the variable is constant.
    ///
    /// A constant variable will never change after linking.
    pub is_const: bool,
    /// The initial value of the variable at link time.
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// No operation
    Nop,
    /// 32-bit integer addition.
    ///
    /// Computes `lhs` + `rhs` and stores the result in `dst`.
    IAdd(Binop),
    /// 32-bit float addition.
    ///
    /// Computes `lhs` + `rhs` and stores the result in `dst`.
    FAdd(Binop),
    /// 32-bit integer subtraction.
    ///
    /// Computes `lhs` - `rhs` and stores the result in `dst`.
    ISub(Binop),
    FSub(Binop),
    IMul(Binop),
    FMul(Binop),
    IDiv(Binop),
    FDiv(Binop),
    IMod(Binop),
    Not(Unop),
    INeg(Unop),
    FNeg(Unop),
    Store(Unop),
    Cast(Unop),
    CmpEq(Binop),
    CmpLt(Binop),
    CmpLe(Binop),
    CmpGt(Binop),
    CmpGe(Binop),
    Branch(Branch),
    BranchIfTrue(ConditionalBranch),
    BranchIfFalse(ConditionalBranch),
    CallMethod(CallMethod),
    CallParent(CallParent),
    CallStatic(CallStatic),
    Return(Value),
    StringConcat(Binop),
    PropertyGet(PropertyGet),
    PropertySet(PropertySet),
    ArrayCreate(Unop),
    ArrayLength(Unop),
    ArrayGet(ArrayGet),
    ArraySet(ArraySet),
    ArrayFind(ArrayFind),
    ArrayRFind(ArrayFind),
    Is(Is),
    StructCreate(StructCreate),
    StructGet(StructGet),
    StructSet(StructSet),
    ArrayFindStruct(ArrayFindStruct),
    ArrayRFindStruct(ArrayFindStruct),
    ArrayAdd(ArrayAdd),
    ArrayInsert(ArrayInsert),
    ArrayPop(ArrayPop),
    ArrayRemove(ArrayRemove),
    ArrayClear(ArrayClear),
}

impl Instruction {
    /// Returns the textural representation of the `Instruction`.
    pub fn text(&self) -> String {
        let mut buf = String::new();
        text::for_instruction(self, &mut buf).unwrap();
        buf
    }

    fn convert(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let read_unop = || -> Result<Unop, Error> {
            if instruction.args.len() != 2 {
                return Err(Error::InvalidInstructionArgCount {
                    op: instruction.op,
                    expected: 2,
                    found: instruction.args.len(),
                });
            }

            let dst = Value::convert(&instruction.args[0], strings)?;
            let src = Value::convert(&instruction.args[1], strings)?;

            Ok(Unop { src, dst })
        };

        let read_binop = || -> Result<Binop, Error> {
            if instruction.args.len() != 3 {
                return Err(Error::InvalidInstructionArgCount {
                    op: instruction.op,
                    expected: 3,
                    found: instruction.args.len(),
                });
            }

            let dst = Value::convert(&instruction.args[0], strings)?;
            let lhs = Value::convert(&instruction.args[1], strings)?;
            let rhs = Value::convert(&instruction.args[2], strings)?;

            Ok(Binop { lhs, rhs, dst })
        };

        match instruction.op {
            Opcode::Nop => Ok(Self::Nop),
            Opcode::IAdd => read_binop().map(Self::IAdd),
            Opcode::FAdd => read_binop().map(Self::FAdd),
            Opcode::ISub => read_binop().map(Self::ISub),
            Opcode::FSub => read_binop().map(Self::FSub),
            Opcode::IMul => read_binop().map(Self::IMul),
            Opcode::FMul => read_binop().map(Self::FMul),
            Opcode::IDiv => read_binop().map(Self::IDiv),
            Opcode::FDiv => read_binop().map(Self::FDiv),
            Opcode::IMod => read_binop().map(Self::IMod),
            Opcode::Not => read_unop().map(Self::Not),
            Opcode::INeg => read_unop().map(Self::INeg),
            Opcode::FNeg => read_unop().map(Self::FNeg),
            Opcode::Store => read_unop().map(Self::Store),
            Opcode::Cast => read_unop().map(Self::Cast),
            Opcode::CmpEq => read_binop().map(Self::CmpEq),
            Opcode::CmpLt => read_binop().map(Self::CmpLt),
            Opcode::CmpLe => read_binop().map(Self::CmpLe),
            Opcode::CmpGt => read_binop().map(Self::CmpGt),
            Opcode::CmpGe => read_binop().map(Self::CmpGe),
            Opcode::Branch => Branch::from_raw(instruction, strings).map(Self::Branch),
            Opcode::BranchIfTrue => {
                ConditionalBranch::from_raw(instruction, strings).map(Self::BranchIfTrue)
            }
            Opcode::BranchIfFalse => {
                ConditionalBranch::from_raw(instruction, strings).map(Self::BranchIfFalse)
            }
            Opcode::CallMethod => CallMethod::from_raw(instruction, strings).map(Self::CallMethod),
            Opcode::CallParent => CallParent::from_raw(instruction, strings).map(Self::CallParent),
            Opcode::CallStatic => CallStatic::from_raw(instruction, strings).map(Self::CallStatic),
            Opcode::Return => Value::convert(&instruction.args[0], strings).map(Self::Return),
            Opcode::StringConcat => read_binop().map(Self::StringConcat),
            Opcode::PropertyGet => {
                PropertyGet::from_raw(instruction, strings).map(Self::PropertyGet)
            }
            Opcode::PropertySet => {
                PropertySet::from_raw(instruction, strings).map(Self::PropertySet)
            }
            Opcode::ArrayCreate => read_unop().map(Self::ArrayCreate),
            Opcode::ArrayLength => read_unop().map(Self::ArrayLength),
            Opcode::ArrayGet => ArrayGet::from_raw(instruction, strings).map(Self::ArrayGet),
            Opcode::ArraySet => ArraySet::from_raw(instruction, strings).map(Self::ArraySet),
            Opcode::ArrayFind => ArrayFind::from_raw(instruction, strings).map(Self::ArrayFind),
            Opcode::ArrayRFind => ArrayFind::from_raw(instruction, strings).map(Self::ArrayRFind),
            Opcode::Is => Is::from_raw(instruction, strings).map(Self::Is),
            Opcode::StructCreate => {
                StructCreate::from_raw(instruction, strings).map(Self::StructCreate)
            }
            Opcode::StructGet => StructGet::from_raw(instruction, strings).map(Self::StructGet),
            Opcode::StructSet => StructSet::from_raw(instruction, strings).map(Self::StructSet),
            Opcode::ArrayFindStruct => {
                ArrayFindStruct::from_raw(instruction, strings).map(Self::ArrayFindStruct)
            }
            Opcode::ArrayRFindStruct => {
                ArrayFindStruct::from_raw(instruction, strings).map(Self::ArrayRFindStruct)
            }
            Opcode::ArrayAdd => ArrayAdd::from_raw(instruction, strings).map(Self::ArrayAdd),
            Opcode::ArrayInsert => {
                ArrayInsert::from_raw(instruction, strings).map(Self::ArrayInsert)
            }
            Opcode::ArrayPop => ArrayPop::from_raw(instruction, strings).map(Self::ArrayPop),
            Opcode::ArrayRemove => {
                ArrayRemove::from_raw(instruction, strings).map(Self::ArrayRemove)
            }
            Opcode::ArrayClear => ArrayClear::from_raw(instruction, strings).map(Self::ArrayClear),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unop {
    pub src: Value,
    pub dst: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Binop {
    pub lhs: Value,
    pub rhs: Value,
    pub dst: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub offset: i32,
}

impl Branch {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        if instruction.args.len() != 1 {
            return Err(Error::InvalidInstructionArgCount {
                op: instruction.op,
                expected: 1,
                found: instruction.args.len(),
            });
        }

        let offset = Value::convert(&instruction.args[0], strings)?;
        let Value::Int(offset) = offset else {
            return Err(Error::InvalidValue {
                op: Opcode::Branch,
                operand: "offset",
                value: offset,
            });
        };

        Ok(Self { offset })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalBranch {
    pub condition: Value,
    pub offset: i32,
}

impl ConditionalBranch {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        if instruction.args.len() != 2 {
            return Err(Error::InvalidInstructionArgCount {
                op: instruction.op,
                expected: 2,
                found: instruction.args.len(),
            });
        }

        let condition = Value::convert(&instruction.args[0], strings)?;
        let offset = Value::convert(&instruction.args[1], strings)?;
        let Value::Int(offset) = offset else {
            return Err(Error::InvalidValue {
                op: instruction.op,
                operand: "offset",
                value: offset,
            });
        };

        Ok(ConditionalBranch { condition, offset })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallMethod {
    pub function_name: String,
    /// The receiver object on which the function is called.
    pub object: Value,
    /// The variable that will store the return value.
    pub return_assignment: Value,
    pub args: Vec<Value>,
}

impl CallMethod {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let function_name = Value::convert(&instruction.args[0], strings)?;
        let Value::Ident(function_name) = function_name else {
            return Err(Error::InvalidValue {
                op: Opcode::CallMethod,
                operand: "function_name",
                value: function_name,
            });
        };

        let object = Value::convert(&instruction.args[1], strings)?;
        let return_assignment = Value::convert(&instruction.args[2], strings)?;

        let args = instruction.args[3..]
            .iter()
            .map(|v| Value::convert(v, strings))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            function_name,
            object,
            return_assignment,
            args,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallParent {
    /// The name of the function to call.
    pub function: String,
    pub return_assignment: Value,
    pub args: Vec<Value>,
}

impl CallParent {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let function = Value::convert(&instruction.args[0], strings)?;
        let Value::Ident(function) = function else {
            return Err(Error::InvalidValue {
                op: Opcode::CallParent,
                operand: "function",
                value: function,
            });
        };

        let return_assignment = Value::convert(&instruction.args[1], strings)?;

        let args = instruction.args[2..]
            .iter()
            .map(|v| Value::convert(v, strings))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            function,
            return_assignment,
            args,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallStatic {
    pub class: String,
    pub function: String,
    pub return_assignment: Value,
    pub args: Vec<Value>,
}

impl CallStatic {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let class = Value::convert(&instruction.args[0], strings)?;
        let Value::Ident(class) = class else {
            return Err(Error::InvalidValue {
                op: Opcode::CallStatic,
                operand: "class",
                value: class,
            });
        };

        let function = Value::convert(&instruction.args[1], strings)?;
        let Value::Ident(function) = function else {
            return Err(Error::InvalidValue {
                op: Opcode::CallStatic,
                operand: "function",
                value: function,
            });
        };

        let return_assignment = Value::convert(&instruction.args[2], strings)?;

        let args = instruction.args[3..]
            .iter()
            .map(|v| Value::convert(v, strings))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            class,
            function,
            return_assignment,
            args,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyGet {
    /// The name of the property to load.
    pub property: String,
    /// The value that contains the property to be loaded.
    pub value: Value,
    /// The variable where the loaded value should be stored.
    pub return_assignment: Value,
}

impl PropertyGet {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let property = Value::convert(&instruction.args[0], strings)?;
        let Value::Ident(property) = property else {
            return Err(Error::InvalidValue {
                op: Opcode::PropertyGet,
                operand: "property",
                value: property,
            });
        };

        // TODO: Double check that this is the correct order:
        let value = Value::convert(&instruction.args[1], strings)?;
        let return_assignment = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            property,
            value,
            return_assignment,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertySet {
    /// The name of the property.
    pub property: String,
    /// The object with the property.
    pub object: Value,
    /// The new value to store.
    pub value: Value,
}

impl PropertySet {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let property = Value::convert(&instruction.args[0], strings)?;
        let Value::Ident(property) = property else {
            return Err(Error::InvalidValue {
                op: Opcode::PropertySet,
                operand: "property",
                value: property,
            });
        };

        let object = Value::convert(&instruction.args[1], strings)?;
        let value = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            property,
            object,
            value,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayGet {
    pub array: Value,
    pub index: Value,
    pub return_assignment: Value,
}

impl ArrayGet {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let return_assignment = Value::convert(&instruction.args[0], strings)?;
        let array = Value::convert(&instruction.args[1], strings)?;
        let index = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            array,
            index,
            return_assignment,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArraySet {
    pub array: Value,
    pub index: Value,
    pub value: Value,
}

impl ArraySet {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array = Value::convert(&instruction.args[0], strings)?;
        let index = Value::convert(&instruction.args[1], strings)?;
        let value = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            array,
            index,
            value,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayFind {
    /// The array object to be searched.
    pub array_value: Value,
    /// The variable in which the return index will be written.
    ///
    /// `-1` should be written if the value is not found.
    pub return_index: Value,
    /// The value that should be searched for.
    pub value: Value,
    /// The index where to start searching from.
    pub start_index: Value,
}

impl ArrayFind {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array_value = Value::convert(&instruction.args[0], strings)?;
        let return_index = Value::convert(&instruction.args[1], strings)?;
        let value = Value::convert(&instruction.args[2], strings)?;
        let start_index = Value::convert(&instruction.args[3], strings)?;

        Ok(Self {
            array_value,
            return_index,
            value,
            start_index,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Is {
    /// The variable where the bool will be written.
    pub return_assignment: Value,
    /// The object that will be checked.
    pub object: Value,
    /// The class name that will be checked for.
    pub class: String,
}

impl Is {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let return_assignment = Value::convert(&instruction.args[0], strings)?;
        let object = Value::convert(&instruction.args[1], strings)?;

        let class = Value::convert(&instruction.args[2], strings)?;
        let Value::Ident(class) = class else {
            return Err(Error::InvalidValue {
                op: Opcode::Is,
                operand: "class",
                value: class,
            });
        };

        Ok(Self {
            return_assignment,
            object,
            class,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructCreate {
    pub return_assignment: Value,
}

impl StructCreate {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let return_assignment = Value::convert(&instruction.args[0], strings)?;

        Ok(Self { return_assignment })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructGet {
    /// The struct value.
    pub struct_value: Value,
    /// The value in which the loaded member value will be stored.
    pub return_assignment: Value,
    /// The name of the member value to load.
    pub member_name: String,
}

impl StructGet {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let return_assignment = Value::convert(&instruction.args[0], strings)?;
        let struct_value = Value::convert(&instruction.args[1], strings)?;

        let member_name = Value::convert(&instruction.args[2], strings)?;
        let Value::Ident(member_name) = member_name else {
            return Err(Error::InvalidValue {
                op: Opcode::StructGet,
                operand: "member_name",
                value: member_name,
            });
        };

        Ok(Self {
            struct_value,
            return_assignment,
            member_name,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructSet {
    /// The struct value.
    pub struct_value: Value,
    /// The name of the member.
    pub member_name: String,
    /// The new value.
    pub value: Value,
}

impl StructSet {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let struct_value = Value::convert(&instruction.args[0], strings)?;
        let value = Value::convert(&instruction.args[2], strings)?;

        let member_name = Value::convert(&instruction.args[1], strings)?;
        let Value::Ident(member_name) = member_name else {
            return Err(Error::InvalidValue {
                op: Opcode::StructGet,
                operand: "member_name",
                value: member_name,
            });
        };

        Ok(Self {
            struct_value,
            member_name,
            value,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayFindStruct {
    /// The array object.
    pub array_value: Value,
    /// The variable in which the index will be written.
    ///
    /// `-1` should be written if the value is not found.
    pub return_index: Value,
    /// The name of the member to search.
    pub member_name: String,
    /// The value to search for in the given member.
    pub value: Value,
    /// The starting index within the array.
    pub start_index: Value,
}

impl ArrayFindStruct {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array_value = Value::convert(&instruction.args[0], strings)?;
        let return_index = Value::convert(&instruction.args[1], strings)?;

        let member_name = Value::convert(&instruction.args[2], strings)?;
        let Value::String(member_name) = member_name else {
            return Err(Error::InvalidValue {
                op: Opcode::StructGet,
                operand: "member_name",
                value: member_name,
            });
        };

        let value = Value::convert(&instruction.args[3], strings)?;
        let start_index = Value::convert(&instruction.args[4], strings)?;

        Ok(Self {
            array_value,
            return_index,
            member_name,
            value,
            start_index,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayAdd {
    pub array: Value,
    pub value: Value,
    pub count: Value,
}

impl ArrayAdd {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array = Value::convert(&instruction.args[0], strings)?;
        let value = Value::convert(&instruction.args[1], strings)?;
        let count = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            array,
            value,
            count,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayInsert {
    pub array: Value,
    pub value: Value,
    pub index: Value,
}

impl ArrayInsert {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array = Value::convert(&instruction.args[0], strings)?;
        let value = Value::convert(&instruction.args[1], strings)?;
        let index = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            array,
            value,
            index,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayPop {
    pub array: Value,
}

impl ArrayPop {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array = Value::convert(&instruction.args[0], strings)?;

        Ok(Self { array })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayRemove {
    pub array_value: Value,
    pub index: Value,
    pub count: Value,
}

impl ArrayRemove {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array_value = Value::convert(&instruction.args[0], strings)?;
        let index = Value::convert(&instruction.args[1], strings)?;
        let count = Value::convert(&instruction.args[2], strings)?;

        Ok(Self {
            array_value,
            index,
            count,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayClear {
    /// The array that will be cleared.
    pub array: Value,
}

impl ArrayClear {
    fn from_raw(instruction: &raw::Instruction, strings: &[String]) -> Result<Self, Error> {
        let array = Value::convert(&instruction.args[0], strings)?;

        Ok(Self { array })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Ident(String),
}

impl Value {
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub const fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub const fn is_ident(&self) -> bool {
        matches!(self, Self::Ident(_))
    }

    fn convert(value: &raw::Value, strings: &[String]) -> Result<Self, Error> {
        match value {
            raw::Value::Null => Ok(Self::Null),
            raw::Value::Bool(val) => Ok(Self::Bool(*val)),
            raw::Value::Int(val) => Ok(Self::Int(*val)),
            raw::Value::Float(val) => Ok(Self::Float(*val)),
            raw::Value::String(index) => {
                let s = strings
                    .get(usize::from(*index))
                    .ok_or(Error::InvalidStringPointer(*index))?;
                Ok(Self::String(s.to_owned()))
            }
            raw::Value::Identifier(index) => {
                let s = strings
                    .get(usize::from(*index))
                    .ok_or(Error::InvalidStringPointer(*index))?;
                Ok(Self::Ident(s.to_owned()))
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Reader<B> {
    buf: B,
    endian: Endianess,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Endianess {
    Little,
    Big,
}

trait Decode: Sized {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf;
}

macro_rules! impl_decode_primitive {
    ($($t:ty),*) => {
        $(
            impl Decode for $t {
                fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
                where
                    B: Buf,
                {
                    if reader.buf.remaining() >= size_of::<Self>() {
                        let mut bytes = [0; size_of::<Self>()];
                        reader.buf.copy_to_slice(&mut bytes);

                        match reader.endian {
                            Endianess::Little => Ok(Self::from_le_bytes(bytes)),
                            Endianess::Big => Ok(Self::from_be_bytes(bytes)),
                        }
                    } else {
                        Err(Error::Eof)
                    }
                }
            }
        )*
    };
}

impl_decode_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl Decode for bool {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        u8::decode(reader).map(|v| v != 0)
    }
}

impl Decode for String {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        let len = u16::decode(reader)?;
        let mut bytes = Vec::with_capacity(usize::min(len.into(), reader.buf.remaining()));
        for _ in 0..len {
            let b = u8::decode(reader)?;
            bytes.push(b);
        }

        Self::from_utf8(bytes).map_err(Error::InvalidString)
    }
}
