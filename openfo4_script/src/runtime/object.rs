use std::alloc::Layout;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::compiler::TypeLayout;

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
        let ptr = unsafe { std::alloc::alloc_zeroed(alloc_layout) };
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

    // pub fn get(&self, addr: ObjectAddr) -> Option<&Object> {
    //     self.objects.get(&addr)
    // }

    // pub fn get_mut(&mut self, addr: ObjectAddr) -> Option<&mut Object> {
    //     self.objects.get_mut(&addr)
    // }
}

/// A reference handle to an [`Object`] in a pool.
#[derive(Clone, Debug)]
pub struct ObjectRef(Arc<ObjectRefInner>);

impl ObjectRef {
    pub fn addr(&self) -> ObjectAddr {
        self.0.addr
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
            return;
        };

        let bytes = value.as_bytes();
        assert_eq!(bytes.len(), field.ty.size());

        unsafe {
            let dst = self.memory.add(field.offset);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        }
    }

    pub fn as_ptr(&mut self) -> *mut u8 {
        self.memory
    }
}

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

    pub fn finalize(self) -> ObjectRef {
        self.obj_ref
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ObjectTag {
    Bool = 0,
    Int = 1,
    Float = 2,
    Class = 3,
}
