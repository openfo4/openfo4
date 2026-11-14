use hashbrown::HashMap;
use openfo4_common::string::Lowercase;
use openfo4_core::record::RecordId;
use openfo4_esp::FormId;
use openfo4_esp::vmad::{Property, Script, Value};
use openfo4_script::compiler::resources::TypeId;
use openfo4_script::engine::{Engine, TypeRef};
use openfo4_script::runtime::object::{ObjectArrayBuilder, ObjectHandle};
use openfo4_script::runtime::{ObjectAddr, Val};
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
        class: TypeId,
        prop: Lowercase<String>,
    },
    ArrayIndex(usize),
}

enum Builder<'a> {
    ObjectRef(&'a ObjectHandle),
    Object(ObjectHandle),
    Array(ObjectArrayBuilder),
}

impl<'a> Builder<'a> {
    fn set(&mut self, location: Location, value: Val) {
        match (location, self) {
            (Location::Property { class, prop }, Self::ObjectRef(builder)) => {
                builder.set(class, prop.as_inner(), value).unwrap();
            }
            (Location::Property { class, prop }, Self::Object(builder)) => {
                builder.set(class, prop.as_inner(), value).unwrap();
            }
            (Location::ArrayIndex(index), Self::Array(builder)) => {
                builder.set_at(index, value);
            }
            _ => unreachable!(),
        }
    }

    fn finalize(self) -> ObjectHandle {
        match self {
            Self::ObjectRef(handle) => handle.clone(),
            Self::Object(handle) => handle,
            Self::Array(builder) => builder.finalize(),
        }
    }
}

enum Members<'a> {
    Struct {
        properties: &'a [Property],
        class: TypeId,
        ty: TypeRef<'a>,
    },
    Array {
        values: &'a [Value],
        next_index: usize,
        elem_ty: TypeRef<'a>,
    },
}

