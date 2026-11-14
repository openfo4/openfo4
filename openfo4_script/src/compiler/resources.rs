use std::alloc::Layout;
use std::sync::Arc;

use bytemuck::NoUninit;
use hashbrown::HashMap;
use openfo4_common::string::Lowercase;
use parking_lot::Mutex;
use slab::Slab;

use crate::compiler::IPtr;
use crate::runtime::ObjectAddr;

/// A collection of all compiled artifacts required for runnable functions.
#[derive(Debug)]
pub struct Resources {
    pub types: Types,
    pub constants: ConstantPool,
    pub primitives: PrimitiveTypes,
    pub papyrus: PapyrusRes,
}

impl Resources {
    pub fn new() -> Self {
        let types = Types::default();
        let primitives = PrimitiveTypes::new(&types);
        let mut string_to_type = HashMap::default();

        for (id, name) in [
            (primitives.none, Some(Lowercase::from_static("none"))),
            (primitives.bool, Some(Lowercase::from_static("bool"))),
            (primitives.int, Some(Lowercase::from_static("int"))),
            (primitives.float, Some(Lowercase::from_static("float"))),
            (
                primitives.string_ptr,
                Some(Lowercase::from_static("string")),
            ),
            (primitives.var_ptr, Some(Lowercase::from_static("var"))),
        ] {
            if let Some(name) = name {
                string_to_type.insert(name.to_owned(), id);
            }
        }

        Self {
            types,
            primitives,
            constants: ConstantPool::default(),
            papyrus: PapyrusRes {
                types: Mutex::new(string_to_type),
                type_to_array_ptr: Mutex::default(),
                arrays_to_elem: Mutex::default(),
                type_to_array: Mutex::default(),
            },
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
    pub(crate) const fn as_bits(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct Types {
    types: Mutex<Slab<Option<Arc<Type>>>>,
    ptr_types: Mutex<HashMap<TypeId, TypeId>>,
}

impl Types {
    pub fn declare(&self) -> TypeId {
        let id = self.types.lock().insert(None);
        TypeId(id)
    }

    pub fn define(&self, id: TypeId, ty: Type) {
        self.types.lock()[id.0] = Some(Arc::new(ty));
    }

    pub fn get(&self, id: TypeId) -> Option<Arc<Type>> {
        self.types
            .lock()
            .get(id.0)
            .and_then(|v| v.as_ref())
            .cloned()
    }

    /// Returns the pointer type of the given type.
    pub fn ptr_of(&self, ty: TypeId) -> Option<TypeId> {
        self.ptr_types.lock().get(&ty).copied()
    }

    pub fn set_ptr_type(&self, ty: TypeId, ptr_ty: TypeId) {
        self.ptr_types.lock().insert(ty, ptr_ty);
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
    Int {
        bits: u32,
    },
    Float {
        bits: u32,
    },
    Composite(CompositeType),
    /// A pointer to another [`Type`].
    Pointer(TypeId),
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
            Self::Pointer(_) => size_of::<*const ()>(),
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
            Self::Pointer(_) => align_of::<*const ()>(),
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
    pub default_value: Option<Words>,
}

#[derive(Clone, Debug, Default)]
pub struct Words {
    words: Vec<IPtr>,
    len: usize,
}

impl Words {
    pub fn bytes(bytes: &[u8]) -> Self {
        let num_words = bytes.len().div_ceil(size_of::<IPtr>());
        let mut words = vec![IPtr(core::ptr::null_mut()); num_words];

        unsafe {
            let src = bytes.as_ptr();
            let dst = words.as_mut_ptr().cast::<u8>();
            let count = bytes.len();
            core::ptr::copy_nonoverlapping(src, dst, count);
        }

        Self {
            words,
            len: bytes.len(),
        }
    }

    pub fn ptr(ptr: *mut ()) -> Self {
        Self {
            words: vec![IPtr(ptr)],
            len: size_of::<IPtr>(),
        }
    }

    pub fn nullptr() -> Self {
        Self::ptr(core::ptr::null_mut())
    }

    pub fn ptrs(ptrs: &[IPtr]) -> Self {
        Self {
            words: ptrs.to_vec(),
            len: ptrs.len() * size_of::<IPtr>(),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub unsafe fn write(&self, dst: *mut u8) {
        unsafe {
            let src = self.words.as_ptr();
            core::ptr::copy_nonoverlapping(src.cast::<u8>(), dst, self.len);
        }
    }
}

/// Pool for constant memory allocation.
#[derive(Debug, Default)]
pub struct ConstantPool {
    allocs: Mutex<HashMap<*mut u8, Layout>>,
    objects: Mutex<HashMap<ObjectAddr, ConstantObject>>,
}

impl ConstantPool {
    pub fn alloc(&self, buf: &[u8], align: usize) -> *const u8 {
        debug_assert!(align > 0);

        if buf.is_empty() {
            return core::ptr::dangling();
        }

        let layout = Layout::for_value(buf);
        let layout = layout.align_to(align).unwrap();

        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null());

        let mut allocs = self.allocs.lock();
        allocs.insert(ptr, layout);

        unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), ptr, buf.len());
        }

        ptr
    }

    pub fn alloc_words(&self, words: &Words, align: usize) -> *const u8 {
        debug_assert!(align > 0);

        if words.len() == 0 {
            return core::ptr::dangling();
        }

        let layout = Layout::from_size_align(words.len, align).unwrap();

        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null());

        let mut allocs = self.allocs.lock();
        allocs.insert(ptr, layout);

        unsafe {
            words.write(ptr);
        }

        ptr
    }

    pub fn alloc_object(&self, buf: &[u8], align: usize, ty: TypeId) -> *const u8 {
        let ptr = self.alloc(buf, align);
        self.objects.lock().insert(
            ObjectAddr::from_raw(ptr.cast_mut()),
            ConstantObject { ptr, ty },
        );
        ptr
    }

    pub fn object(&self, addr: ObjectAddr) -> Option<ConstantObject> {
        self.objects.lock().get(&addr).copied()
    }
}

impl Drop for ConstantPool {
    fn drop(&mut self) {
        for (ptr, layout) in self.allocs.get_mut().drain() {
            unsafe {
                std::alloc::dealloc(ptr, layout);
            }
        }
    }
}

unsafe impl Send for ConstantPool {}
unsafe impl Sync for ConstantPool {}

#[derive(Copy, Clone, Debug)]
pub struct ConstantObject {
    pub ptr: *const u8,
    pub ty: TypeId,
}

unsafe impl Send for ConstantObject {}
unsafe impl Sync for ConstantObject {}

/// The set of primitive types.
#[derive(Copy, Clone, Debug)]
pub struct PrimitiveTypes {
    pub none: TypeId,
    pub bool: TypeId,
    pub int: TypeId,
    pub float: TypeId,
    pub string: TypeId,
    pub byte: TypeId,
    pub var: TypeId,
    /// An untyped pointer.
    pub opaqueptr: TypeId,
    pub string_ptr: TypeId,
    pub var_ptr: TypeId,
}

impl PrimitiveTypes {
    pub fn new(types: &Types) -> Self {
        let none = types.declare();
        types.define(
            none,
            Type {
                repr: TypeRepr::Unit,
                info: TypeInfo {
                    id: none,
                    name: Some(Lowercase::new("none")),
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let bool = types.declare();
        types.define(
            bool,
            Type {
                repr: TypeRepr::Int { bits: 8 },
                info: TypeInfo {
                    id: bool,
                    name: Some(Lowercase::new("bool")),
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let int = types.declare();
        types.define(
            int,
            Type {
                repr: TypeRepr::Int { bits: 32 },
                info: TypeInfo {
                    id: int,
                    name: Some(Lowercase::new("int")),
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let float = types.declare();
        types.define(
            float,
            Type {
                repr: TypeRepr::Float { bits: 32 },
                info: TypeInfo {
                    id: float,
                    name: Some(Lowercase::new("float")),
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let opaqueptr = types.declare();
        types.define(
            opaqueptr,
            Type {
                repr: TypeRepr::Int { bits: usize::BITS },
                info: TypeInfo {
                    id: opaqueptr,
                    name: Some(Lowercase::new("opaqueptr")),
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let byte = types.declare();
        types.define(
            byte,
            Type {
                repr: TypeRepr::Int { bits: 8 },
                info: TypeInfo {
                    id: byte,
                    name: None,
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
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
                            ty: opaqueptr,
                            offset: 0,
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                        CompositeTypeMember {
                            ty: opaqueptr,
                            offset: size_of::<usize>(),
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                    ],
                    size: 2 * size_of::<usize>(),
                    align: align_of::<usize>(),
                }),
                info: TypeInfo {
                    id: string,
                    name: Some(Lowercase::new("string")),
                    properties: TypeProperties::default(),
                    allows_null: false,
                    lock: None,
                    supertypes: Box::new([]),
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
                            ty: opaqueptr,
                            offset: 0,
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                        CompositeTypeMember {
                            ty: opaqueptr,
                            offset: size_of::<usize>(),
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                    ],
                    size: 2 * size_of::<usize>(),
                    align: align_of::<usize>(),
                }),
                info: TypeInfo {
                    id: array,
                    name: None,
                    properties: TypeProperties::default(),
                    allows_null: false,
                    lock: None,
                    supertypes: Box::new([]),
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
                            ty: opaqueptr,
                            offset: 0,
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                        CompositeTypeMember {
                            ty: opaqueptr,
                            offset: size_of::<usize>(),
                            default_value: None,
                            size: size_of::<usize>(),
                        },
                    ],
                    size: 2 * size_of::<usize>(),
                    align: align_of::<usize>(),
                }),
                info: TypeInfo {
                    id: var,
                    name: Some(Lowercase::new("var")),
                    properties: TypeProperties::default(),
                    allows_null: false,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let string_ptr = types.declare();
        types.define(
            string_ptr,
            Type {
                repr: TypeRepr::Pointer(string),
                info: TypeInfo {
                    id: string_ptr,
                    name: Some(Lowercase::new("*string")),
                    properties: TypeProperties::default(),
                    allows_null: false,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        let var_ptr = types.declare();
        types.define(
            var_ptr,
            Type {
                repr: TypeRepr::Pointer(var),
                info: TypeInfo {
                    id: var_ptr,
                    name: Some(Lowercase::new("*var")),
                    properties: TypeProperties::default(),
                    allows_null: true,
                    lock: None,
                    supertypes: Box::new([]),
                },
            },
        );

        types.set_ptr_type(string, string_ptr);
        types.set_ptr_type(var, var_ptr);

        Self {
            none,
            bool,
            int,
            float,
            string,
            byte,
            var,
            opaqueptr,
            string_ptr,
            var_ptr,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypeInfo {
    pub(crate) id: TypeId,
    pub(crate) name: Option<Lowercase<String>>,
    pub properties: TypeProperties,
    pub allows_null: bool,
    /// The offset from the object start where the [`ObjectLock`] is located.
    pub(crate) lock: Option<usize>,
    /// Sorted list of supertypes of this type.
    ///
    /// Index 0 contains the most generic supertype, while the last index contains the immediate
    /// supertype of this type.
    pub(crate) supertypes: Box<[TypeId]>,
}

impl TypeInfo {
    /// Returns the unique ID of this type.
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// Returns the supertype of this type.
    pub fn supertype(&self) -> Option<TypeId> {
        self.supertypes.last().copied()
    }

    /// Returns whether `self` can be implicitly coerced into the given `target` type.
    ///
    /// This is possible if `self` is `target` or `self` is a subtype of `target`.
    pub fn is_coercible(&self, target: TypeId) -> bool {
        self.id == target || self.supertypes.contains(&target)
    }

    /// Returns a debug name for this type.
    ///
    /// The value has no semantic meaning and is only meant for debugging.
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|s| s.as_str().as_inner())
    }

    pub(crate) fn name_or_placeholder(&self) -> &str {
        self.name().unwrap_or("<unnamed>")
    }

    /// Returns the first property with the given `name`.
    ///
    /// This will search the supertype recursively if the type itself does not have the matching
    /// property. For any two types in the hierarchy that both have the matching `name` it is
    /// guaranteed that the subtype property is returned.
    pub fn find_property(&self, name: &Lowercase<str>) -> Option<&TypeProperty> {
        let mut id = self.id;
        let mut supertypes = &*self.supertypes;

        loop {
            if let Some(field) = self.properties.get(id, name) {
                return Some(field);
            }

            let (ty, rem) = supertypes.split_last()?;
            supertypes = rem;
            id = *ty;
        }
    }

    /// To be removed later.
    pub fn papyrus_name(&self) -> Option<Lowercase<String>> {
        let mut name = self.name.as_ref()?.as_str();
        name = name.strip_prefix("*").unwrap_or(&name);
        Some(name.to_owned())
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypeProperties {
    properties: HashMap<TypeId, HashMap<Lowercase<String>, TypeProperty>>,
}

impl TypeProperties {
    pub fn get(&self, ty: TypeId, name: &Lowercase<str>) -> Option<&TypeProperty> {
        self.properties.get(&ty)?.get(name)
    }

    /// Returns an [`Iterator`] over all properties of this type.
    ///
    /// The order the properties are returned are unspecified.
    pub fn iter(&self) -> impl Iterator<Item = (TypeId, &Lowercase<str>, &TypeProperty)> {
        self.properties
            .iter()
            .map(|(ty, fields)| fields.iter().map(|(name, prop)| (*ty, name.as_str(), prop)))
            .flatten()
    }

    pub(crate) fn insert(&mut self, ty: TypeId, name: Lowercase<String>, prop: TypeProperty) {
        self.properties.entry(ty).or_default().insert(name, prop);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TypeProperty {
    pub ty: TypeId,
    /// The field where the data is stored.
    pub field: usize,
}

#[derive(Debug)]
pub(crate) struct PapyrusRes {
    /// High-level string to type mapping.
    ///
    /// For non-primitive types the returned value will be the pointer type, not the inline type.
    pub types: Mutex<HashMap<Lowercase<String>, TypeId>>,
    /// Maps from a type to the array pointer type.
    pub type_to_array_ptr: Mutex<HashMap<TypeId, TypeId>>,
    pub type_to_array: Mutex<HashMap<TypeId, TypeId>>,
    /// Maps from array type (not array pointer) to element type.
    pub arrays_to_elem: Mutex<HashMap<TypeId, TypeId>>,
}

pub fn define_array_ty(
    res: &Resources,
    ty: TypeId,
    array_ty: TypeId,
    array_ptr_ty: TypeId,
    name: &str,
) {
    res.types.define(
        array_ty,
        Type {
            repr: TypeRepr::Composite(CompositeType {
                members: vec![
                    CompositeTypeMember {
                        ty: ty,
                        offset: 0,
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                    CompositeTypeMember {
                        ty: res.primitives.opaqueptr,
                        offset: size_of::<usize>(),
                        default_value: None,
                        size: size_of::<usize>(),
                    },
                ],
                size: 2 * size_of::<usize>(),
                align: align_of::<usize>(),
            }),
            info: TypeInfo {
                id: array_ty,
                name: Some(Lowercase::new(&*format!("{}[]", name))),
                properties: TypeProperties::default(),
                allows_null: false,
                lock: None,
                supertypes: Box::new([]),
            },
        },
    );

    res.types.define(
        array_ptr_ty,
        Type {
            repr: TypeRepr::Pointer(array_ty),
            info: TypeInfo {
                id: array_ptr_ty,
                name: Some(Lowercase::new(&*format!("*{}[]", name))),
                properties: TypeProperties::default(),
                allows_null: true,
                lock: None,
                supertypes: Box::new([]),
            },
        },
    );
}

#[cfg(test)]
mod tests {
    use openfo4_common::string::Lowercase;

    use crate::compiler::resources::{TypeId, TypeInfo, TypeProperties, TypeProperty};

    #[test]
    fn type_info_find_property() {
        let mut properties = TypeProperties::default();

        for (id, name, field) in [
            // Only in self type.
            (TypeId(0), "a", 0),
            // In self type and immediate supertype.
            (TypeId(1), "b", 1),
            (TypeId(0), "b", 2),
            // In self type and recursive supertype.
            (TypeId(2), "c", 3),
            (TypeId(0), "c", 4),
            // In immediate supertype and recursive supertype.
            (TypeId(2), "d", 5),
            (TypeId(1), "d", 6),
            // Only in recursive supertype.
            (TypeId(2), "e", 7),
        ] {
            properties.insert(
                id,
                Lowercase::new(name),
                TypeProperty {
                    ty: TypeId(0),
                    field,
                },
            );
        }

        let info = TypeInfo {
            id: TypeId(0),
            name: None,
            properties,
            allows_null: false,
            lock: None,
            supertypes: Box::new([TypeId(2), TypeId(1)]),
        };

        assert_eq!(info.find_property(&Lowercase::new("a")).unwrap().field, 0);
        assert_eq!(info.find_property(&Lowercase::new("b")).unwrap().field, 2);
        assert_eq!(info.find_property(&Lowercase::new("c")).unwrap().field, 4);
        assert_eq!(info.find_property(&Lowercase::new("d")).unwrap().field, 6);
        assert_eq!(info.find_property(&Lowercase::new("e")).unwrap().field, 7);
    }
}
