use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_pex::Value;

use crate::compiler::codegen::pex::registry::{
    ClassDef, ClassId, Registry, StructDef, StructId, Type,
};
use crate::compiler::resources::{
    self, CompositeType, CompositeTypeMember, PropertyType, Resources, TypeId, TypeInfo,
    TypeProperty, TypeRepr, Words,
};
use crate::runtime::ArrayObject;

#[derive(Clone, Debug, Default)]
pub struct Layouts {
    classes: HashMap<ClassId, ClassLayout>,
    class_ptr_types: HashMap<ClassId, TypeId>,
    class_queue: Vec<ClassId>,
    structs: HashMap<StructId, ClassLayout>,
    struct_ptr_types: HashMap<StructId, TypeId>,
    struct_queue: Vec<StructId>,
}

impl Layouts {
    pub fn class(&self, id: ClassId) -> &ClassLayout {
        self.classes.get(&id).unwrap()
    }

    pub fn struct_(&self, id: StructId) -> &ClassLayout {
        self.structs.get(&id).unwrap()
    }

    pub fn compute_class(&mut self, res: &Resources, reg: &Registry, class: &ClassDef) {
        if self.classes.contains_key(&class.id) {
            return;
        }

        self.class_queue.push(class.id);

        let mut class = class;
        while let Some(parent) = class.parent
            && !self.classes.contains_key(&parent)
        {
            self.class_queue.push(parent);
            class = reg.class(parent).unwrap();
        }

        self.process_queues(res, reg);
    }

    pub fn compute_struct(&mut self, res: &Resources, reg: &Registry, st: &StructDef) {
        if self.structs.contains_key(&st.id) {
            return;
        }

        self.struct_queue.push(st.id);
        self.process_queues(res, reg);
    }

    fn process_queues(&mut self, res: &Resources, reg: &Registry) {
        while !self.class_queue.is_empty() || !self.struct_queue.is_empty() {
            while let Some(id) = self.class_queue.pop() {
                let class = reg.class(id).unwrap();
                self.compute_class_single(res, reg, class);
            }

            while let Some(id) = self.struct_queue.pop() {
                let struct_ = reg.struct_(id).unwrap();
                self.compute_struct_single(res, reg, struct_);
            }
        }
    }

    fn compute_class_single(&mut self, res: &Resources, reg: &Registry, class: &ClassDef) {
        if self.classes.contains_key(&class.id) {
            return;
        }

        let (ty, mut offsets) = match class.parent {
            Some(parent) => {
                let parent = self.classes.get(&parent).unwrap();
                (
                    (*res.types.get(parent.ty).unwrap()).clone(),
                    parent.offsets.clone(),
                )
            }
            None => (build_class_header(res), HashMap::new()),
        };

        let mut properties = ty.info.properties.clone();
        let mut classes = ty.info.classes.clone();
        let TypeRepr::Composite(mut ty) = ty.repr else {
            unreachable!()
        };

        // Sort class fields by alignment descending to minimize padding
        // between fields.
        let mut fields: Vec<_> = class.fields.iter().collect();
        fields.sort_by_key(|(_, field)| field.ty.to_jit().align());
        fields.reverse();

        let base_index = ty.members.len();
        let mut indices = HashMap::new();
        for (index, (name, field)) in fields.iter().enumerate() {
            let size = field.ty.to_jit().size();
            let align = field.ty.to_jit().align();
            let (member_ty, mut default_value) = self.resolve_ty_and_default_value(res, &field.ty);

            match &field.value {
                Value::Bool(v) => {
                    default_value = Words::bytes(&[u8::from(*v)]);
                }
                Value::Int(v) => {
                    default_value = Words::bytes(&v.to_ne_bytes());
                }
                Value::Float(v) => {
                    default_value = Words::bytes(&v.to_ne_bytes());
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

                    default_value = Words::ptr(string_ptr.cast_mut().cast());
                }
                Value::Null | Value::Ident(_) => {}
            };

            ty.size = (ty.size + align - 1) & !(align - 1);

            ty.members.push(CompositeTypeMember {
                ty: member_ty,
                offset: ty.size,
                default_value: Some(default_value),
                size,
            });
            offsets.insert((*name).clone(), ty.size);
            indices.insert(name, base_index + index);

            ty.size += size;
            ty.align = ty.align.max(align);
        }

        for (name, prop) in &class.properties {
            let Some(field) = &prop.field else {
                continue;
            };

            let index = indices.get(&field).unwrap();
            let field = &fields[*index - base_index];
            let ty = resolve_property_type(res, reg, &field.1.ty);

            properties.insert(
                (class.name.clone(), name.clone()),
                TypeProperty { field: *index, ty },
            );
        }

        let id = *res
            .string_to_type
            .lock()
            .entry(class.name.clone())
            .or_insert_with(|| res.types.declare());

        classes.push(id);

        res.types.define(
            id,
            resources::Type {
                repr: TypeRepr::Composite(ty),
                info: TypeInfo {
                    name: Some(class.name.clone()),
                    classes,
                    properties,
                    allows_null: true,
                },
            },
        );

        let ptr_ty = *self
            .class_ptr_types
            .entry(class.id)
            .or_insert_with(|| res.types.declare());
        res.types.define(
            ptr_ty,
            resources::Type {
                repr: TypeRepr::Pointer(id),
                info: TypeInfo {
                    name: None,
                    classes: Vec::new(),
                    properties: HashMap::new(),
                    allows_null: true,
                },
            },
        );

        self.classes.insert(
            class.id,
            ClassLayout {
                ty: id,
                ptr_ty,
                offsets,
            },
        );
    }

