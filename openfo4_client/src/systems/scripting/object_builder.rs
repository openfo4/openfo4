use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;
use openfo4_esp::vmad::{Property, Script, Value};
use openfo4_script::compiler::resources::{PropertyType, TypeId, TypeInfo};
use openfo4_script::engine::Engine;
use openfo4_script::runtime::object::{ObjectArrayBuilder, ObjectBuilder};
use openfo4_script::runtime::{ObjectAddr, ObjectRef, Val};
use thiserror::Error;

use crate::systems::scripting::loader::AliasKey;
use crate::systems::scripting::objects::ScriptObjects;

#[derive(Clone, Debug, Error)]
pub enum BuildScriptObjectWarning {
    #[error("unknown alias: {0:?}")]
    UnknownAlias(AliasKey),
    #[error("unknown object: {0:?}")]
    UnknownObject(RecordId),
    #[error("unknown field or property {0}.{1}")]
    UnknownField(String, String),
    #[error("no applicable script found in {0:?}")]
    NoApplicableScriptFound(RecordId),
    #[error("invalid value type {found} for papyrus type {expected}")]
    InvalidType {
        found: &'static str,
        expected: String,
    },
}

struct Entry<'a> {
    /// The [`Location`] in the parent object where this object should be placed when finalized.
    parent: Option<Location>,
    builder: Builder<'a>,
    members: Members<'a>,
}

#[derive(Clone, Debug)]
enum Location {
    Property {
        class: Lowercase<String>,
        prop: Lowercase<String>,
    },
    ArrayIndex(usize),
}

enum Builder<'a> {
    ObjectRef(&'a mut ObjectBuilder),
    Object(ObjectBuilder),
    Array(ObjectArrayBuilder),
}

impl<'a> Builder<'a> {
    fn set(&mut self, location: Location, value: Val) {
        match (location, self) {
            (Location::Property { class, prop }, Self::ObjectRef(builder)) => {
                builder.set(class.as_inner(), prop.as_inner(), value);
            }
            (Location::Property { class, prop }, Self::Object(builder)) => {
                builder.set(class.as_inner(), prop.as_inner(), value);
            }
            (Location::ArrayIndex(index), Self::Array(builder)) => {
                builder.set_at(index, "", "", value);
            }
            _ => unreachable!(),
        }
    }

    fn finalize(self) -> ObjectRef {
        match self {
            Self::ObjectRef(builder) => unsafe { builder.finalize_copy() },
            Self::Object(builder) => builder.finalize(),
            Self::Array(builder) => builder.finalize(),
        }
    }
}

enum Members<'a> {
    Struct {
        properties: &'a [Property],
        class: Lowercase<String>,
        info: TypeInfo,
    },
    Array {
        values: &'a [Value],
        next_index: usize,
        info: PropertyType,
    },
}

impl Members<'_> {
    fn is_exhausted(&self) -> bool {
        match self {
            Self::Struct {
                properties,
                class: _,
                info: _,
            } => properties.is_empty(),
            Self::Array {
                values,
                next_index: _,
                info: _,
            } => values.is_empty(),
        }
    }
}

struct ObjectBuilderInner<'a, F>
where
    F: Fn(FormId) -> RecordId,
{
    engine: &'a Engine,
    resolve_form: F,
    scripts: &'a ScriptObjects,
    addrs: &'a HashMap<RecordId, HashMap<Lowercase<String>, ObjectAddr>>,
    aliases: &'a HashMap<AliasKey, HashMap<Lowercase<String>, ObjectAddr>>,
    script_defined_in: RecordId,
    stack: Vec<Entry<'a>>,
    warnings: Vec<BuildScriptObjectWarning>,
    types: PrimitiveTypes,
}

#[derive(Copy, Clone, Debug)]
struct PrimitiveTypes {
    bool: TypeId,
    int: TypeId,
    float: TypeId,
    string: TypeId,
}

