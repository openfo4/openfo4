use std::alloc::Layout;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use futures_lite::future::block_on;
use hashbrown::HashMap;
use openfo4_common::string::Lowercase;
use parking_lot::Mutex;
use thiserror::Error;

use crate::compiler::resources::{Resources, Type, TypeId, TypeRepr};
use crate::runtime::lock::ObjectLock;
use crate::runtime::{TypeRef, Val};

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

unsafe impl Send for ObjectAddr {}
unsafe impl Sync for ObjectAddr {}

#[derive(Clone, Debug)]
pub struct ObjectPool(Arc<ObjectPoolInner>);

#[derive(Debug)]
struct ObjectPoolInner {
    resources: Arc<Resources>,
    objects: Mutex<HashMap<ObjectAddr, Arc<Object>>>,
}

impl ObjectPool {
    pub(crate) fn new(resources: Arc<Resources>) -> Self {
        Self(Arc::new(ObjectPoolInner {
            resources,
            objects: Mutex::default(),
        }))
    }

    /// Allocates a new object with the given ty.
    pub(crate) fn alloc(&self, ty: TypeId, array_len: usize) -> ObjectHandle {
        let typ = self.0.resources.types.get(ty).unwrap();

        let align = typ.repr.align();
        let stride = typ.repr.size_padded();

        let array_size = stride * array_len;

        let alloc_layout = unsafe { Layout::from_size_align_unchecked(array_size, align) };

        // We must not allocate zero sized memory.
        // Any well-aligned pointer is valid for a zero sized allocation.
        let ptr = if alloc_layout.size() == 0 {
            alloc_layout.dangling_ptr().as_ptr()
        } else {
            unsafe { std::alloc::alloc_zeroed(alloc_layout) }
        };

        if ptr.is_null() {
            std::alloc::handle_alloc_error(alloc_layout);
        }

        let object = unsafe { Object::new(ptr, array_len, typ, ty, self.clone()) };
        let object = Arc::new(object);
        let addr = ObjectAddr(ptr);

        let mut objects = self.0.objects.lock();

        objects.insert(addr, Arc::clone(&object));

        ObjectHandle { object, index: 0 }
    }

    /// Creates a new object of the given type.
    pub fn insert(&self, mut ty: TypeId) -> ObjectHandle {
        let typ = self.0.resources.types.get(ty).unwrap();
        if let TypeRepr::Pointer(inner) = typ.repr {
            ty = inner;
        }

        self.alloc(ty, 1)
    }

    pub fn insert_string(&self, s: &str) -> ObjectHandle {
        let bytes = self.alloc(self.0.resources.primitives.byte, s.len());

        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), bytes.addr().0, s.len());
        }

        let string = self.alloc(self.0.resources.primitives.string, 1);
        let array = unsafe { string.object.ptr.cast::<ArrayObject>().as_mut_unchecked() };
        array.ptr = bytes.object.ptr;
        array.len = s.len();

        string
    }

    pub fn insert_array(&self, elem_ty: TypeId, array_len: usize) -> ObjectArrayBuilder {
        let elems = self.alloc(elem_ty, array_len);

        let array_ty = *self
            .0
            .resources
            .papyrus
            .type_to_array
            .lock()
            .get(&elem_ty)
            .unwrap();

        let array_obj = self.alloc(array_ty, 1);
        let array = unsafe {
            array_obj
                .object
                .ptr
                .cast::<ArrayObject>()
                .as_mut_unchecked()
        };
        array.ptr = elems.object.ptr;
        array.len = array_len;

        ObjectArrayBuilder {
            elems,
            array: array_obj,
        }
    }

    /// Returns an [`ObjectHandle`] to an existing object.
    pub fn get<T>(&self, object: T) -> Option<ObjectHandle>
    where
        T: AsObject,
    {
        let addr = object.addr();

        let objects = self.0.objects.lock();
        if let Some(object) = objects.get(&addr) {
            return Some(ObjectHandle {
                object: Arc::clone(object),
                index: 0,
            });
        }

        drop(objects);

        let object = self.0.resources.constants.object(addr)?;
        let ty = self.0.resources.types.get(object.ty).unwrap();
        let object = unsafe { Object::new_static(object.ptr, 1, ty, object.ty, self.clone()) };
        let object = Arc::new(object);
        Some(ObjectHandle { object, index: 0 })
    }

    unsafe fn array<U>(&self, addr: ObjectAddr) -> &[U] {
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

    pub unsafe fn string(&self, addr: ObjectAddr) -> &str {
        unsafe {
            let data = self.array::<u8>(addr);
            std::str::from_utf8_unchecked(data)
        }
    }

    pub(crate) fn resources(&self) -> &Resources {
        &self.0.resources
    }

    fn get_object(&self, addr: ObjectAddr) -> Option<(Arc<Object>, usize)> {
        let objects = self.0.objects.lock();
        let object = objects.get(&addr)?;
        Some((Arc::clone(object), 0))
    }
}

