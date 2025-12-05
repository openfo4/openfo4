use crate::compiler::TypeLayout;

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
