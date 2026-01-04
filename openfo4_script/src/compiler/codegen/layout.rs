use std::collections::HashMap;

use openfo4_jit::Type as JitType;

use crate::compiler::codegen::{ClassDef, CodegenData, StructDef};

pub fn compute_class_layout(data: &CodegenData, class: &ClassDef) -> ClassLayout {
    let mut fields = HashMap::new();

    // The first word is the ClassInfo pointer.
    // If we have a parent class it already contains the
    // ClassInfo pointer.
    let mut offset = if let Some(parent) = &class.parent {
        let parent = data.classes.get(&parent.to_lowercase()).unwrap();
        parent.layout.as_ref().unwrap().size
    } else {
        JitType::Pointer.size()
    };

    for (name, ty) in &class.fields {
        let ty = data.resolve_ty_jit(ty);

        if offset % ty.align() != 0 {
            offset += ty.align() - (offset % ty.align());
        }

        fields.insert(name.to_lowercase(), offset);
        offset += ty.size();
    }

    ClassLayout {
        field_offsets: fields,
        size: offset,
    }
}

#[derive(Clone, Debug)]
pub struct ClassLayout {
    pub field_offsets: HashMap<String, usize>,
    /// Total size of the object.
    pub size: usize,
}

impl ClassLayout {
    pub fn offset(&self, field: &str) -> Option<usize> {
        self.field_offsets.get(&field.to_lowercase()).copied()
    }
}

pub fn compute_struct_layout(data: &CodegenData, st: &StructDef) -> ClassLayout {
    let mut fields = HashMap::new();

    let mut offset = 0;
    for member in &st.s.members {
        let ty = data.resolve_ty_jit(&member.ty);

        if offset % ty.align() != 0 {
            offset += ty.align() - (offset % ty.align());
        }

        fields.insert(member.name.to_lowercase(), offset);
        offset += ty.size();
    }

    ClassLayout {
        field_offsets: fields,
        size: offset,
    }
}