impl<'a, F> ObjectBuilderInner<'a, F>
where
    F: Fn(FormId) -> RecordId,
{
    fn step(&mut self) {
        let Some(entry) = self.stack.last_mut() else {
            return;
        };

        if entry.members.is_exhausted() {
            let entry = self.stack.pop().unwrap();
            let Some(location) = entry.parent else {
                return;
            };

            let object = entry.builder.finalize();
            let parent = self.stack.last_mut().unwrap();

            parent.builder.set(location, Val::Object(object.addr()));
            return;
        }

        let (value, location, info) = match &mut entry.members {
            Members::Struct {
                properties,
                class,
                info,
            } => {
                let Some(prop) = properties.split_off_first() else {
                    return;
                };

                let prop_name = Lowercase::new(&*prop.name);

                // If we are processing a class it is possible to access properties of
                // superclasses.
                // If a field exists on both a superclass and a subclass we must prefer
                // the subclass.
                let info = info.classes.iter().rev().find_map(|class| {
                    let class = self.engine.type_info(*class);
                    let class = class.name.unwrap();
                    info.properties.get(&(class, prop_name.clone()))
                });

                // Setting an unknown field is generally not a good idea,
                // but it is done in some official ESP files and we must also support this.
                let Some(info) = info else {
                    self.warnings.push(BuildScriptObjectWarning::UnknownField(
                        class.to_string(),
                        prop_name.to_string(),
                    ));
                    return;
                };

                let location = Location::Property {
                    class: class.clone(),
                    prop: prop_name,
                };
                (&prop.value, location, info.ty)
            }
            Members::Array {
                values,
                next_index: len,
                info,
            } => {
                let Some(value) = values.split_off_first() else {
                    return;
                };

                let location = Location::ArrayIndex(*len);
                *len += 1;
                (value, location, *info)
            }
        };

        let value = match (info, value) {
            (PropertyType::Inline(ty), Value::Bool(val)) if ty == self.types.bool => {
                Val::Bool(*val)
            }
            (PropertyType::Inline(ty), Value::Int(val)) if ty == self.types.int => Val::I32(*val),
            (PropertyType::Inline(ty), Value::Float(val)) if ty == self.types.float => {
                Val::F32(*val)
            }
            (PropertyType::Inline(ty), Value::String(val)) if ty == self.types.string => {
                let object = self.engine.object_pool().insert_string(val);
                Val::Object(object.addr())
            }
            (PropertyType::Pointer(ty), Value::Object(object)) => {
                let info = self.engine.type_info(ty);
                let class = info.name.as_ref().unwrap();
                // TODO: Assert that info is a class, not a struct.

                // Object references are already null by default.
                // We don't need to set it explicitly.
                if object.form.is_null() {
                    return;
                }

                let id = (self.resolve_form)(object.form);

                // Note that in addition to the exact class, any subclass of the requested
                // class is also usable.
                // We do not need to search for classes other than the exact one because
                // the caller is expected to provide all superclasses in addition to the
                // exact class provided, which includes the exact class we are searching
                // for.
                let addr = match object.alias {
                    Some(alias) => {
                        let key = AliasKey {
                            accessor: self.script_defined_in,
                            owner: id,
                            id: alias.into(),
                        };

                        let Some(scripts) = self.aliases.get(&key) else {
                            self.warnings
                                .push(BuildScriptObjectWarning::UnknownAlias(key));
                            return;
                        };

                        scripts.get(class).copied()
                    }
                    None => {
                        if let Some(scripts) = self.scripts.get_by_record(id) {
                            scripts
                                .get(class.as_inner())
                                .map(|obj| obj.get().object.addr())
                        } else if let Some(scripts) = self.addrs.get(&id) {
                            scripts.get(class).copied()
                        } else {
                            self.warnings
                                .push(BuildScriptObjectWarning::UnknownObject(id));
                            return;
                        }
                    }
                };

                let Some(addr) = addr else {
                    self.warnings
                        .push(BuildScriptObjectWarning::NoApplicableScriptFound(id));
                    return;
                };

                Val::Object(addr)
            }
            (PropertyType::Pointer(ty), Value::Struct(fields, _)) => {
                let info = self.engine.type_info(ty);
                let name = info.name.clone().unwrap();
                // TODO: Assert that info is indeed a struct.

                let builder = self.engine.object_pool().insert(ty);
                self.stack.push(Entry {
                    parent: Some(location),
                    builder: Builder::Object(builder),
                    members: Members::Struct {
                        properties: &fields,
                        class: name,
                        info,
                    },
                });

                return;
            }
            (PropertyType::ArrayPointer(ty), Value::Array(fields, _)) => {
                let info = self.engine.type_info(ty);

                // FIXME: This check for primitive is a "hack" around the fact
                // that arrays are currently handled weirdly in the papyrus compiler:
                // All primitive types are inlined directly in the array storage,
                // while all other types are stored by pointer,
                // but the compiler does currently not expose whether we expect
                // an inline value or a pointer.
                // In the future the compiler should expose whether we expect a pointer
                // or an inline value here.
                // This would also allow us to support nested arrays.
                let (info, ty) = if is_primitive(ty, &info) {
                    (PropertyType::Inline(ty), ty)
                } else {
                    (PropertyType::Pointer(ty), self.engine.ptr_type())
                };
                let builder = self.engine.object_pool().insert_array(ty, fields.len());

                self.stack.push(Entry {
                    parent: Some(location),
                    builder: Builder::Array(builder),
                    members: Members::Array {
                        values: fields,
                        next_index: 0,
                        info,
                    },
                });

                return;
            }
            (ty, value) => {
                let ty = match ty {
                    PropertyType::Inline(ty) => ty,
                    PropertyType::Pointer(ty) => ty,
                    PropertyType::ArrayPointer(ty) => ty,
                };
                let info = self.engine.type_info(ty);
                let expected = info
                    .name
                    .map(|name| name.into_inner())
                    .unwrap_or_else(|| "<unknown>".to_owned());

                let found = match value {
                    Value::Bool(_) => "bool",
                    Value::Int(_) => "int",
                    Value::Float(_) => "float",
                    Value::String(_) => "string",
                    Value::Object(_) => "object",
                    Value::Struct(_, _) => "struct object",
                    Value::Array(_, _) => "array object",
                };

                self.warnings
                    .push(BuildScriptObjectWarning::InvalidType { found, expected });
                return;
            }
        };

        entry.builder.set(location, value);
    }
}

