use bytes::Buf;

use crate::{Decode, Error, FormId, U16String};

#[derive(Clone, Debug)]
pub struct VirtualMachineAdapter<T = ()> {
    pub scripts: Vec<Script>,
    pub fragments: Option<T>,
}

impl<T> Decode for VirtualMachineAdapter<T>
where
    T: DecodeScriptSection<Error = Error>,
{
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let version = u16::decode(&mut buf)?;
        let object_format = match u16::decode(&mut buf)? {
            1 => ObjectFormat::V1,
            2 => ObjectFormat::V2,
            _ => todo!(),
        };

        let num_scripts = u16::decode(&mut buf)?;
        let scripts = (0..num_scripts)
            .map(|_| Script::decode_section(&mut buf, version, object_format))
            .collect::<Result<_, _>>()?;

        let fragments = if buf.has_remaining() {
            Some(T::decode_section(&mut buf, version, object_format)?)
        } else {
            None
        };

        Ok(Self { scripts, fragments })
    }
}

#[derive(Clone, Debug)]
pub struct Script {
    pub name: String,
    pub flags: u8,
    pub properties: Vec<Property>,
}

impl DecodeScriptSection for Script {
    type Error = Error;

    fn decode_section<B>(
        mut buf: B,
        version: u16,
        object_format: ObjectFormat,
    ) -> Result<Self, Error>
    where
        B: Buf,
    {
        let name = U16String::decode(&mut buf)?;
        let flags = u8::decode(&mut buf)?;

        let num_properties = u16::decode(&mut buf)?;
        let properties = (0..num_properties)
            .map(|_| Property::decode_section(&mut buf, version, object_format))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            name: name.0,
            flags,
            properties,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub name: String,
    pub value: Value,
}

impl DecodeScriptSection for Property {
    type Error = Error;

    fn decode_section<B>(
        mut buf: B,
        version: u16,
        object_format: ObjectFormat,
    ) -> Result<Self, Error>
    where
        B: Buf,
    {
        let mut stack = Vec::<Property>::new();
        let mut cmds = vec![ParseCommand::Value];

        while let Some(cmd) = cmds.pop() {
            // Reduce step
            while let Some(prop) = stack.pop() {
                let can_reduce = match &prop.value {
                    Value::Object(_) => true,
                    Value::String(_) => true,
                    Value::Int(_) => true,
                    Value::Float(_) => true,
                    Value::Bool(_) => true,
                    Value::Array(_, count) => *count == 0,
                    Value::Struct(_, count) => *count == 0,
                };

                if !can_reduce {
                    stack.push(prop);
                    break;
                }

                let Some(parent) = stack.last_mut() else {
                    stack.push(prop);
                    break;
                };

                match &mut parent.value {
                    Value::Array(elems, count) => {
                        elems.push(prop.value);
                        *count -= 1;
                    }
                    Value::Struct(members, count) => {
                        members.push(prop);
                        *count -= 1;
                    }
                    _ => {
                        stack.push(prop);
                        break;
                    }
                }
            }

            // Shift step
            match cmd {
                ParseCommand::Value => {
                    let name = U16String::decode(&mut buf)?;
                    let ty = u8::decode(&mut buf)?;
                    let flags = u8::decode(&mut buf)?;

                    let value = match ty {
                        // Object
                        1 => {
                            let value = Object::decode_section(&mut buf, version, object_format)?;
                            Value::Object(value)
                        }
                        // String
                        2 => {
                            let value = U16String::decode(&mut buf)?;
                            Value::String(value.0)
                        }
                        // Int
                        3 => {
                            let value = i32::decode(&mut buf)?;
                            Value::Int(value)
                        }
                        // Float
                        4 => {
                            let value = f32::decode(&mut buf)?;
                            Value::Float(value)
                        }
                        // Bool
                        5 => {
                            let value = u8::decode(&mut buf)?;
                            Value::Bool(value != 0)
                        }
                        // Struct
                        7 => {
                            let member_count = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Struct(member_count));

                            Value::Struct(Vec::new(), member_count)
                        }
                        // Array of Objects
                        11 => {
                            let len = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Array(ValueType::Object, len));

                            Value::Array(Vec::new(), len)
                        }
                        // Array of Strings
                        12 => {
                            let len = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Array(ValueType::String, len));

                            Value::Array(Vec::new(), len)
                        }
                        // Array of Ints
                        13 => {
                            let len = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Array(ValueType::Int, len));

                            Value::Array(Vec::new(), len)
                        }
                        // Array of Floats
                        14 => {
                            let len = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Array(ValueType::Float, len));

                            Value::Array(Vec::new(), len)
                        }
                        // Array of Bools
                        15 => {
                            let len = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Array(ValueType::Bool, len));