impl Members<'_> {
    fn is_exhausted(&self) -> bool {
        match self {
            Self::Struct {
                properties,
                class: _,
                ty: _,
            } => properties.is_empty(),
            Self::Array {
                values,
                next_index: _,
                elem_ty: _,
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
    addrs: &'a HashMap<RecordId, HashMap<TypeId, ObjectAddr>>,
    aliases: &'a HashMap<AliasKey, HashMap<TypeId, ObjectAddr>>,
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

        let (value, location, ty) = match &mut entry.members {
            Members::Struct {
                properties,
                class,
                ty,
            } => {
                let Some(prop) = properties.split_off_first() else {
                    return;
                };

                let prop_name = Lowercase::new(&*prop.name);

                // If we are processing a class it is possible to access properties of
                // superclasses.
                // If a field exists on both a superclass and a subclass we must prefer
                // the subclass.
                let field = ty.info().find_property(&prop_name);

                // Setting an unknown field is generally not a good idea,
                // but it is done in some official ESP files and we must also support this.
                let Some(info) = field else {
                    self.warnings.push(BuildScriptObjectWarning::UnknownField(
                        ty.info().name().unwrap_or_default().to_owned(),
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
                elem_ty,
            } => {
                let Some(value) = values.split_off_first() else {
                    return;
                };

                let location = Location::ArrayIndex(*len);
                *len += 1;
                (value, location, elem_ty.id())
            }
        };

        let ty = self.engine.get_type_by_id(ty).unwrap();

        let value = match value {
            Value::Bool(val) if ty.id() == self.types.bool => Val::Bool(*val),
            Value::Int(val) if ty.id() == self.types.int => Val::I32(*val),
            Value::Float(val) if ty.id() == self.types.float => Val::F32(*val),
            Value::String(val) if ty.id() == self.types.string => {
                let object = self.engine.object_pool().insert_string(val);
                Val::Object(object.addr())
            }
            Value::Object(object) => {
                let class = ty.id();
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

                        scripts.get(&class).copied()
                    }
                    None => {
                        if let Some(scripts) = self.scripts.get_by_record(id) {
                            scripts
                                .get_by_type(class)
                                .map(|obj| obj.get().object.addr())
                        } else if let Some(scripts) = self.addrs.get(&id) {
                            scripts.get(&class).copied()
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
            Value::Struct(fields, _) => {
                // TODO: Assert that info is indeed a struct.

                let builder = self.engine.object_pool().insert(ty.id());
                self.stack.push(Entry {
                    parent: Some(location),
                    builder: Builder::Object(builder),
                    members: Members::Struct {
                        properties: &fields,
                        class: ty.id(),
                        ty,
                    },
                });

                return;
            }
            Value::Array(fields, _) if let Some(elem_ty) = ty.elem_ty() => {
                let builder = self
                    .engine
                    .object_pool()
                    .insert_array(elem_ty.id(), fields.len());

                self.stack.push(Entry {
                    parent: Some(location),
                    builder: Builder::Array(builder),
                    members: Members::Array {
                        values: fields,
                        next_index: 0,
                        elem_ty,
                    },
                });

                return;
            }
            value => {
                let expected = ty.info().name().unwrap_or_else(|| "<unknown>").to_owned();

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
    partial_addrs: &HashMap<RecordId, HashMap<TypeId, ObjectAddr>>,
    aliases: &HashMap<AliasKey, HashMap<TypeId, ObjectAddr>>,
    builder: &ObjectHandle,
    script_defined_in: RecordId,
) -> (ObjectHandle, Vec<BuildScriptObjectWarning>) {
    let ty = engine.get_type(&script.name).unwrap();

    let entry = Entry {
        parent: None,
        builder: Builder::ObjectRef(builder),
        members: Members::Struct {
            properties: &script.properties,
            class: ty.id(),
            ty,
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
            string: engine.get_type("string").unwrap().id(),
        },
    };

    while !inner.stack.is_empty() {
        inner.step();
    }

    let warnings = core::mem::take(&mut inner.warnings);
    drop(inner);

    (builder.clone(), warnings)
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;
    use std::num::NonZeroU32;

    use futures_lite::future::block_on;
    use hashbrown::HashMap;
    use openfo4_core::record::{ModuleId, RecordId};
    use openfo4_esp::FormId;
    use openfo4_esp::vmad::{Object, Property, Script, Value};
    use openfo4_script::assembler::assemble;
    use openfo4_script::compiler::CompilerOptions;
    use openfo4_script::compiler::resources::TypeId;
    use openfo4_script::engine::{Engine, Options};
    use openfo4_script::runtime::object::ObjectHandle;
    use openfo4_script::runtime::{ArrayObject, ObjectAddr};

    use crate::systems::scripting::objects::ScriptObjects;

    use super::build_script_object;

    const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
        validation: true,
        debug: true,
    };

    fn build_direct(engine: &Engine, script: &Script) -> ObjectHandle {
        let type_id = engine.get_type(&script.name).unwrap();
        let mut builder = engine.object_pool().insert(type_id.id());

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
        object: &ObjectHandle,
        class: TypeId,
        name: &str,
        len: usize,
        data: &[u8],
    ) {
        unsafe {
            let array_ptr = object.get::<ObjectAddr>(class, name).unwrap();
            let array = engine.object_pool().get(array_ptr).unwrap();
            let array = &*array.as_ptr().cast::<ArrayObject>();

            assert_eq!(array.len, len);

            let mut mem = Vec::new();

            let array_obj = engine
                .object_pool()
                .get(ObjectAddr::from_raw(array.ptr))
                .unwrap();

            for index in 0..array.len {
                let elem = array_obj.at(index).unwrap();
                mem.extend_from_slice(elem.memory());
            }

            assert_eq!(mem, data);
        }
    }

    #[test]
    fn build_script_object_basic() {
        let engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class test:
            global a: int = 0
            global b: int = 0
            property a: int -> a
            property b: int -> b
            "#,
        );

        block_on(engine.register(&module));
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

        assert_eq!(object.get_self::<i32>("a").unwrap(), 1);
        assert_eq!(object.get_self::<i32>("b").unwrap(), 2);
    }

    #[test]
    fn build_script_object_array() {
        let engine = Engine::new(COMPILER_OPTIONS);

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

        block_on(engine.register(&module));
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
            object.type_id(),
            "a",
            3,
            bytemuck::cast_slice(&[0_i32, 1_i32, 2_i32]),
        );
        assert_array_contents_eq(
            &engine,
            &object,
            object.type_id(),
            "b",
            4,
            bytemuck::cast_slice(&[0.0_f32, 1.0_f32, 2.0_f32, 3.0_f32]),
        );
        assert_array_contents_eq(&engine, &object, object.type_id(), "c", 0, &[]);
    }

    #[test]
    fn build_script_object_struct() {
        let engine = Engine::new(COMPILER_OPTIONS);

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

        block_on(engine.register(&module));
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

        assert_eq!(object.get_self::<i32>("a").unwrap(), 1);
        let struct_addr = object.get_self::<ObjectAddr>("b").unwrap();

        let st = engine.object_pool().get(struct_addr).unwrap();
        assert_eq!(st.get_self::<i32>("x").unwrap(), 2);
        assert_eq!(st.get_self::<i32>("y").unwrap(), 3);
    }

    #[test]
    fn build_script_object_array_of_structs() {
        let engine = Engine::new(COMPILER_OPTIONS);

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

        block_on(engine.register(&module));
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

        unsafe {
            assert_eq!(object.get_self::<i32>("a").unwrap(), 1);
            let array_ptr = object.get_self::<ObjectAddr>("b").unwrap();
            let data_ptr = array_ptr
                .to_raw()
                .add(offset_of!(ArrayObject, ptr))
                .cast::<ObjectAddr>()
                .read();

            let object0 = data_ptr.to_raw().cast::<ObjectAddr>().read();
            let object1 = data_ptr.to_raw().cast::<ObjectAddr>().add(1).read();

            let object0 = engine.object_pool().get(object0).unwrap();
            assert_eq!(object0.get_self::<i32>("x",).unwrap(), 2);
            assert_eq!(object0.get_self::<i32>("y").unwrap(), 3);

            let object1 = engine.object_pool().get(object1).unwrap();
            assert_eq!(object1.get_self::<i32>("x",).unwrap(), 4);
            assert_eq!(object1.get_self::<i32>("y").unwrap(), 5);
        }
    }

    #[test]
    fn build_script_object_parent_class_property() {
        let engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class A:

            global a: int = 0
            property a: int -> a

            class B:
            parent: A
            "#,
        );

        block_on(engine.register(&module));
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

        let a = engine.get_type("A").unwrap();

        assert_eq!(object.get::<i32>(a.id(), "a").unwrap(), 1);
    }

    #[test]
    fn build_script_object_prefer_subclass_property() {
        let engine = Engine::new(COMPILER_OPTIONS);

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

        block_on(engine.register(&module));
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

        let a = engine.get_type("A").unwrap();
        let b = engine.get_type("B").unwrap();

        assert_eq!(object.get::<i32>(a.id(), "a").unwrap(), 0);
        assert_eq!(object.get::<i32>(b.id(), "a").unwrap(), 1);
    }

    #[test]
    fn build_script_object_reflect_script_name() {
        let engine = Engine::new(COMPILER_OPTIONS);

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

        block_on(engine.register(&module));
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

        let a = engine.object_pool().insert(type_a.id()).addr();
        let b = engine.object_pool().insert(type_b.id()).addr();
        let c = engine.object_pool().insert(type_c.id()).addr();

        let scripts = ScriptObjects::default();
        let partial_addrs = [(
            RecordId::new(ModuleId(NonZeroU32::MIN), FormId(0x01)),
            [(type_a.id(), a), (type_b.id(), b), (type_c.id(), c)]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();

        let aliases = HashMap::new();

        let mut object = engine.object_pool().insert(type_c.id());

        let (_, warnings) = build_script_object(
            |form| RecordId::new(ModuleId(NonZeroU32::MIN), form),
            &engine,
            &script,
            &scripts,
            &partial_addrs,
            &aliases,
            &mut object,
            RecordId::DANGLING,
        );
        assert!(warnings.is_empty());

        assert_eq!(object.get_self::<ObjectAddr>("a").unwrap(), a);
        assert_eq!(object.get_self::<ObjectAddr>("b").unwrap(), b);
        assert_eq!(object.get_self::<ObjectAddr>("c").unwrap(), c);
    }

    #[test]
    fn build_script_object_string() {
        let engine = Engine::new(COMPILER_OPTIONS);

        let module = assemble(
            r#"
            class A:

            global s: string = "s"
            property s: string -> s
            "#,
        );

        block_on(engine.register(&module));
        block_on(engine.compile("A", &Options::default())).unwrap();

        let script = Script {
            name: "A".to_owned(),
            flags: 0,
            properties: vec![Property {
                name: "s".to_owned(),
                value: Value::String("Hello World".to_owned()),
            }],
        };

        let object = build_direct(&engine, &script);

        let type_a = engine.get_type("A").unwrap();

        let s = object.get::<ObjectHandle>(type_a.id(), "s").unwrap();
        assert_eq!(s.string().unwrap(), "Hello World");
    }
}