pub fn build_script_object(
    resolve_form: impl Fn(FormId) -> RecordId,
    engine: &Engine,
    script: &Script,
    scripts: &ScriptObjects,
    partial_addrs: &HashMap<RecordId, HashMap<Lowercase<String>, ObjectAddr>>,
    aliases: &HashMap<AliasKey, HashMap<Lowercase<String>, ObjectAddr>>,
    builder: &mut ObjectBuilder,
    script_defined_in: RecordId,
) -> (ObjectRef, Vec<BuildScriptObjectWarning>) {
    let ty = engine.get_type(&script.name).unwrap();
    let info = engine.type_info(ty);

    let entry = Entry {
        parent: None,
        builder: Builder::ObjectRef(builder),
        members: Members::Struct {
            properties: &script.properties,
            class: Lowercase::new(&*script.name),
            info,
        },
    };

    let mut inner = ObjectBuilderInner {
        resolve_form,
        engine,
        scripts,
        addrs: partial_addrs,
        aliases,
        script_defined_in,
        stack: vec![entry],
        warnings: Vec::new(),
        types: PrimitiveTypes {
            bool: engine.bool_type(),
            int: engine.int_type(),
            float: engine.float_type(),
            string: engine.get_type("string").unwrap(),
        },
    };

    while !inner.stack.is_empty() {
        inner.step();
    }

    let warnings = core::mem::take(&mut inner.warnings);
    drop(inner);

    (unsafe { builder.finalize_copy() }, warnings)
}