                            Value::Array(Vec::new(), len)
                        }
                        // Array of Structs
                        17 => {
                            let len = u32::decode(&mut buf)?;
                            cmds.push(ParseCommand::Array(ValueType::Struct, len));

                            Value::Array(Vec::new(), len)
                        }
                        _ => {
                            dbg!(&name, ty);
                            todo!();
                        }
                    };

                    stack.push(Self {
                        name: name.0,
                        value,
                    });
                }
                ParseCommand::Array(ty, len) => {
                    if len == 0 {
                        continue;
                    }

                    cmds.push(ParseCommand::Array(ty, len - 1));

                    let value = match ty {
                        ValueType::Object => {
                            let value = Object::decode_section(&mut buf, version, object_format)?;
                            Value::Object(value)
                        }
                        // String
                        ValueType::String => {
                            let value = U16String::decode(&mut buf)?;
                            Value::String(value.0)
                        }
                        // Int
                        ValueType::Int => {
                            let value = i32::decode(&mut buf)?;
                            Value::Int(value)
                        }
                        // Float
                        ValueType::Float => {
                            let value = f32::decode(&mut buf)?;
                            Value::Float(value)
                        }
                        // Bool
                        ValueType::Bool => {
                            let value = u8::decode(&mut buf)?;
                            Value::Bool(value != 0)
                        }
                        // Struct
                        ValueType::Struct => {
                            let member_count = u32::decode(&mut buf)?;
                            if member_count != 0 {
                                cmds.push(ParseCommand::Struct(member_count));
                            }

                            Value::Struct(Vec::new(), member_count)
                        }
                    };

                    stack.push(Self {
                        name: String::new(),
                        value,
                    });
                }
                ParseCommand::Struct(len) => {
                    if len == 0 {
                        continue;
                    }

                    cmds.push(ParseCommand::Struct(len - 1));
                    cmds.push(ParseCommand::Value);
                }
            }
        }

        debug_assert_eq!(stack.len(), 1);
        let value = stack.remove(0);

        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Object(Object),
    String(String),
    Int(i32),
    Float(f32),
    Bool(bool),
    Array(Vec<Value>, u32),
    Struct(Vec<Property>, u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub form: FormId,
    pub alias: Option<u16>,
}

impl DecodeScriptSection for Object {
    type Error = Error;

    fn decode_section<B>(
        mut buf: B,
        _version: u16,
        object_format: ObjectFormat,
    ) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let (form, alias) = match object_format {
            ObjectFormat::V1 => {
                let form = FormId::decode(&mut buf)?;
                let alias = u16::decode(&mut buf)?;
                u16::decode(&mut buf)?;
                (form, alias)
            }
            ObjectFormat::V2 => {
                u16::decode(&mut buf)?;
                let alias = u16::decode(&mut buf)?;
                let form = FormId::decode(&mut buf)?;
                (form, alias)
            }
        };

        let alias = (alias != u16::MAX).then_some(alias);
        Ok(Self { form, alias })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ObjectFormat {
    V1,
    V2,
}

#[derive(Copy, Clone, Debug)]
enum ParseCommand {
    Value,
    Array(ValueType, u32),
    Struct(u32),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ValueType {
    Object,
    String,
    Int,
    Float,
    Bool,
    Struct,
}

trait DecodeScriptSection: Sized {
    type Error;

    fn decode_section<B>(
        buf: B,
        version: u16,
        object_format: ObjectFormat,
    ) -> Result<Self, Self::Error>
    where
        B: Buf;
}

impl<T> DecodeScriptSection for T
where
    T: Decode,
{
    type Error = T::Error;

    fn decode_section<B>(
        buf: B,
        _version: u16,
        _object_format: ObjectFormat,
    ) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        T::decode(buf)
    }
}

#[derive(Clone, Debug)]
pub struct QuestFragments {
    pub script: Script,
    pub fragments: Vec<QuestFragment>,
    pub aliases: Vec<QuestAlias>,
}

impl DecodeScriptSection for QuestFragments {
    type Error = Error;