#[derive(Debug)]
struct Object {
    ptr: *mut u8,
    len: usize,
    ty: Arc<Type>,
    id: TypeId,
    pool: ObjectPool,
    is_static: bool,
}

impl Object {
    unsafe fn new(ptr: *mut u8, len: usize, ty: Arc<Type>, id: TypeId, pool: ObjectPool) -> Self {
        if let TypeRepr::Composite(ty) = &ty.repr {
            for member in &ty.members {
                let Some(value) = &member.default_value else {
                    continue;
                };

                debug_assert_eq!(value.len(), member.size);

                unsafe {
                    value.write(ptr.add(member.offset));
                }
            }
        }

        if let Some(offset) = ty.info.lock {
            let lock = ObjectLock::new();
            unsafe {
                ptr.add(offset).cast::<ObjectLock>().write(lock);
            }
        }

        Self {
            ptr,
            len,
            ty,
            id,
            pool,
            is_static: false,
        }
    }

    unsafe fn new_static(
        ptr: *const u8,
        len: usize,
        ty: Arc<Type>,
        id: TypeId,
        pool: ObjectPool,
    ) -> Self {
        Self {
            ptr: ptr.cast_mut(),
            len,
            ty,
            id,
            pool,
            is_static: true,
        }
    }

    /// Get the value at the given location.
    ///
    /// # Safety
    ///
    /// No other threads may access the same location of this [`Object`] for the duration of this
    /// function call, including guest code.
    unsafe fn get<'a, T>(&self, index: usize, class: &'a str, name: &'a str) -> Result<T, Error<'a>>
    where
        T: ObjectValue,
    {
        assert!(index < self.len);
        let array_offset = self.ty.repr.size_padded() * index;

        let Some((offset, _size, ty)) = self.get_prop_offset_and_size(class, name) else {
            return Err(Error::NoProperty { class, name });
        };

        unsafe {
            let ptr = self.ptr.add(array_offset).add(offset);
            T::read(&self.pool, ty, ptr).map_err(|()| Error::Incompatible)
        }
    }