    fn compute_struct_single(&mut self, res: &Resources, reg: &Registry, st: &StructDef) {
        if self.structs.contains_key(&st.id) {
            return;
        }

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

        let id = *res
            .string_to_type
            .lock()
            .entry(st.name.clone())
            .or_insert_with(|| res.types.declare());

        res.types.define(
            id,
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
        );

        let ptr_ty = *self
            .struct_ptr_types
            .entry(st.id)
            .or_insert_with(|| res.types.declare());
        res.types.define(
            ptr_ty,
            resources::Type {
                repr: TypeRepr::Pointer(id),
                info: TypeInfo {
                    name: None,
                    classes: Vec::new(),
                    properties: HashMap::new(),
                    allows_null: true,
                },
            },
        );

        self.structs.insert(
            st.id,
            ClassLayout {
                ty: id,
                ptr_ty,
                offsets,
            },
        );
    }

    fn resolve_ty_and_default_value(&mut self, res: &Resources, ty: &Type) -> (TypeId, Words) {
        match ty {
            Type::None => (res.primitives.none, Words::bytes(&[])),
            Type::Bool => (res.primitives.bool, Words::bytes(&[0; 1])),
            Type::Int => (res.primitives.int, Words::bytes(&[0; size_of::<i32>()])),
            Type::Float => (res.primitives.float, Words::bytes(&[0; size_of::<f32>()])),
            Type::String | Type::Struct(_) | Type::Var | Type::Array(_) => (
                res.primitives.pointer,
                Words::bytes(&[0; size_of::<usize>()]),
            ),
            Type::Class(id) => {
                let ty = self.class_ptr_types.entry(*id).or_insert_with(|| {
                    let ty = res.types.declare();
                    self.class_queue.push(*id);
                    ty
                });

                (
                    res.primitives.pointer,
                    Words::bytes(&[0; size_of::<*const ()>()]),
                )
            }
        }
    }
}

fn build_class_header(res: &Resources) -> resources::Type {
    resources::Type {
        repr: TypeRepr::Composite(
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
            },
        ),
        info: TypeInfo::default(),
    }
}

#[derive(Clone, Debug)]
pub struct ClassLayout {
    pub ty: TypeId,
    pub ptr_ty: TypeId,
    pub offsets: HashMap<Lowercase<String>, usize>,
}

fn resolve_ty_and_default_value(res: &Resources, ty: &Type) -> (TypeId, Words) {
    match ty {
        Type::None => (res.primitives.none, Words::bytes(&[])),
        Type::Bool => (res.primitives.bool, Words::bytes(&[0; 1])),
        Type::Int => (res.primitives.int, Words::bytes(&[0; size_of::<i32>()])),
        Type::Float => (res.primitives.float, Words::bytes(&[0; size_of::<f32>()])),
        Type::String | Type::Struct(_) | Type::Class(_) | Type::Var | Type::Array(_) => (
            res.primitives.pointer,
            Words::bytes(&[0; size_of::<usize>()]),
        ),
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
            let id = *res
                .string_to_type
                .lock()
                .entry(class.name.clone())
                .or_insert_with(|| res.types.declare());
            PropertyType::Pointer(id)
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
                    let id = *res
                        .string_to_type
                        .lock()
                        .entry(class.name.clone())
                        .or_insert_with(|| res.types.declare());
                    id
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
