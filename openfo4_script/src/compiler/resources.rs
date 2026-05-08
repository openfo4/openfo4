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
    pub infos: Mutex<HashMap<TypeId, TypeInfo>>,
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
            infos: Mutex::new(infos),
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
pub enum Type {
    Unit,
    Int { bits: u32 },
    Float { bits: u32 },
    Composite(CompositeType),
}

impl Type {
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
    /// Optional name of this member.
    pub name: Option<Name>,
    /// Optional default value of this member.
    pub default_value: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Name {
    Struct {
        field: Lowercase<String>,
    },
    Class {
        class: Lowercase<String>,
        field: Lowercase<String>,
    },
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
        types.define(none, Type::Unit);

        let bool = types.declare();
        types.define(bool, Type::Int { bits: 8 });

        let int = types.declare();
        types.define(int, Type::Int { bits: 32 });

        let float = types.declare();
        types.define(float, Type::Float { bits: 32 });

        let pointer = types.declare();
        types.define(pointer, Type::Int { bits: usize::BITS });

        let byte = types.declare();
        types.define(byte, Type::Int { bits: 8 });

        let string = types.declare();
        types.define(
            string,
            Type::Composite(CompositeType {
                members: vec![
                    CompositeTypeMember {
                        ty: pointer,
                        offset: 0,
                        name: None,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                    CompositeTypeMember {
                        ty: pointer,
                        offset: size_of::<usize>(),
                        name: None,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                ],
                size: 2 * size_of::<usize>(),
                align: align_of::<usize>(),
            }),
        );

        let array = types.declare();
        types.define(
            array,
            Type::Composite(CompositeType {
                members: vec![
                    CompositeTypeMember {
                        ty: pointer,
                        offset: 0,
                        name: None,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                    CompositeTypeMember {
                        ty: pointer,
                        offset: size_of::<usize>(),
                        name: None,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                ],
                size: 2 * size_of::<usize>(),
                align: align_of::<usize>(),
            }),
        );

        let var = types.declare();
        types.define(
            var,
            Type::Composite(CompositeType {
                members: vec![
                    CompositeTypeMember {
                        ty: pointer,
                        offset: 0,
                        name: None,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                    CompositeTypeMember {
                        ty: pointer,
                        offset: size_of::<usize>(),
                        name: None,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                ],
                size: 2 * size_of::<usize>(),
                align: align_of::<usize>(),
            }),
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