    /// Set the value at the given location.
    ///
    /// # Safety
    ///
    /// No other threads may access the same location of this [`Object`] for the duration of this
    /// function call, including guest code.
    unsafe fn set<'a, T>(
        &self,
        index: usize,
        class: &'a str,
        name: &'a str,
        value: T,
    ) -> Result<(), Error<'a>>
    where
        T: ObjectValue,
    {
        assert!(index < self.len);
        let array_offset = self.ty.repr.size_padded() * index;

        if self.is_static {
            return Err(Error::Static);
        }

        let Some((offset, _size, ty)) = self.get_prop_offset_and_size(class, name) else {
            return Err(Error::NoProperty { class, name });
        };

        unsafe {
            let ptr = self.ptr.add(array_offset).add(offset);
            if !value.write(&self.pool, ty, ptr) {
                return Err(Error::Incompatible);
            }
        }

        Ok(())
    }

    fn get_prop_offset_and_size(&self, class: &str, name: &str) -> Option<(usize, usize, TypeId)> {
        let index = self.get_property_var(class, name)?;

        let TypeRepr::Composite(ty) = &self.ty.repr else {
            // Primitive type.
            return Some((0, self.ty.repr.size(), self.id));
        };

        let field = &ty.members[index];

        Some((field.offset, field.size, field.ty))
    }

    #[track_caller]
    fn get_property_var(&self, class: &str, name: &str) -> Option<usize> {
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
            return Some(prop.field);
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
            return Some(prop.field);
        }

        None
    }

    fn lock(&self) -> Option<&ObjectLock> {
        let offset = self.ty.info.lock?;
        Some(unsafe { self.ptr.add(offset).cast::<ObjectLock>().as_ref_unchecked() })
    }
}

// All values of objects are POD values or pointers that
// are threadsafe themselves.
unsafe impl Send for Object {}
unsafe impl Sync for Object {}

#[derive(Debug)]
pub struct ObjectArrayBuilder {
    elems: ObjectHandle,
    array: ObjectHandle,
}

impl ObjectArrayBuilder {
    #[track_caller]
    pub fn set_at<T>(&mut self, index: usize, class: &str, name: &str, value: T)
    where
        T: ObjectValue,
    {
        self.elems
            .at(index)
            .unwrap()
            .set(class, name, value)
            .unwrap();
    }

    pub fn finalize(self) -> ObjectHandle {
        self.array
    }
}

pub trait ObjectValue: Sized {
    #[doc(hidden)]
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()>;

    #[doc(hidden)]
    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool;
}

impl ObjectValue for bool {
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()> {
        let ty = pool.resources().types.get(ty).unwrap();
        match ty.repr {
            TypeRepr::Int { bits } if bits == 8 => Ok(unsafe { ptr.cast::<bool>().read() }),
            _ => Err(()),
        }
    }

    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool {
        if Val::Bool(*self).get_type_id(pool) != TypeRef::Concrete(ty) {
            return false;
        }

        unsafe {
            ptr.cast::<bool>().write(*self);
        }

        true
    }
}

impl ObjectValue for i32 {
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()> {
        let ty = pool.resources().types.get(ty).unwrap();
        match ty.repr {
            TypeRepr::Int { bits } if bits == 32 => Ok(unsafe { ptr.cast::<i32>().read() }),
            _ => Err(()),
        }
    }

    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool {
        if Val::I32(*self).get_type_id(pool) != TypeRef::Concrete(ty) {
            return false;
        }

        unsafe {
            ptr.cast::<i32>().write(*self);
        }

        true
    }
}

impl ObjectValue for f32 {
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()> {
        let ty = pool.resources().types.get(ty).unwrap();
        match ty.repr {
            TypeRepr::Float { bits } if bits == 32 => Ok(unsafe { ptr.cast::<f32>().read() }),
            _ => Err(()),
        }
    }

    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool {
        if Val::F32(*self).get_type_id(pool) != TypeRef::Concrete(ty) {
            return false;
        }

        unsafe {
            ptr.cast::<f32>().write(*self);
        }

        true
    }
}

impl ObjectValue for ObjectAddr {
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()> {
        let ty = pool.resources().types.get(ty).unwrap();
        // TODO: CHECK
        // match ty.repr {
        //     TypeRepr::Composite(_) => Ok(unsafe { ptr.cast::<ObjectAddr>().read() }),
        //     _ => Err(()),
        // }
        Ok(unsafe { ptr.cast::<ObjectAddr>().read() })
    }

    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool {
        let expect = pool.resources().types.get(ty).unwrap();
        let is = Val::Object(*self).get_type_id(pool);
        match is {
            TypeRef::Invalid => return false,
            TypeRef::Null if !expect.info.allows_null => return false,
            TypeRef::Null => (),
            TypeRef::Concrete(is) => {
                // TODO: CHECK
                // let is = pool.resources().types.get(is).unwrap();
                // if !is.info.is_subtype(&expect.info) {
                //     return false;
                // }
            }
        }

        unsafe {
            ptr.cast::<ObjectAddr>().write(*self);
        }

        true
    }
}

