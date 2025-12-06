use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::compiler::TypeLayout;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ObjectAddr(usize);

impl ObjectAddr {
    pub const fn from_raw(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn to_raw(self) -> usize {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct ObjectPool {
    objects: HashMap<ObjectAddr, Object>,
    dropped_refs: Arc<Mutex<Vec<ObjectAddr>>>,
}

impl ObjectPool {
    pub fn insert(&mut self, layout: TypeLayout) -> (&mut Object, ObjectRef) {
        let mut object = Object::new(layout);
        let addr = ObjectAddr(object.as_ptr().addr());

        self.objects.insert(addr, object);

        let obj_ref = ObjectRef(Arc::new(ObjectRefInner {
            addr,
            drop: self.dropped_refs.clone(),
        }));

        (self.objects.get_mut(&addr).unwrap(), obj_ref)
    }

    pub fn get(&self, addr: ObjectAddr) -> Option<&Object> {
        self.objects.get(&addr)
    }

    pub fn get_mut(&mut self, addr: ObjectAddr) -> Option<&mut Object> {
        self.objects.get_mut(&addr)
    }
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
    memory: Vec<u8>,
    layout: TypeLayout,
}

impl Object {
    pub fn new(layout: TypeLayout) -> Self {
        let mut memory = vec![0; layout.size()];

        if let Some(header) = &layout.header {
            let ptr = header.as_ptr().expose_provenance();
            memory[0..8].copy_from_slice(&ptr.to_ne_bytes());
        }

        Self { memory, layout }
    }

    pub fn set(&mut self, name: &str, value: i64) {
        let Some(field) = self
            .layout
            .fields
            .iter()
            .find(|f| f.name.to_lowercase() == name.to_lowercase())
        else {
            return;
        };

        let start = field.offset;
        let end = field.offset + field.ty.size();

        self.memory[start..end].copy_from_slice(&value.to_ne_bytes()[0..field.ty.size()]);
    }

    pub fn as_ptr(&mut self) -> *mut u8 {
        self.memory.as_mut_ptr()
    }
}
