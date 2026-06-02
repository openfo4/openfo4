use std::alloc::Layout;

use bytemuck::NoUninit;
use game_common::string::Lowercase;
use hashbrown::HashMap;
use parking_lot::Mutex;
use slab::Slab;

/// A collection of all compiled artifacts required for runnable functions.
#[derive(Debug)]
pub struct Resources {
    pub types: Types,
    pub constants: ConstantPool,
    pub primitives: PrimitiveTypes,
    pub string_to_type: Mutex<HashMap<Lowercase<String>, TypeId>>,
}

impl Resources {
    pub fn new() -> Self {
        let types = Types::default();
        let primitives = PrimitiveTypes::new(&types);
        let mut string_to_type = HashMap::default();

        let mut infos = HashMap::new();
        for (id, name) in [
            (primitives.none, Some(Lowercase::from_static("none"))),
            (primitives.bool, Some(Lowercase::from_static("bool"))),
            (primitives.int, Some(Lowercase::from_static("int"))),
            (primitives.float, Some(Lowercase::from_static("float"))),
            (primitives.string, Some(Lowercase::from_static("string"))),
            (primitives.var, Some(Lowercase::from_static("var"))),
            (primitives.pointer, None),
        ] {
            infos.insert(
                id,
                TypeInfo {
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(id),
                            field: 0,
                        },
                    )]),
                    name: name.map(|v| v.to_owned()),
                },
            );

            if let Some(name) = name {
                string_to_type.insert(name.to_owned(), id);
            }
        }

        Self {
            types,
            primitives,
            constants: ConstantPool::default(),
            string_to_type: Mutex::new(string_to_type),
        }
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

// FIXME: Remove `Default` derive.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, NoUninit)]
#[repr(transparent)]
pub struct TypeId(usize);

impl TypeId {
    pub(crate) const DANGLING: Self = Self(usize::MAX);

    pub(crate) const fn as_bits(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct Types {
    types: Mutex<Slab<Option<Type>>>,
}

impl Types {
    pub fn declare(&self) -> TypeId {
        let id = self.types.lock().insert(None);
        TypeId(id)
    }

    pub fn define(&self, id: TypeId, ty: Type) {
        self.types.lock()[id.0] = Some(ty);
    }

    pub fn get(&self, id: TypeId) -> Option<Type> {
        self.types
            .lock()
            .get(id.0)
            .and_then(|v| v.as_ref())
            .cloned()
    }
}

#[derive(Clone, Debug)]
pub struct Type {
    pub repr: TypeRepr,
    pub info: TypeInfo,
}

/// Description of the binary representation of the type.
#[derive(Clone, Debug)]
pub enum TypeRepr {
    Unit,
    Int { bits: u32 },
    Float { bits: u32 },
    Composite(CompositeType),
}

impl TypeRepr {
    pub fn size(&self) -> usize {
        match self {
            Self::Unit => 0,
            Self::Int { bits } | Self::Float { bits } if *bits == 0 => 0,
            Self::Int { bits } | Self::Float { bits } => {
                bits.div_ceil(8).next_power_of_two() as usize
            }
            Self::Composite(ty) => ty.size,
        }
    }

    pub fn align(&self) -> usize {
        match self {
            Self::Unit => 1,
            Self::Int { bits } | Self::Float { bits } if *bits == 0 => 1,
            Self::Int { bits } | Self::Float { bits } => {
                bits.div_ceil(8).next_power_of_two() as usize
            }
            Self::Composite(ty) => ty.align,
        }
    }

    /// Returns [`size`] rounded up to the next multiple of [`align`].
    ///
    /// [`size`]: Self::size
    /// [`align`]: Self::align
    pub fn size_padded(&self) -> usize {
        let size = self.size();
        let align = self.align();
        (size + align - 1) & !(align - 1)
    }
}

#[derive(Clone, Debug)]
pub struct CompositeType {
    pub members: Vec<CompositeTypeMember>,
    /// The total size of this composite type, in bytes.
    pub size: usize,
    /// The alignment required for all instances of this composite type.
    pub align: usize,
}

#[derive(Clone, Debug)]
pub struct CompositeTypeMember {
    /// The type of this member.
    pub ty: TypeId,
    /// Offset in bytes from the start of the composite structure.
    pub offset: usize,
    pub size: usize,
    /// Optional default value of this member.
    pub default_value: Option<Vec<u8>>,
}

/// Pool for constant memory allocation.
#[derive(Clone, Debug, Default)]
pub struct ConstantPool {}

impl ConstantPool {
    pub fn alloc(&self, buf: &[u8], align: usize) -> *const u8 {
        debug_assert!(align > 0);

        let layout = Layout::for_value(buf);
        let layout = layout.align_to(align).unwrap();

        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null());

        unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), ptr, buf.len());
        }

        ptr
    }
}

