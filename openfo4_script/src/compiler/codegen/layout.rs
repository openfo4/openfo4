use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

use openfo4_jit::Type as JitType;

use crate::classes;
use crate::compiler::codegen::{ClassDef, CodegenData, StructDef, Type};
use crate::compiler::{FieldType, ObjectField, TypeLayout};

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

pub static ARRAY_LAYOUT: LazyLock<Box<TypeLayout>> = LazyLock::new(|| {
    Box::new(TypeLayout {
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
                ty: FieldType::Pointer,
                ty_name: "".to_owned(),
            },
        ],
        properties: Vec::new(),
    })
});

pub fn generate_type_layout(data: &CodegenData, ty: &Type) -> TypeLayout {
    let field_ty = match ty {
        Type::None => FieldType::I32,
        Type::Bool => FieldType::Bool,
        Type::Int => FieldType::I32,
        Type::Float => FieldType::F32,
        Type::Class(_) => FieldType::Pointer,
        Type::String => FieldType::Pointer,
        Type::Struct(_) => FieldType::Pointer,
        Type::Var => FieldType::Pointer,
        Type::Array(_) => FieldType::Pointer,
    };

    TypeLayout {
        header: None,
        fields: vec![ObjectField {
            class: "".to_owned(),
            name: "".to_owned(),
            offset: 0,
            ty: field_ty,
            ty_name: "".to_owned(),
        }],
        properties: Vec::new(),
    }
}