fn is_primitive(ty: TypeId, info: &TypeInfo) -> bool {
    // A primitive type is defined as a type with a single property
    // with class name "" and property name "" and inline storage
    // that points back to the primitive type.
    if info.properties.len() != 1 {
        return false;
    }

    let ((class, prop), info) = info.properties.iter().next().unwrap();
    if class.as_inner() != "" || prop.as_inner() != "" {
        return false;
    }

    info.ty == PropertyType::Inline(ty)
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;
    use std::num::NonZeroU32;

    use futures_lite::future::block_on;
    use game_common::string::Lowercase;
    use hashbrown::HashMap;
    use openfo4_core::record::{ModuleId, RecordId};
    use openfo4_esp::FormId;
    use openfo4_esp::vmad::{Object, Property, Script, Value};
    use openfo4_script::assembler::assemble;
    use openfo4_script::compiler::CompilerOptions;
    use openfo4_script::engine::{Engine, Options};
    use openfo4_script::runtime::{ArrayObject, ObjectAddr, ObjectRef};

    use crate::systems::scripting::objects::ScriptObjects;

    use super::build_script_object;

    const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
        validation: true,
        debug: true,
    };

    fn build_direct(engine: &Engine, script: &Script) -> ObjectRef {
        let type_id = engine.get_type(&script.name).unwrap();
        let mut builder = engine.object_pool().insert(type_id);

        let scripts = ScriptObjects::default();
        let partial_addrs = HashMap::new();
        let aliases = HashMap::new();

        let (res, warnings) = build_script_object(
            |_| unreachable!(),
            &engine,
            &script,
            &scripts,
            &partial_addrs,
            &aliases,
            &mut builder,
            RecordId::DANGLING,
        );

        assert!(warnings.is_empty());
        res
    }

    fn assert_array_contents_eq(
        engine: &Engine,
        object: &ObjectRef,
        class: &str,
        name: &str,
        len: usize,
        data: &[u8],
    ) {
        let object = engine.object_pool().get(&object).unwrap();

        unsafe {
            let ptr = object.get(class, name).unwrap();
            let ptr = ObjectAddr::from_raw(ptr.as_ptr().cast::<*mut u8>().read());

            let array = engine.object_pool().get(ptr).unwrap();
            let array = &*array.as_ptr().cast::<ArrayObject>();

            assert_eq!(array.len, len);

            let mut mem = Vec::new();

            let mut next_ptr = array.ptr;
            for _ in 0..array.len {
                let addr = ObjectAddr::from_raw(next_ptr);
                let elem = engine.object_pool().get(addr).unwrap();
                mem.extend_from_slice(elem.memory());

                next_ptr = next_ptr.add(elem.size());
            }

            assert_eq!(mem, data);
        }
    }

    #[test]
    fn build_script_object_basic() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class test:
            global a: int = 0
            global b: int = 0
            property a: int -> a
            property b: int -> b
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("test", &Options::default())).unwrap();

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Int(1),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Int(2),
                },
            ],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(&object).unwrap();

        unsafe {
            assert_eq!(object.get("test", "a").unwrap(), 1u32.to_ne_bytes());
            assert_eq!(object.get("test", "b").unwrap(), 2u32.to_ne_bytes());
        }
    }

    #[test]
    fn build_script_object_array() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class test:
            global a: int[] = null
            global b: float[] = null
            global c: bool[] = null
            property a: int[] -> a
            property b: float[] -> b
            property c: bool[] -> c
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("test", &Options::default())).unwrap();

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Array(vec![Value::Int(0), Value::Int(1), Value::Int(2)], 0),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Array(
                        vec![
                            Value::Float(0.0),
                            Value::Float(1.0),
                            Value::Float(2.0),
                            Value::Float(3.0),
                        ],
                        0,
                    ),
                },
                Property {
                    name: "c".to_owned(),
                    value: Value::Array(Vec::new(), 0),
                },
            ],
        };

        let object = build_direct(&engine, &script);

        assert_array_contents_eq(
            &engine,
            &object,
            "test",
            "a",
            3,
            bytemuck::cast_slice(&[0_i32, 1_i32, 2_i32]),
        );
        assert_array_contents_eq(
            &engine,
            &object,
            "test",
            "b",
            4,
            bytemuck::cast_slice(&[0.0_f32, 1.0_f32, 2.0_f32, 3.0_f32]),
        );
        assert_array_contents_eq(&engine, &object, "test", "c", 0, &[]);
    }

    #[test]
    fn build_script_object_struct() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class test:

            struct MyStruct:
                field x: int
                field y: int

            global a: int = 0
            global b: test#MyStruct = null
            property a: int -> a
            property b: test#MyStruct -> b
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("test", &Options::default())).unwrap();

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Int(1),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Struct(
                        vec![
                            Property {
                                name: "x".to_owned(),
                                value: Value::Int(2),
                            },
                            Property {
                                name: "y".to_owned(),
                                value: Value::Int(3),
                            },
                        ],
                        0,
                    ),
                },
            ],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(&object).unwrap();

        unsafe {
            assert_eq!(object.get("test", "a").unwrap(), 1u32.to_ne_bytes());
            let struct_addr = object
                .get("test", "b")
                .unwrap()
                .as_ptr()
                .cast::<ObjectAddr>()
                .read();

            let st = engine.object_pool().get(struct_addr).unwrap();
            assert_eq!(st.get("test#MyStruct", "x").unwrap(), 2u32.to_ne_bytes());
            assert_eq!(st.get("test#MyStruct", "y").unwrap(), 3u32.to_ne_bytes());
        }
    }

    #[test]
    fn build_script_object_array_of_structs() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class test:

            struct MyStruct:
                field x: int
                field y: int

            global a: int = 0
            global b: test#MyStruct[] = null
            property a: int -> a
            property b: test#MyStruct[] -> b
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("test", &Options::default())).unwrap();

        let script = Script {
            name: "test".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Int(1),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Array(
                        vec![
                            Value::Struct(
                                vec![
                                    Property {
                                        name: "x".to_owned(),
                                        value: Value::Int(2),
                                    },
                                    Property {
                                        name: "y".to_owned(),
                                        value: Value::Int(3),
                                    },
                                ],
                                0,
                            ),
                            Value::Struct(
                                vec![
                                    Property {
                                        name: "x".to_owned(),
                                        value: Value::Int(4),
                                    },
                                    Property {
                                        name: "y".to_owned(),
                                        value: Value::Int(5),
                                    },
                                ],
                                0,
                            ),
                        ],
                        0,
                    ),
                },
            ],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(&object).unwrap();

        unsafe {
            assert_eq!(object.get_typed::<i32>("test", "a").unwrap(), 1);
            let array_ptr = object
                .get("test", "b")
                .unwrap()
                .as_ptr()
                .cast::<ObjectAddr>()
                .read();
            let data_ptr = array_ptr
                .to_raw()
                .add(offset_of!(ArrayObject, ptr))
                .cast::<ObjectAddr>()
                .read();

            let object0 = data_ptr.to_raw().cast::<ObjectAddr>().read();
            let object1 = data_ptr.to_raw().cast::<ObjectAddr>().add(1).read();

            let object0 = engine.object_pool().get(object0).unwrap();
            assert_eq!(object0.get_typed::<i32>("test#MyStruct", "x",).unwrap(), 2);
            assert_eq!(object0.get_typed::<i32>("test#MyStruct", "y").unwrap(), 3);

            let object1 = engine.object_pool().get(object1).unwrap();
            assert_eq!(object1.get_typed::<i32>("test#MyStruct", "x",).unwrap(), 4);
            assert_eq!(object1.get_typed::<i32>("test#MyStruct", "y").unwrap(), 5);
        }
    }

    #[test]
    fn build_script_object_parent_class_property() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class A:

            global a: int = 0
            property a: int -> a

            class B:
            parent: A
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("B", &Options::default())).unwrap();

        let script = Script {
            name: "B".to_owned(),
            flags: 0,
            properties: vec![Property {
                name: "a".to_owned(),
                value: Value::Int(1),
            }],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(object).unwrap();

        unsafe {
            assert_eq!(object.get_typed::<i32>("A", "a").unwrap(), 1);
        }
    }

    #[test]
    fn build_script_object_prefer_subclass_property() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class A:

            global a: int = 0
            property a: int -> a

            class B:
            parent: A

            global a: int = 0
            property a: int -> a
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("B", &Options::default())).unwrap();

        let script = Script {
            name: "B".to_owned(),
            flags: 0,
            properties: vec![Property {
                name: "a".to_owned(),
                value: Value::Int(1),
            }],
        };

        let object = build_direct(&engine, &script);
        let object = engine.object_pool().get(object).unwrap();

        unsafe {
            assert_eq!(object.get_typed::<i32>("A", "a").unwrap(), 0);
            assert_eq!(object.get_typed::<i32>("B", "a").unwrap(), 1);
        }
    }

    #[test]
    fn build_script_object_reflect_script_name() {
        let mut engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class A:

            class B:

            class C:
            global a: A = null
            global b: B = null
            global c: C = null
            property a: A -> a
            property b: B -> b
            property c: C -> c
            "#,
        );

        engine.register(&module);
        block_on(engine.compile("A", &Options::default())).unwrap();
        block_on(engine.compile("B", &Options::default())).unwrap();
        block_on(engine.compile("C", &Options::default())).unwrap();

        let script = Script {
            name: "C".to_owned(),
            flags: 0,
            properties: vec![
                Property {
                    name: "a".to_owned(),
                    value: Value::Object(Object {
                        form: FormId(0x01),
                        alias: None,
                    }),
                },
                Property {
                    name: "b".to_owned(),
                    value: Value::Object(Object {
                        form: FormId(0x01),
                        alias: None,
                    }),
                },
                Property {
                    name: "c".to_owned(),
                    value: Value::Object(Object {
                        form: FormId(0x01),
                        alias: None,
                    }),
                },
            ],
        };

        let type_a = engine.get_type("A").unwrap();
        let type_b = engine.get_type("B").unwrap();
        let type_c = engine.get_type("C").unwrap();

        let a = engine.object_pool().insert(type_a).finalize().addr();
        let b = engine.object_pool().insert(type_b).finalize().addr();
        let c = engine.object_pool().insert(type_c).finalize().addr();

        let scripts = ScriptObjects::default();
        let partial_addrs = [(
            RecordId::new(ModuleId(NonZeroU32::MIN), FormId(0x01)),
            [
                (Lowercase::new("A"), a),
                (Lowercase::new("B"), b),
                (Lowercase::new("C"), c),
            ]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();

        let aliases = HashMap::new();

        let mut builder = engine.object_pool().insert(type_c);

        let (_, warnings) = build_script_object(
            |form| RecordId::new(ModuleId(NonZeroU32::MIN), form),
            &engine,
            &script,
            &scripts,
            &partial_addrs,
            &aliases,
            &mut builder,
            RecordId::DANGLING,
        );
        assert!(warnings.is_empty());

        let object = engine.object_pool().get(builder.finalize()).unwrap();
        unsafe {
            assert_eq!(
                object.get("C", "a").unwrap(),
                a.to_raw().addr().to_ne_bytes()
            );

            assert_eq!(
                object.get("C", "b").unwrap(),
                b.to_raw().addr().to_ne_bytes()
            );

            assert_eq!(
                object.get("C", "c").unwrap(),
                c.to_raw().addr().to_ne_bytes()
            );
        }
    }
}
