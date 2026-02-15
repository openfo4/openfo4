use std::alloc::Layout;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use game_common::string::Lowercase;

use crate::compiler::{FieldType, ObjectField, ObjectProperty, TypeLayout};

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

        for field in &layout.fields {
            if let Some(default_value) = &field.default_value {
                assert!(default_value.len() <= field.ty.size());

                unsafe {
                    core::ptr::copy_nonoverlapping(
                        default_value.as_ptr(),
                        ptr.add(field.offset),
                        default_value.len(),
                    );
                }
            }
        }

        Self {
            memory: ptr,
            layout,
        }
    }

    pub unsafe fn get(&self, class: &str, name: &str) -> Option<&[u8]> {
        let prop = self.get_property_var(class, name);
        let field = &self.layout.fields[prop.var];

        let ptr = unsafe { self.memory.add(field.offset) };
        let len = field.ty.size();
        Some(unsafe { core::slice::from_raw_parts(ptr, len) })
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
        let prop = self.get_property_var(class, name);

        let field = &self.layout.fields[prop.var];

        let bytes = value.as_bytes();

        assert_eq!(
            bytes.len(),
            field.ty.size(),
            "{:?}.{:?} (type {:?}) requires {} bytes, but {} were provided",
            prop.class,
            prop.name,
            field.ty,
            field.ty.size(),
            bytes.len(),
        );

        unsafe {
            let dst = self.memory.add(field.offset);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        }
    }

    #[track_caller]
    fn get_property_var(&self, class: &str, name: &str) -> &ObjectProperty {
        let class = Lowercase::new(class);
        let name = Lowercase::new(name);

        // Always prefer properties that match the requested class.
        if let Some(prop) = self
            .layout
            .properties
            .iter()
            .find(|p| p.class == class && p.name == name)
        {
            return prop;
        }

        // We may also access fields of parent classes.
        // FIXME: Ideally we would recursively walk up the class
        // hierarchy instead of selecting any other matching property,
        // this requires us to have access to the class names from here.
        if let Some(prop) = self.layout.properties.iter().find(|p| p.name == name) {
            return prop;
        }

        dbg!(&self.layout);
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
        // FIXME: Bool is currently represented as 32-bit, hence
        // we need this hack.
        const FALSE: u32 = 0;
        const TRUE: u32 = 1;

        if *self {
            bytemuck::bytes_of(&TRUE)
        } else {
            bytemuck::bytes_of(&FALSE)
        }
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
    String = 5,
}

pub(crate) static ARRAY_LAYOUT: LazyLock<TypeLayout> = LazyLock::new(|| TypeLayout {
    header: None,
    fields: vec![
        ObjectField {
            class: Lowercase::new(""),
            name: Lowercase::new("ptr"),
            offset: 0,
            ty: FieldType::Pointer,
            ty_name: Lowercase::new(""),
            default_value: None,
        },
        ObjectField {
            class: Lowercase::new(""),
            name: Lowercase::new("len"),
            offset: size_of::<*mut u8>(),
            ty: FieldType::Isize,
            ty_name: Lowercase::new(""),
            default_value: None,
        },
    ],
    properties: vec![
        ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new("ptr"),
            var: 0,
        },
        ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new("len"),
            var: 1,
        },
    ],
    classes: None,
});

pub(crate) const BYTE_LAYOUT: LazyLock<TypeLayout> = LazyLock::new(|| TypeLayout {
    header: None,
    fields: vec![ObjectField {
        class: Lowercase::new(""),
        name: Lowercase::new(""),
        offset: 0,
        ty: FieldType::I8,
        ty_name: Lowercase::new(""),
        default_value: None,
    }],
    properties: vec![ObjectProperty {
        class: Lowercase::new(""),
        name: Lowercase::new(""),
        var: 0,
    }],
    classes: None,
});

pub(crate) const VAR_LAYOUT: LazyLock<TypeLayout> = LazyLock::new(|| TypeLayout {
    header: None,
    fields: vec![
        ObjectField {
            class: Lowercase::new(""),
            name: Lowercase::new("tag"),
            offset: 0,
            ty: FieldType::Isize,
            ty_name: Lowercase::new(""),
            default_value: None,
        },
        ObjectField {
            class: Lowercase::new(""),
            name: Lowercase::new("data"),
            offset: 0,
            ty: FieldType::Pointer,
            ty_name: Lowercase::new(""),
            default_value: None,
        },
    ],
    properties: vec![
        ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new("tag"),
            var: 0,
        },
        ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new("data"),
            var: 1,
        },
    ],
    classes: None,
});
