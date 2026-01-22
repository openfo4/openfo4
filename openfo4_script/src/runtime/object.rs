use std::alloc::Layout;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::compiler::{FieldType, ObjectField, TypeLayout};

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
}

impl AsRef<ObjectAddr> for ObjectAddr {
    fn as_ref(&self) -> &ObjectAddr {
        self
    }
}

unsafe impl Send for ObjectAddr {}
unsafe impl Sync for ObjectAddr {}

#[derive(Debug, Default)]
pub struct ObjectPool {
    objects: Mutex<HashMap<ObjectAddr, Object>>,
    dropped_refs: Arc<Mutex<Vec<ObjectAddr>>>,
}

impl ObjectPool {
    pub fn alloc(&self, layout: TypeLayout, array_len: usize) -> ObjectRef {
        let size = layout.size();
        let align = layout.align();
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

        assert!(!ptr.is_null());

        let mut objects = self.objects.lock().unwrap();

        for i in 0..array_len {
            let elem_ptr = unsafe { ptr.add(stride * i) };
            let addr = ObjectAddr::from_raw(elem_ptr);

            let object = unsafe { Object::new_at(layout.clone(), elem_ptr) };

            objects.insert(addr, object);
        }

        ObjectRef(Arc::new(ObjectRefInner {
            addr: ObjectAddr(ptr),
            drop: self.dropped_refs.clone(),
        }))
    }

    pub fn insert(&self, layout: TypeLayout) -> ObjectBuilder {
        let object_ref = self.alloc(layout.clone(), 1);

        let object = unsafe { Object::new_at(layout, object_ref.addr().0 as *mut u8) };

        ObjectBuilder {
            obj: object,
            obj_ref: object_ref,
        }
    }

    pub fn insert_string(&self, s: &str) -> ObjectRef {
        let bytes_ref = self.alloc(BYTE_LAYOUT.clone(), s.len());

        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), bytes_ref.as_ptr(), s.len());
        }

        let mut string_object = self.insert(ARRAY_LAYOUT.clone());
        string_object.set("", "ptr", bytes_ref.addr());
        string_object.set("", "len", s.len());

        string_object.finalize()
    }

    pub fn insert_array(&self, elem_layout: TypeLayout, array_len: usize) -> ObjectArrayBuilder {
        let array_ref = self.alloc(elem_layout.clone(), array_len);

        let mut objects = Vec::new();

        let size = elem_layout.size();
        let align = elem_layout.align();
        let stride = (size + align - 1) & !(align - 1);

        for i in 0..array_len {
            let elem_ptr = unsafe { array_ref.as_ptr().add(stride * i) };

            let object = unsafe { Object::new_at(elem_layout.clone(), elem_ptr) };
            objects.push(object);
        }

        let mut array_object = self.insert(ARRAY_LAYOUT.clone());
        array_object.set("", "ptr", array_ref.addr());
        array_object.set("", "len", array_len);

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
        let layout = {
            let objects = self.objects.lock().unwrap();

            let object = objects.get(&object.addr()).unwrap();
            object.layout.clone()
        };

        let builder = self.insert(layout);

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
    layout: TypeLayout,
}

impl Object {
    unsafe fn new_at(layout: TypeLayout, ptr: *mut u8) -> Self {
        if let Some(header) = &layout.header {
            let header_ptr: *const usize = header.as_ptr();

            // SAFETY:
            // - The caller guarantees that `ptr` is valid.
            // - The caller must align this pointer to `align_of<*const T>`
            //   if layout.header is `Some(..)`.
            unsafe {
                ptr.cast::<*const usize>().write(header_ptr);
            }
        }

        Self {
            memory: ptr,
            layout,
        }
    }

    pub unsafe fn get(&self, class: &str, name: &str) -> Option<&[u8]> {
        let field = self.layout.fields.iter().find(|f| {
            f.class.to_lowercase() == class.to_lowercase()
                && f.name.to_lowercase() == name.to_lowercase()
        })?;

        let ptr = unsafe { self.memory.add(field.offset) };
        let len = field.ty.size();
        Some(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    /// # Safety
    ///
    /// The caller must guarantee that no memory accesses to the field of the `Object` are
    /// currently active.
    pub unsafe fn set<T>(&mut self, class: &str, name: &str, value: T)
    where
        T: IntoValue,
    {
        let Some(field) = self.layout.fields.iter().find(|f| {
            f.class.to_lowercase() == class.to_lowercase()
                && f.name.to_lowercase() == name.to_lowercase()
        }) else {
            panic!("no field {:?}.{:?}", class, name);
        };

        let bytes = value.as_bytes();
        assert_eq!(bytes.len(), field.ty.size());

        unsafe {
            let dst = self.memory.add(field.offset);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        }
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
        self.layout.size()
    }

    pub fn layout(&self) -> &TypeLayout {
        &self.layout
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
}

#[derive(Debug)]
pub struct ObjectArrayBuilder {
    objects: Vec<Object>,
    object_ref: ObjectRef,
}

impl ObjectArrayBuilder {
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
pub struct StringObject {
    pub ptr: *mut u8,
    pub len: usize,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ArrayObject {
    pub ptr: *mut u8,
    pub len: usize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct VarObject {
    pub tag: ObjectTag,
    pub data: VarObjectData,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union VarObjectData {
    pub bool: bool,
    pub int: i32,
    pub float: f32,
    pub class: ObjectAddr,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ObjectTag {
    None = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    Class = 4,
}

pub(crate) static ARRAY_LAYOUT: LazyLock<TypeLayout> = LazyLock::new(|| TypeLayout {
    header: None,
    fields: vec![
        ObjectField {
            class: "".to_owned(),
            name: "ptr".to_owned(),
            offset: 0,
            ty: FieldType::Pointer,
            ty_name: "".to_owned(),
        },
        ObjectField {
            class: "".to_owned(),
            name: "len".to_owned(),
            offset: size_of::<*mut u8>(),
            ty: FieldType::Isize,
            ty_name: "".to_owned(),
        },
    ],
    properties: Vec::new(),
});

pub(crate) const BYTE_LAYOUT: LazyLock<TypeLayout> = LazyLock::new(|| TypeLayout {
    header: None,
    fields: vec![ObjectField {
        class: "".to_owned(),
        name: "".to_owned(),
        offset: 0,
        ty: FieldType::I8,
        ty_name: "".to_owned(),
    }],
    properties: Vec::new(),
});

pub(crate) const VAR_LAYOUT: LazyLock<TypeLayout> = LazyLock::new(|| TypeLayout {
    header: None,
    fields: vec![
        ObjectField {
            class: "".to_owned(),
            name: "tag".to_owned(),
            offset: 0,
            ty: FieldType::Isize,
            ty_name: "".to_owned(),
        },
        ObjectField {
            class: "".to_owned(),
            name: "data".to_owned(),
            offset: 0,
            ty: FieldType::Pointer,
            ty_name: "".to_owned(),
        },
    ],
    properties: Vec::new(),
});
