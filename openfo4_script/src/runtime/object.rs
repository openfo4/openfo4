use std::alloc::Layout;
use std::sync::{Arc, Mutex};

use game_common::string::Lowercase;
use hashbrown::HashMap;

use crate::compiler::resources::{Resources, Type, TypeId, TypeRepr};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ObjectAddr(*mut u8);

impl ObjectAddr {
    pub const NULL: Self = Self(core::ptr::null_mut());

    pub const fn from_raw(ptr: *mut u8) -> Self {
        Self(ptr)
    }

    pub const fn to_raw(self) -> *mut u8 {
        self.0
    }

    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl AsRef<ObjectAddr> for ObjectAddr {
    fn as_ref(&self) -> &ObjectAddr {
        self
    }
}

unsafe impl Send for ObjectAddr {}
unsafe impl Sync for ObjectAddr {}

#[derive(Debug)]
pub struct ObjectPool {
    resources: Arc<Resources>,
    objects: Mutex<HashMap<ObjectAddr, Object>>,
    dropped_refs: Arc<Mutex<Vec<ObjectAddr>>>,
}

impl ObjectPool {
    pub(crate) fn new(resources: Arc<Resources>) -> Self {
        Self {
            resources,
            objects: Mutex::default(),
            dropped_refs: Arc::default(),
        }
    }

    pub fn alloc(&self, ty: TypeId, array_len: usize) -> ObjectRef {
        let typ = self.resources.types.get(ty).unwrap();

        let size = typ.repr.size();
        let align = typ.repr.align();
        let stride = (size + align - 1) & !(align - 1);

        let array_size = stride * array_len;

        let alloc_layout = unsafe { Layout::from_size_align_unchecked(array_size, align) };

        // We must not allocate zero sized memory.
        // Any well-aligned pointer is valid for a zero sized allocation.
        let ptr = if alloc_layout.size() == 0 {
            unsafe { core::ptr::null_mut::<u8>().add(alloc_layout.align()) }
        } else {
            unsafe { std::alloc::alloc_zeroed(alloc_layout) }
        };

        if ptr.is_null() {
            std::alloc::handle_alloc_error(alloc_layout);
        }

        let mut objects = self.objects.lock().unwrap();

        for i in 0..array_len {
            let elem_ptr = unsafe { ptr.add(stride * i) };
            let addr = ObjectAddr::from_raw(elem_ptr);

            let object = unsafe { Object::new_at(ty, typ.clone(), elem_ptr) };

            objects.insert(addr, object);
        }

        ObjectRef(Arc::new(ObjectRefInner {
            addr: ObjectAddr(ptr),
            drop: self.dropped_refs.clone(),
        }))
    }

    pub fn insert(&self, ty: TypeId) -> ObjectBuilder {
        let object_ref = self.alloc(ty, 1);
        let typ = self.resources.types.get(ty).unwrap();

        let object = unsafe { Object::new_at(ty, typ, object_ref.addr().0 as *mut u8) };

        ObjectBuilder {
            obj: object,
            obj_ref: object_ref,
        }
    }