impl ObjectValue for Option<ObjectHandle> {
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()> {
        let addr = unsafe { ObjectAddr::read(pool, ty, ptr)? };
        if addr.is_null() {
            Ok(None)
        } else {
            let handle = pool
                .get_object(addr)
                .map(|(object, index)| ObjectHandle { object, index });
            Ok(handle)
        }
    }

    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool {
        let addr = self
            .as_ref()
            .map_or(ObjectAddr::NULL, |handle| handle.addr());
        unsafe { addr.write(pool, ty, ptr) }
    }
}

impl ObjectValue for Val {
    unsafe fn read(pool: &ObjectPool, ty: TypeId, ptr: *const u8) -> Result<Self, ()> {
        unsafe {
            match ty {
                ty if ty == pool.resources().primitives.bool => {
                    bool::read(pool, ty, ptr).map(Self::Bool)
                }
                ty if ty == pool.resources().primitives.int => {
                    i32::read(pool, ty, ptr).map(Self::I32)
                }
                ty if ty == pool.resources().primitives.float => {
                    f32::read(pool, ty, ptr).map(Self::F32)
                }
                _ => ObjectAddr::read(pool, ty, ptr).map(Self::Object),
            }
        }
    }

    unsafe fn write(&self, pool: &ObjectPool, ty: TypeId, ptr: *mut u8) -> bool {
        unsafe {
            match self {
                Self::Bool(v) => v.write(pool, ty, ptr),
                Self::I32(v) => v.write(pool, ty, ptr),
                Self::F32(v) => v.write(pool, ty, ptr),
                Self::Object(v) => v.write(pool, ty, ptr),
            }
        }
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

/// An owned handle to a [`Object`].
///
/// While at least on `ObjectHandle` exists the underlying [`Object`] will not be deallocated.
#[derive(Clone)]
pub struct ObjectHandle {
    object: Arc<Object>,
    index: usize,
}

impl ObjectHandle {
    pub fn addr(&self) -> ObjectAddr {
        ObjectAddr(self.object.ptr)
    }

    /// Returns the number of values in the array.
    ///
    /// If the object is not an array `1` is returned.
    pub fn len(&self) -> usize {
        self.object.len
    }

    pub fn at(&self, index: usize) -> Option<Self> {
        if index >= self.len() {
            None
        } else {
            Some(Self {
                object: Arc::clone(&self.object),
                index,
            })
        }
    }

    /// Gets and returns the value of the object at the given location.
    ///
    /// This function will block until the object can be locked.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The given location does not exist within the object.
    /// - The value at the location cannot be mapped to `T`.
    pub fn get<'a, T>(&self, class: &'a str, name: &'a str) -> Result<T, Error<'a>>
    where
        T: ObjectValue,
    {
        assert!(self.index < self.len());

        self.with_locked_blocking(|| {
            // SAFETY: We have locked the entire object.
            unsafe { self.object.get(self.index, class, name) }
        })
    }

    pub fn set<'a, T>(&self, class: &'a str, name: &'a str, value: T) -> Result<(), Error<'a>>
    where
        T: ObjectValue,
    {
        assert!(self.index < self.len());

        self.with_locked_blocking(|| {
            // SAFETY: We have locked the entire object.
            unsafe { self.object.set(self.index, class, name, value) }
        })
    }

    pub fn try_get<'a, T>(&self, class: &'a str, name: &'a str) -> Option<Result<T, Error<'a>>>
    where
        T: ObjectValue,
    {
        assert!(self.index < self.len());

        self.try_with_locked(|| {
            // SAFETY: We have locked the entire object.
            unsafe { self.object.get(self.index, class, name) }
        })
    }

    pub fn try_set<'a, T>(
        &self,
        class: &'a str,
        name: &'a str,
        value: T,
    ) -> Option<Result<(), Error<'a>>>
    where
        T: ObjectValue,
    {
        assert!(self.index < self.len());

        self.try_with_locked(|| {
            // SAFETY: We have locked the entire object.
            unsafe { self.object.set(self.index, class, name, value) }
        })
    }

    pub(crate) fn type_id(&self) -> TypeId {
        self.object.id
    }

    /// TODO: Remove soon
    pub fn as_ptr(&self) -> *mut u8 {
        self.object.ptr
    }

    /// TODO: Remove soon
    pub unsafe fn memory(&self) -> &[u8] {
        let size = self.object.ty.repr.size_padded();

        unsafe {
            let ptr = self.as_ptr().add(size * self.index);
            core::slice::from_raw_parts(ptr, size)
        }
    }

    /// TODO: Remove soon
    pub fn elem_size(&self) -> usize {
        self.object.ty.repr.size_padded()
    }

    fn with_locked_blocking<T, F>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.lock_blocking();

        let res = f();

        unsafe {
            self.unlock();
        }

        res
    }

