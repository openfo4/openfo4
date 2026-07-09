use std::sync::Arc;

use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_pex::Value;

use crate::compiler::codegen::pex::registry::{ClassDef, Registry, StructDef, Type};
use crate::compiler::resources::{
    self, CompositeType, CompositeTypeMember, PropertyType, Resources, TypeId, TypeInfo,
    TypeProperty, TypeRepr,
};
use crate::runtime::ArrayObject;

pub fn compute_class_layout(
    res: &Resources,
    reg: &Registry,
    class: &ClassDef,
    id: TypeId,
    parent_ty: Option<TypeId>,
    parent_offsets: Option<HashMap<Lowercase<String>, usize>>,
) -> (resources::Type, HashMap<Lowercase<String>, usize>) {
    let mut offsets = parent_offsets.unwrap_or_else(|| HashMap::new());

    let mut properties = HashMap::new();
    let mut classes = Vec::new();

    let mut ty = if let Some(parent) = parent_ty {
        let parent = Arc::make_mut(&mut res.types.get(parent).unwrap()).clone();

        properties = parent.info.properties;
        classes = parent.info.classes;

        match parent.repr {
            TypeRepr::Composite(ty) => ty.clone(),
            _ => unreachable!(),
        }
    } else {
        // First member is for RTTI.
        CompositeType {
            members: vec![CompositeTypeMember {
                ty: res.primitives.pointer,
                offset: 0,
                default_value: None,
                size: size_of::<usize>(),
            }],
            align: align_of::<usize>(),
            size: size_of::<usize>(),
        }
    };

    let parent_field_count = ty.members.len();

    classes.push(id);

    for (field_name, field) in &class.fields {
        let member_size = field.ty.to_jit().size();
        let member_align = field.ty.to_jit().align();
        let (member_ty, mut default_value) = resolve_ty_and_default_value(res, &field.ty);

        match &field.value {
            Value::Bool(v) => {
                default_value = vec![u8::from(*v)];
            }
            Value::Int(v) => {
                default_value = v.to_ne_bytes().to_vec();
            }
            Value::Float(v) => {
                default_value = v.to_ne_bytes().to_vec();
            }
            Value::String(v) => {
                let ptr = res.constants.alloc(v.as_bytes(), 1);

                let string_ptr = res.constants.alloc_object(
                    ArrayObject {
                        ptr: ptr.cast_mut(),
                        len: v.len(),
                    }
                    .as_bytes(),
                    align_of::<ArrayObject>(),
                    res.primitives.string,
                );

                default_value = string_ptr.expose_provenance().to_ne_bytes().to_vec();
            }
            Value::Null | Value::Ident(_) => {}
        };

        ty.size = (ty.size + member_align - 1) & !(member_align - 1);

        ty.members.push(CompositeTypeMember {
            ty: member_ty,
            offset: ty.size,
            default_value: Some(default_value),
            size: member_size,
        });
        offsets.insert(field_name.clone(), ty.size);

        ty.size += member_size;
        ty.align = ty.align.max(member_align);
    }

    for (name, prop) in &class.properties {
        let Some(field) = &prop.field else {
            continue;
        };

        // This is sound assuming the field ordering of the hashmap never changes,
        // but this is given since a class can never be redefined and not changed
        // after being registered.
        let (index, (_, field)) = class
            .fields
            .iter()
            .enumerate()
            .find(|(_, (k, _))| *k == field)
            .unwrap();

        let offset = parent_field_count;
        let ty = resolve_property_type(res, reg, &field.ty);

        properties.insert(
            (class.name.clone(), name.clone()),
            TypeProperty {
                ty,
                field: offset + index,
            },
        );
    }

    (
        resources::Type {
            repr: TypeRepr::Composite(ty),
            info: TypeInfo {
                name: Some(class.name.clone()),
                classes,
                properties,
                allows_null: true,
            },
        },
        offsets,
    )
}

pub fn compute_struct_layout(
    res: &Resources,
    reg: &Registry,
    id: TypeId,
    st: &StructDef,
) -> (resources::Type, HashMap<Lowercase<String>, usize>) {
    let mut offset = 0;
    let mut align = 1;
    let mut members = Vec::new();
    let mut offsets = HashMap::new();
    let mut properties = HashMap::new();

    for (name, ty) in &st.fields {
        let member_size = ty.to_jit().size();
        let member_align = ty.to_jit().align();

        offset = (offset + member_align - 1) & !(member_align - 1);

        let (ty_id, default_value) = resolve_ty_and_default_value(res, &ty);

        let index = members.len();
        properties.insert(
            (st.name.clone(), name.clone()),
            TypeProperty {
                field: index,
                ty: resolve_property_type(res, reg, &ty),
            },
        );

        members.push(CompositeTypeMember {
            ty: ty_id,
            offset,
            default_value: Some(default_value),
            size: member_size,
        });
        offsets.insert(name.clone(), offset);

        offset += member_size;
        align = align.max(member_align);
    }

    (
        resources::Type {
            repr: TypeRepr::Composite(CompositeType {
                members,
                size: offset,
                align,
            }),
            info: TypeInfo {
                name: Some(st.name.clone()),
                classes: vec![id],
                properties,
                allows_null: true,
            },
        },
        offsets,
    )
}

fn resolve_ty_and_default_value(res: &Resources, ty: &Type) -> (TypeId, Vec<u8>) {
    match ty {
        Type::None => (res.primitives.none, vec![]),
        Type::Bool => (res.primitives.bool, vec![0; 1]),
        Type::Int => (res.primitives.int, vec![0; size_of::<i32>()]),
        Type::Float => (res.primitives.float, vec![0; size_of::<f32>()]),
        Type::String | Type::Struct(_) | Type::Class(_) | Type::Var | Type::Array(_) => {
            (res.primitives.pointer, vec![0; size_of::<usize>()])
        }
    }
}

fn resolve_property_type(res: &Resources, reg: &Registry, ty: &Type) -> PropertyType {
    match ty {
        Type::None => PropertyType::Inline(res.primitives.none),
        Type::Bool => PropertyType::Inline(res.primitives.bool),
        Type::Int => PropertyType::Inline(res.primitives.int),
        Type::Float => PropertyType::Inline(res.primitives.float),
        Type::String => PropertyType::Pointer(res.primitives.string),
        Type::Class(id) => {
            let class = reg.class(*id).unwrap();
            let s = res.string_to_type.lock();
            let id = s.get(&class.name).unwrap();
            PropertyType::Pointer(*id)
        }
        Type::Struct(id) => {
            let st = reg.struct_(*id).unwrap();
            let s = res.string_to_type.lock();
            let id = s.get(&st.name).unwrap();
            PropertyType::Pointer(*id)
        }
        Type::Array(ty) => {
            // FIXME: For now we only support one level of nesting.
            // The game currently never supports instantiating nested arrays,
            // however we should still be able to implement that.
            let id = match &**ty {
                Type::None => res.primitives.none,
                Type::Bool => res.primitives.bool,
                Type::Int => res.primitives.int,
                Type::Float => res.primitives.float,
                Type::String => res.primitives.string,
                Type::Class(id) => {
                    let class = reg.class(*id).unwrap();
                    let s = res.string_to_type.lock();
                    *s.get(&class.name).unwrap()
                }
                Type::Struct(id) => {
                    let st = reg.struct_(*id).unwrap();
                    let s = res.string_to_type.lock();
                    *s.get(&st.name).unwrap()
                }
                Type::Var => todo!(),
                Type::Array(_) => todo!(),
            };

            PropertyType::ArrayPointer(id)
        }
        Type::Var => todo!(),
    }
}