    fn decode_section<B>(
        mut buf: B,
        version: u16,
        object_format: ObjectFormat,
    ) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        u8::decode(&mut buf)?; // Unknwon?
        let fragment_count = u16::decode(&mut buf)?;

        let script = Script::decode_section(&mut buf, version, object_format)?;

        let fragments = (0..fragment_count)
            .map(|_| QuestFragment::decode(&mut buf))
            .collect::<Result<_, _>>()?;

        let alias_count = u16::decode(&mut buf)?;
        let aliases = (0..alias_count)
            .map(|_| QuestAlias::decode_section(&mut buf, version, object_format))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            script,
            fragments,
            aliases,
        })
    }
}

#[derive(Clone, Debug)]
pub struct QuestFragment {
    pub quest_stage: i16,
    pub quest_stage_index: u32,
    pub script_name: String,
    pub fragment_name: String,
}

impl Decode for QuestFragment {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let quest_stage = i16::decode(&mut buf)?;
        u16::decode(&mut buf)?; // Unknown?
        let quest_stage_index = u32::decode(&mut buf)?;
        u8::decode(&mut buf)?; // Unknown?
        let script_name = U16String::decode(&mut buf)?;
        let fragment_name = U16String::decode(&mut buf)?;

        dbg!(&script_name, &fragment_name);

        Ok(Self {
            quest_stage,
            quest_stage_index,
            script_name: script_name.0,
            fragment_name: fragment_name.0,
        })
    }
}

#[derive(Clone, Debug)]
pub struct QuestAlias {
    pub object: Object,
    pub scripts: Vec<Script>,
}

impl DecodeScriptSection for QuestAlias {
    type Error = Error;

    fn decode_section<B>(
        mut buf: B,
        version: u16,
        object_format: ObjectFormat,
    ) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let object = Object::decode_section(&mut buf, version, object_format)?;

        // VMAD version and object format again?
        u16::decode(&mut buf)?;
        u16::decode(&mut buf)?;

        let script_count = u16::decode(&mut buf)?;
        let scripts = (0..script_count)
            .map(|_| Script::decode_section(&mut buf, version, object_format))
            .collect::<Result<_, _>>()?;

        Ok(Self { object, scripts })
    }
}

#[cfg(test)]
mod tests {
    use super::{DecodeScriptSection, ObjectFormat, Property, Value};

    #[test]
    fn decode_property_struct_recursive() {
        let input = [
            1, 0, b'A', // Property name
            7, 0, // Type and Flags (Struct)
            1, 0, 0, 0, // Struct member count
            1, 0, b'B', // Property name
            7, 0, // Type and Flags (Struct)
            1, 0, 0, 0, // Struct member count
            1, 0, b'C', // Property name
            3, 0, // Type and Flags (Int)
            123, 0, 0, 0, // Int value
        ];

        let property = Property::decode_section(&input[..], 0, ObjectFormat::V1).unwrap();
        assert_eq!(property.name, "A");
        let members = match &property.value {
            Value::Struct(members, _) => members,
            _ => unreachable!(),
        };

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "B");
        let members = match &members[0].value {
            Value::Struct(members, _) => members,
            _ => unreachable!(),
        };

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "C");
        matches!(members[0].value, Value::Int(123));
    }

    #[test]
    fn decode_property_array_of_structs() {
        let input = [
            1, 0, b'A', // Property name
            17, 0, // Type and Flags (Array of Structs)
            2, 0, 0, 0, // Array len
            1, 0, 0, 0, // Struct member count
            1, 0, b'B', // Property name
            3, 0, // Type and Flags (Int)
            42, 0, 0, 0, // Value (Int)
            1, 0, 0, 0, // Struct member count
            1, 0, b'C', // Property name
            3, 0, // Type and Flags (Int)
            43, 0, 0, 0, // Value (Int)
        ];

        let property = Property::decode_section(&input[..], 0, ObjectFormat::V1).unwrap();
        assert_eq!(
            property,
            Property {
                name: "A".to_owned(),
                value: Value::Array(
                    vec![
                        Value::Struct(
                            vec![Property {
                                name: "B".to_owned(),
                                value: Value::Int(42),
                            }],
                            0
                        ),
                        Value::Struct(
                            vec![Property {
                                name: "C".to_owned(),
                                value: Value::Int(43)
                            }],
                            0
                        ),
                    ],
                    0
                )
            }
        );
    }
}