    fn try_with_locked<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce() -> T,
    {
        if !self.try_lock() {
            return None;
        }

        let res = f();

        unsafe {
            self.unlock();
        }

        Some(res)
    }

    fn lock_blocking(&self) {
        if let Some(lock) = self.object.lock() {
            loop {
                match lock.lock() {
                    Ok(()) => break,
                    Err(listen) => block_on(listen),
                }
            }
        }
    }

    fn try_lock(&self) -> bool {
        if let Some(lock) = self.object.lock() {
            lock.try_lock()
        } else {
            true
        }
    }

    unsafe fn unlock(&self) {
        if let Some(lock) = self.object.lock() {
            unsafe {
                lock.unlock();
            }
        }
    }
}

impl Debug for ObjectHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectHandle({:p})", self.addr().0)
    }
}

/// A reference to a [`ObjectHandle`].
#[derive(Copy, Clone)]
pub struct ObjectHandleRef<'a> {
    pool: &'a ObjectPool,
    addr: ObjectAddr,
}

impl<'a> ObjectHandleRef<'a> {
    /// Creates an `ObjectHandleRef` from an address and the pool where it is stored.
    ///
    /// # Safety
    ///
    /// - `addr` must exist in the given `pool` and reference a valid object.
    /// - The referenced object must not be deallocated from the pool while this `ObjectHandleRef`
    ///   exists.
    pub(crate) unsafe fn new(pool: &'a ObjectPool, addr: ObjectAddr) -> Self {
        debug_assert!(!addr.is_null());
        debug_assert!(pool.get(addr).is_some());

        Self { pool, addr }
    }

    pub fn to_owned(&self) -> ObjectHandle {
        // Cannot fail since the constructor guarantees to not discard the existing
        // reference for our lifetime.
        self.pool.get(self.addr).unwrap()
    }
}

impl Debug for ObjectHandleRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectHandle({:p})", self.addr.0)
    }
}

pub trait AsObject {
    fn addr(&self) -> ObjectAddr;
}

impl AsObject for ObjectAddr {
    fn addr(&self) -> ObjectAddr {
        *self
    }
}

impl AsObject for ObjectHandle {
    fn addr(&self) -> ObjectAddr {
        self.addr()
    }
}

impl AsObject for ObjectHandleRef<'_> {
    fn addr(&self) -> ObjectAddr {
        self.addr
    }
}

#[derive(Clone, Debug, Error)]
pub enum Error<'a> {
    #[error("no property {class}.{name} exists")]
    NoProperty { class: &'a str, name: &'a str },
    #[error("object is static")]
    Static,
    #[error("types are incompatible")]
    Incompatible,
}