    pub fn insert_string(&self, s: &str) -> ObjectRef {
        let bytes_ref = self.alloc(self.resources.primitives.byte, s.len());

        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), bytes_ref.as_ptr(), s.len());
        }

        let mut string_object = self.insert(self.resources.primitives.string);
        let string = unsafe { string_object.as_mut::<ArrayObject>() };
        string.ptr = bytes_ref.addr().to_raw();
        string.len = s.len();

        string_object.finalize()
    }

    pub fn insert_array(&self, elem_ty: TypeId, array_len: usize) -> ObjectArrayBuilder {
        let array_ref = self.alloc(elem_ty, array_len);
        let elem_typ = self.resources.types.get(elem_ty).unwrap();

        let mut objects = Vec::new();

        let size = elem_typ.repr.size();
        let align = elem_typ.repr.align();
        let stride = (size + align - 1) & !(align - 1);

        for i in 0..array_len {
            let elem_ptr = unsafe { array_ref.as_ptr().add(stride * i) };

            let object = unsafe { Object::new_at(elem_ty, elem_typ.clone(), elem_ptr) };
            objects.push(object);
        }

        let mut array_object = self.insert(self.resources.primitives.array);
        let array = unsafe { array_object.as_mut::<ArrayObject>() };
        array.ptr = array_ref.addr().to_raw();
        array.len = array_len;

        ObjectArrayBuilder {
            objects,
            object_ref: array_object.finalize(),
        }
    }

    /// Clones an object.
    ///
    /// # Safety
    ///
    /// No thread must currently access the given object referenced by the [`ObjectRef`].
    pub unsafe fn clone_object(&self, object: &ObjectRef) -> ObjectBuilder {
        let ty_id = {
            let objects = self.objects.lock().unwrap();

            let object = objects.get(&object.addr()).unwrap();
            object.ty_id
        };

        let builder = self.insert(ty_id);

        let mut objects = self.objects.lock().unwrap();
        let object = objects.get_mut(&object.addr()).unwrap();

        unsafe {
            let src = object.as_ptr().cast_const();
            let dst = builder.obj.as_ptr();
            let count = object.size();
            debug_assert_eq!(object.size(), builder.obj.size());

            core::ptr::copy_nonoverlapping(src, dst, count);
        }

        builder
    }

    // pub fn get(&self, addr: ObjectAddr) -> Option<&Object> {
    //     self.objects.get(&addr)
    // }

    // pub fn get_mut(&mut self, addr: ObjectAddr) -> Option<&mut Object> {
    //     self.objects.get_mut(&addr)
    // }

    pub fn get<T>(&self, object: T) -> Object
    where
        T: AsRef<ObjectAddr>,
    {
        let addr = *object.as_ref();

        self.objects.lock().unwrap().get(&addr).unwrap().clone()
    }

    pub unsafe fn array<U>(&self, addr: ObjectAddr) -> &[U] {
        unsafe {
            let object = &*addr.to_raw().cast::<ArrayObject>();

            // Array with len 0 MAY have a zero pointer that we cannot use,
            // even for a zero-sized slice.
            if object.len == 0 {
                &[]
            } else {
                core::slice::from_raw_parts(object.ptr.cast::<U>(), object.len)
            }
        }
    }

    pub unsafe fn array_mut<U>(&mut self, addr: ObjectAddr) -> &mut [U] {
        unsafe {
            let object = &*addr.to_raw().cast::<ArrayObject>();

            // Array with len 0 MAY have a zero pointer that we cannot use,
            // even for a zero-sized slice.
            if object.len == 0 {
                &mut []
            } else {
                core::slice::from_raw_parts_mut(object.ptr.cast::<U>(), object.len)
            }
        }
    }

    pub unsafe fn string(&self, addr: ObjectAddr) -> &str {
        unsafe {
            let data = self.array::<u8>(addr);
            std::str::from_utf8_unchecked(data)
        }
    }

    pub(crate) fn resources(&self) -> &Resources {
        &self.resources
    }
}

/// A reference handle to an [`Object`] in a pool.
#[derive(Clone, Debug)]
pub struct ObjectRef(Arc<ObjectRefInner>);

impl ObjectRef {
    pub fn addr(&self) -> ObjectAddr {
        self.0.addr
    }

    fn as_ptr(&self) -> *mut u8 {
        self.0.addr.0
    }
}

impl AsRef<ObjectAddr> for ObjectRef {
    fn as_ref(&self) -> &ObjectAddr {
        &self.0.addr
    }
}

#[derive(Debug)]
struct ObjectRefInner {
    addr: ObjectAddr,
    drop: Arc<Mutex<Vec<ObjectAddr>>>,
}

impl Drop for ObjectRefInner {
    fn drop(&mut self) {
        self.drop.lock().unwrap().push(self.addr);
    }
}

#[derive(Clone, Debug)]
pub struct Object {
    memory: *mut u8,
    ty: Type,
    ty_id: TypeId,
}

impl Object {
    unsafe fn new_at(ty_id: TypeId, ty: Type, ptr: *mut u8) -> Self {
        if let TypeRepr::Composite(ty) = &ty.repr {
            for member in &ty.members {
                let Some(value) = &member.default_value else {
                    continue;
                };

                debug_assert_eq!(value.len(), member.size);

                unsafe {
                    core::ptr::copy_nonoverlapping(
                        value.as_ptr(),
                        ptr.add(member.offset),
                        member.size,
                    );
                }
            }
        }

        Self {
            memory: ptr,
            ty,
            ty_id,
        }
    }

    fn get_prop_offset_and_size(&self, class: &str, name: &str) -> Option<(usize, usize)> {
        let index = self.get_property_var(class, name);

        let TypeRepr::Composite(ty) = &self.ty.repr else {
            return Some((0, self.size()));
        };

        let field = &ty.members[index];

        Some((field.offset, field.size))
    }

    pub unsafe fn get(&self, class: &str, name: &str) -> Option<&[u8]> {
        let (offset, size) = self.get_prop_offset_and_size(class, name)?;

        unsafe {
            let ptr = self.as_ptr().add(offset);
            Some(core::slice::from_raw_parts(ptr, size))
        }
    }