/// The set of primitive types.
#[derive(Copy, Clone, Debug)]
pub struct PrimitiveTypes {
    pub none: TypeId,
    pub bool: TypeId,
    pub int: TypeId,
    pub float: TypeId,
    pub string: TypeId,
    pub array: TypeId,
    pub pointer: TypeId,
    pub byte: TypeId,
    pub var: TypeId,
}

impl PrimitiveTypes {
    pub fn new(types: &Types) -> Self {
        let none = types.declare();
        types.define(
            none,
            Type {
                repr: TypeRepr::Unit,
                info: TypeInfo {
                    name: Some(Lowercase::new("none")),
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(none),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let bool = types.declare();
        types.define(
            bool,
            Type {
                repr: TypeRepr::Int { bits: 8 },
                info: TypeInfo {
                    name: Some(Lowercase::new("bool")),
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(bool),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let int = types.declare();
        types.define(
            int,
            Type {
                repr: TypeRepr::Int { bits: 32 },
                info: TypeInfo {
                    name: Some(Lowercase::new("int")),
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(int),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let float = types.declare();
        types.define(
            float,
            Type {
                repr: TypeRepr::Float { bits: 32 },
                info: TypeInfo {
                    name: Some(Lowercase::new("float")),
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(float),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let pointer = types.declare();
        types.define(
            pointer,
            Type {
                repr: TypeRepr::Int { bits: usize::BITS },
                info: TypeInfo {
                    name: None,
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(pointer),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let byte = types.declare();
        types.define(
            byte,
            Type {
                repr: TypeRepr::Int { bits: 8 },
                info: TypeInfo {
                    name: None,
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(byte),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let string = types.declare();
        types.define(
            string,
            Type {
                repr: TypeRepr::Composite(CompositeType {
                    members: vec![
                        CompositeTypeMember {
                            ty: pointer,
                            offset: 0,
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                        CompositeTypeMember {
                            ty: pointer,
                            offset: size_of::<usize>(),
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                    ],
                    size: 2 * size_of::<usize>(),
                    align: align_of::<usize>(),
                }),
                info: TypeInfo {
                    name: Some(Lowercase::new("string")),
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(string),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        let array = types.declare();
        types.define(
            array,
            Type {
                repr: TypeRepr::Composite(CompositeType {
                    members: vec![
                        CompositeTypeMember {
                            ty: pointer,
                            offset: 0,
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                        CompositeTypeMember {
                            ty: pointer,
                            offset: size_of::<usize>(),
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                    ],
                    size: 2 * size_of::<usize>(),
                    align: align_of::<usize>(),
                }),
                info: TypeInfo {
                    name: None,
                    classes: Vec::new(),
                    properties: HashMap::new(),
                },
            },
        );

        let var = types.declare();
        types.define(
            var,
            Type {
                repr: TypeRepr::Composite(CompositeType {
                    members: vec![
                        CompositeTypeMember {
                            ty: pointer,
                            offset: 0,
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                        CompositeTypeMember {
                            ty: pointer,
                            offset: size_of::<usize>(),
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                    ],
                    size: 2 * size_of::<usize>(),
                    align: align_of::<usize>(),
                }),
                info: TypeInfo {
                    name: Some(Lowercase::new("var")),
                    classes: Vec::new(),
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: PropertyType::Inline(var),
                            field: 0,
                        },
                    )]),
                },
            },
        );

        Self {
            none,
            bool,
            int,
            float,
            string,
            array,
            pointer,
            byte,
            var,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypeInfo {
    pub name: Option<Lowercase<String>>,
    pub classes: Vec<TypeId>,
    pub properties: HashMap<(Lowercase<String>, Lowercase<String>), TypeProperty>,
}

#[derive(Clone, Debug)]
pub struct TypeProperty {
    pub ty: PropertyType,
    /// The field where the data is stored.
    pub field: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PropertyType {
    Inline(TypeId),
    Pointer(TypeId),
    ArrayPointer(TypeId),
}
