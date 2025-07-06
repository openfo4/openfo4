use bitflags::bitflags;
use bytes::Buf;

use crate::{Decode, Error, Reader};

impl<T> Decode for Vec<T>
where
    T: Decode,
{
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        let len = u16::decode(reader)?;

        let mut elems = Vec::new();
        for _ in 0..len {
            let elem = T::decode(reader)?;
            elems.push(elem);
        }

        Ok(elems)
    }
}

macro_rules! binary_struct {
    ( $(#[$($struct_attr:tt)*])* $struct_vis:vis struct $struct_name:ident {
        $(
            $(#[$($field_attr:tt)*])* $field_vis:vis $field_name:ident : $field_ty:ty,
        )*
    }) => {
        // Recreate the struct definition.
        $(
            #[ $( $struct_attr )* ]
        )*
        $struct_vis struct $struct_name {
            $(
                $(#[$($field_attr)*])* $field_vis $field_name : $field_ty,
            )*
        }

        impl Decode for $struct_name {
            fn decode<B>(_reader: &mut Reader<B>) -> Result<Self, Error>
            where
                B: Buf,
            {
                Ok(Self {
                    $(
                        $field_name: <$field_ty as Decode>::decode(_reader)?,
                    )*
                })
            }
        }
    };
}

macro_rules! impl_decode_for_bitflags {
    ($t:ty: $int:ty) => {
        impl Decode for $t {
            fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
            where
                B: Buf,
            {
                <$int as Decode>::decode(reader).map(Self::from_bits_truncate)
            }
        }
    };
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct Header {
        pub major_version: u8,
        pub minor_version: u8,
        pub game_id: u16,
        pub compilation_time: u64,
        pub source_filename: String,
        pub username: String,
        pub machine_name: String,
    }
}

binary_struct! {
    #[derive(Copy, Clone, Debug)]
    pub struct UserFlag {
        pub name_index: u16,
        pub flag_index: u8,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct DebugInfo {
        pub modification_time: u64,
        pub functions: Vec<FunctionDebugInfo>,
        pub property_groups: Vec<PropertyGroupDebugInfo>,
        pub structs: Vec<StructDebugInfo>,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct FunctionDebugInfo {
        pub class_name: u16,
        pub state_name: u16,
        pub function_name: u16,
        pub function_type: FunctionType,
        pub lines: Vec<u16>,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionType {
    Unknown,
}

impl Decode for FunctionType {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        match u8::decode(reader)? {
            _ => Ok(Self::Unknown),
        }
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct PropertyGroupDebugInfo {
        pub class_name: u16,
        pub group_name: u16,
        pub doc_string: u16,
        pub user_flags: u32,
        pub names: Vec<u16>,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct StructDebugInfo {
        pub class_name: u16,
        pub struct_name: u16,
        pub member_names: Vec<u16>,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct Class {
        pub name: u16,
        _size: u32,
        pub parent_class_name: u16,
        pub doc_string: u16,
        pub is_const: bool,
        pub user_flags: u32,
        pub auto_state_name: u16,
        pub structs: Vec<StructInfo>,
        pub variables: Vec<Variable>,
        pub properties: Vec<Property>,
        pub states: Vec<State>,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct StructInfo {
        pub name: u16,
        pub members: Vec<StructMemberInfo>,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct StructMemberInfo {
        pub name: u16,
        pub type_name: u16,
        pub user_flags: u32,
        pub value: Value,
        pub is_const: bool,
        pub doc_string: u16,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct VariableDefinition {
        pub name: u16,
        pub type_name: u16,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct Variable {
        pub definition: VariableDefinition,
        pub user_flags: u32,
        pub value: Value,
        pub is_const: bool,
    }
}

#[derive(Clone, Debug)]
pub struct Property {
    pub name: u16,
    pub type_name: u16,
    pub doc_string: u16,
    pub user_flags: u32,
    pub autovar_name: Option<u16>,
    pub getter: Option<Function>,
    pub setter: Option<Function>,
}

impl Decode for Property {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        let name = u16::decode(reader)?;
        let type_name = u16::decode(reader)?;
        let doc_string = u16::decode(reader)?;
        let user_flags = u32::decode(reader)?;
        let flags = u8::decode(reader)?;

        let mut autovar_name = None;
        let mut getter = None;
        let mut setter = None;

        if flags & 0x04 != 0 {
            autovar_name = Some(u16::decode(reader)?);
        } else {
            if flags & 0x01 != 0 {
                getter = Some(Function::decode(reader)?);
            }

            if flags & 0x02 != 0 {
                setter = Some(Function::decode(reader)?);
            }
        }

        Ok(Self {
            name,
            type_name,
            doc_string,
            user_flags,
            autovar_name,
            getter,
            setter,
        })
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct Function {
        pub return_type_name: u16,
        pub doc_string: u16,
        pub user_flags: u32,
        pub flags: FunctionFlags,
        pub params: Vec<VariableDefinition>,
        pub locals: Vec<VariableDefinition>,
        pub instructions: Vec<Instruction>,
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct FunctionFlags: u8 {
        const STATIC = 0x01;
        const NATIVE = 0x02;
    }
}

impl_decode_for_bitflags!(FunctionFlags: u8);

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct State {
        pub name: u16,
        pub functions: Vec<NamedFunction>,
    }
}

binary_struct! {
    #[derive(Clone, Debug)]
    pub struct NamedFunction {
        pub name: u16,
        pub function: Function,
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Identifier(u16),
    String(u16),
    Int(i32),
    Float(f32),
    Bool(bool),
}

impl Decode for Value {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        let ty = u8::decode(reader)?;

        match ty {
            0 => Ok(Self::Null),
            1 => {
                let value = u16::decode(reader)?;
                Ok(Self::Identifier(value))
            }
            2 => {
                let value = u16::decode(reader)?;
                Ok(Self::String(value))
            }
            3 => {
                let value = i32::decode(reader)?;
                Ok(Self::Int(value))
            }
            4 => {
                let value = f32::decode(reader)?;
                Ok(Self::Float(value))
            }
            5 => {
                let value = bool::decode(reader)?;
                Ok(Self::Bool(value))
            }
            val => Err(Error::BadEnumValue("Value", val.into())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub op: Opcode,
    pub args: Vec<Value>,
}

impl Decode for Instruction {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        let op = Opcode::decode(reader)?;

        let mut args = Vec::new();
        for _ in 0..op.arg_count() {
            let arg = Value::decode(reader)?;
            args.push(arg);
        }

        if op.has_variable_arg_count() {
            let arg_count = Value::decode(reader)?;
            let Value::Int(count) = arg_count else {
                return Err(Error::Other(format!(
                    "opcode {:?} requires variable data with integer, was: {:?}",
                    op, arg_count
                )));
            };

            for _ in 0..count {
                let arg = Value::decode(reader)?;
                args.push(arg);
            }
        }

        Ok(Self { op, args })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Opcode {
    Nop,
    IAdd,
    FAdd,
    ISub,
    FSub,
    IMul,
    FMul,
    IDiv,
    FDiv,
    IMod,
    Not,
    INeg,
    FNeg,
    Store,
    Cast,
    CmpEq,
    CmpLt,
    CmpLe,
    CmpGt,
    CmpGe,
    Branch,
    BranchIfTrue,
    BranchIfFalse,
    CallMethod,
    CallParent,
    CallStatic,
    Return,
    StringConcat,
    PropertyGet,
    PropertySet,
    ArrayCreate,
    ArrayLength,
    ArrayGet,
    ArraySet,
    ArrayFind,
    ArrayRFind,
    Is,
    StructCreate,
    StructGet,
    StructSet,
    ArrayFindStruct,
    ArrayRFindStruct,
    ArrayAdd,
    ArrayInsert,
    ArrayPop,
    ArrayRemove,
    ArrayClear,
}

impl Opcode {
    fn arg_count(&self) -> u32 {
        match self {
            Self::Nop => 0,
            Self::IAdd => 3,
            Self::FAdd => 3,
            Self::ISub => 3,
            Self::FSub => 3,
            Self::IMul => 3,
            Self::FMul => 3,
            Self::IDiv => 3,
            Self::FDiv => 3,
            Self::IMod => 3,
            Self::Not => 2,
            Self::INeg => 2,
            Self::FNeg => 2,
            Self::Store => 2,
            Self::Cast => 2,
            Self::CmpEq => 3,
            Self::CmpLt => 3,
            Self::CmpLe => 3,
            Self::CmpGt => 3,
            Self::CmpGe => 3,
            Self::Branch => 1,
            Self::BranchIfTrue => 2,
            Self::BranchIfFalse => 2,
            Self::CallMethod => 3,
            Self::CallParent => 2,
            Self::CallStatic => 3,
            Self::Return => 1,
            Self::StringConcat => 3,
            Self::PropertyGet => 3,
            Self::PropertySet => 3,
            Self::ArrayCreate => 2,
            Self::ArrayLength => 2,
            Self::ArrayGet => 3,
            Self::ArraySet => 3,
            Self::ArrayFind => 4,
            Self::ArrayRFind => 4,
            Self::Is => 3,
            Self::StructCreate => 1,
            Self::StructGet => 3,
            Self::StructSet => 3,
            Self::ArrayFindStruct => 5,
            Self::ArrayRFindStruct => 5,
            Self::ArrayAdd => 3,
            Self::ArrayInsert => 3,
            Self::ArrayPop => 1,
            Self::ArrayRemove => 3,
            Self::ArrayClear => 1,
        }
    }

    fn has_variable_arg_count(&self) -> bool {
        match self {
            Self::CallMethod => true,
            Self::CallParent => true,
            Self::CallStatic => true,
            _ => false,
        }
    }
}

impl Decode for Opcode {
    fn decode<B>(reader: &mut Reader<B>) -> Result<Self, Error>
    where
        B: Buf,
    {
        match u8::decode(reader)? {
            0x00 => Ok(Self::Nop),
            0x01 => Ok(Self::IAdd),
            0x02 => Ok(Self::FAdd),
            0x03 => Ok(Self::ISub),
            0x04 => Ok(Self::FSub),
            0x05 => Ok(Self::IMul),
            0x06 => Ok(Self::FMul),
            0x07 => Ok(Self::IDiv),
            0x08 => Ok(Self::FDiv),
            0x09 => Ok(Self::IMod),
            0x0a => Ok(Self::Not),
            0x0b => Ok(Self::INeg),
            0x0c => Ok(Self::FNeg),
            0x0d => Ok(Self::Store),
            0x0e => Ok(Self::Cast),
            0x0f => Ok(Self::CmpEq),
            0x10 => Ok(Self::CmpLt),
            0x11 => Ok(Self::CmpLe),
            0x12 => Ok(Self::CmpGt),
            0x13 => Ok(Self::CmpGe),
            0x14 => Ok(Self::Branch),
            0x15 => Ok(Self::BranchIfTrue),
            0x16 => Ok(Self::BranchIfFalse),
            0x17 => Ok(Self::CallMethod),
            0x18 => Ok(Self::CallParent),
            0x19 => Ok(Self::CallStatic),
            0x1a => Ok(Self::Return),
            0x1b => Ok(Self::StringConcat),
            0x1c => Ok(Self::PropertyGet),
            0x1d => Ok(Self::PropertySet),
            0x1e => Ok(Self::ArrayCreate),
            0x1f => Ok(Self::ArrayLength),
            0x20 => Ok(Self::ArrayGet),
            0x21 => Ok(Self::ArraySet),
            0x22 => Ok(Self::ArrayFind),
            0x23 => Ok(Self::ArrayRFind),
            0x24 => Ok(Self::Is),
            0x25 => Ok(Self::StructCreate),
            0x26 => Ok(Self::StructGet),
            0x27 => Ok(Self::StructSet),
            0x28 => Ok(Self::ArrayFindStruct),
            0x29 => Ok(Self::ArrayRFindStruct),
            0x2a => Ok(Self::ArrayAdd),
            0x2b => Ok(Self::ArrayInsert),
            0x2c => Ok(Self::ArrayPop),
            0x2d => Ok(Self::ArrayRemove),
            0x2e => Ok(Self::ArrayClear),
            val => Err(Error::BadEnumValue("Opcode", val.into())),
        }
    }
}