    /// # Safety
    ///
    /// The caller must guarantee that no memory accesses to the field of the `Object` are
    /// currently active.
    #[track_caller]
    pub unsafe fn set<T>(&mut self, class: &str, name: &str, value: T)
    where
        T: IntoValue,
    {
        let (offset, size) = self.get_prop_offset_and_size(class, name).unwrap();

        let bytes = value.as_bytes();

        assert_eq!(
            bytes.len(),
            size,
            "{:?}.{:?} requires {} bytes, but {} were provided",
            class,
            name,
            size,
            bytes.len(),
        );

        unsafe {
            let dst = self.memory.add(offset);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        }
    }

    #[track_caller]
    fn get_property_var(&self, class: &str, name: &str) -> usize {
        let class = Lowercase::new(class);
        let name = Lowercase::new(name);

        // Always prefer properties that match the requested class.
        if let Some((_, prop)) = self
            .ty
            .info
            .properties
            .iter()
            .find(|((cl, fl), _)| *cl == class && *fl == name)
        {
            return prop.field;
        }

        // We may also access fields of parent classes.
        // FIXME: Ideally we would recursively walk up the class
        // hierarchy instead of selecting any other matching property,
        // this requires us to have access to the class names from here.
        if let Some((_, prop)) = self
            .ty
            .info
            .properties
            .iter()
            .find(|((_, fl), _)| *fl == name)
        {
            return prop.field;
        }

        panic!("no field {:?}.{:?}", class, name);
    }

    /// View the entire memory of this `Object`.
    pub unsafe fn memory(&self) -> &[u8] {
        let ptr = self.as_ptr();
        let len = self.size();
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.memory
    }

    pub fn size(&self) -> usize {
        self.ty.repr.size()
    }

    pub fn type_id(&self) -> TypeId {
        self.ty_id
    }
}

unsafe impl Send for Object {}
unsafe impl Sync for Object {}

#[derive(Debug)]
pub struct ObjectBuilder {
    obj: Object,
    obj_ref: ObjectRef,
}

impl ObjectBuilder {
    #[track_caller]
    pub fn set<T>(&mut self, class: &str, name: &str, value: T)
    where
        T: IntoValue,
    {
        // SAFETY:
        // We don't give access to the underlying `ObjectRef` until the `ObjectBuilder` is dropped.
        // This means the caller does not have access to the pointer to pass anywhere else yet.
        unsafe {
            self.obj.set(class, name, value);
        }
    }

    pub unsafe fn finalize_copy(&self) -> ObjectRef {
        self.obj_ref.clone()
    }

    pub fn finalize(self) -> ObjectRef {
        self.obj_ref
    }

    /// Returns the address of the object being built.
    ///
    /// Useful for building recursive objects.
    pub fn addr(&self) -> ObjectAddr {
        self.obj_ref.addr()
    }

    unsafe fn as_mut<T>(&mut self) -> &mut T {
        unsafe { &mut *self.obj.as_ptr().cast() }
    }
}

#[derive(Debug)]
pub struct ObjectArrayBuilder {
    objects: Vec<Object>,
    object_ref: ObjectRef,
}

impl ObjectArrayBuilder {
    #[track_caller]
    pub fn set_at<T>(&mut self, index: usize, class: &str, name: &str, value: T)
    where
        T: IntoValue,
    {
        let object = &mut self.objects[index];

        unsafe {
            object.set(class, name, value);
        }
    }

    pub fn finalize(self) -> ObjectRef {
        self.object_ref
    }
}

pub trait IntoValue {
    fn as_bytes(&self) -> &[u8];
}

impl IntoValue for bool {
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl IntoValue for i32 {
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl IntoValue for f32 {
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl IntoValue for usize {
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl IntoValue for ObjectAddr {
    fn as_bytes(&self) -> &[u8] {
        let addr = unsafe { core::mem::transmute::<&*mut u8, &usize>(&self.0) };
        bytemuck::bytes_of(addr)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ArrayObject {
    pub ptr: *mut u8,
    pub len: usize,
}

impl ArrayObject {
    pub fn as_bytes(&self) -> &[u8; size_of::<Self>()] {
        // SAFETY: We have no padding and all bit patterns are valid.
        unsafe { core::mem::transmute::<&Self, &[u8; size_of::<Self>()]>(self) }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) struct VarObject {
    /// Raw version of an [`ObjectTag`].
    pub tag: usize,
    pub data: VarObjectData,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) union VarObjectData {
    pub bool: bool,
    pub int: i32,
    pub float: f32,
    pub class: ObjectAddr,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ObjectTag {
    /// A class pointer.
    Class,
    TypeId(TypeId),
}

impl ObjectTag {
    pub fn to_bits(self) -> usize {
        match self {
            Self::Class => usize::MAX,
            Self::TypeId(id) => {
                let bits = id.as_bits();
                debug_assert!(bits != usize::MAX);
                bits
            }
        }
    }
}
