use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::Type as JitType;

use crate::compiler::codegen::{ClassDef, CodegenData, StructDef, Type};
use crate::compiler::{FieldType, ObjectField, ObjectProperty, TypeLayout};

pub fn compute_class_layout(data: &CodegenData, class: &ClassDef) -> ClassLayout {
    let mut fields = HashMap::new();

    // The first word is the ClassInfo pointer.
    // If we have a parent class it already contains the
    // ClassInfo pointer.
    let mut offset = if let Some(parent) = &class.parent {
        let parent = data.classes.get(parent).unwrap();
        parent.layout.as_ref().unwrap().size
    } else {
        JitType::Pointer.size()
    };

    for (name, ty) in &class.fields {
        let ty = data.resolve_ty_jit(ty);

        if offset % ty.align() != 0 {
            offset += ty.align() - (offset % ty.align());
        }

        fields.insert(name.clone(), offset);
        offset += ty.size();
    }

    ClassLayout {
        field_offsets: fields,
        size: offset,
    }
}

#[derive(Clone, Debug)]
pub struct ClassLayout {
    pub field_offsets: HashMap<Lowercase<String>, usize>,
    /// Total size of the object.
    pub size: usize,
}

impl ClassLayout {
    pub fn offset(&self, field: &Lowercase<str>) -> Option<usize> {
        self.field_offsets.get(field).copied()
    }
}

pub fn compute_struct_layout(data: &CodegenData, st: &StructDef) -> ClassLayout {
    let mut fields = HashMap::new();

    let mut offset = 0;
    for member in &st.s.members {
        let ty = data.resolve_ty_jit(&Lowercase::new(member.ty.as_str()));

        if offset % ty.align() != 0 {
            offset += ty.align() - (offset % ty.align());
        }

        fields.insert(Lowercase::new(member.name.as_str()), offset);
        offset += ty.size();
    }

    ClassLayout {
        field_offsets: fields,
        size: offset,
    }
}

pub fn compute_struct_type_layout(data: &CodegenData, st: &StructDef) -> TypeLayout {
    let layout = compute_struct_layout(data, st);

    let mut fields = Vec::new();
    let mut properties = Vec::new();
    for (field_name, field_offset) in &layout.field_offsets {
        let field_ty_name =
            st.s.members
                .iter()
                .find(|v| &Lowercase::new(v.name.as_str()) == field_name)
                .map(|v| &v.ty)
                .unwrap();
        let field_ty_name = Lowercase::new(field_ty_name.as_str());

        let field_ty = data.resolve_ty(&field_ty_name);

        fields.push(ObjectField {
            class: Lowercase::new(st.s.name.as_str()),
            name: field_name.clone(),
            offset: *field_offset,
            ty: match field_ty {
                Type::Bool => FieldType::Bool,
                Type::Int => FieldType::I32,
                Type::Float => FieldType::F32,
                _ => FieldType::Pointer,
            },
            ty_name: field_ty_name,
            default_value: None,
        });

        properties.push(ObjectProperty {
            class: Lowercase::new(st.s.name.as_str()),
            name: field_name.clone(),
            var: 0,
        });
    }

    TypeLayout {
        header: None,
        fields,
        properties,
        classes: None,
    }
}

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
            class: Lowercase::new(""),
            name: Lowercase::new(""),
            offset: 0,
            ty: field_ty,
            ty_name: Lowercase::new(""),
            default_value: None,
        }],
        properties: vec![ObjectProperty {
            class: Lowercase::new(""),
            name: Lowercase::new(""),
            var: 0,
        }],
        classes: None,
    }
}
