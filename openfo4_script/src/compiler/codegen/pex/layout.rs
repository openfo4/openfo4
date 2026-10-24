use std::mem::offset_of;

use hashbrown::HashMap;
use openfo4_common::string::Lowercase;
use openfo4_pex::Value;

use crate::compiler::codegen::pex::registry::{
    ClassDef, ClassId, Registry, StructDef, StructId, Type,
};
use crate::compiler::resources::{
    self, CompositeType, CompositeTypeMember, Resources, TypeId, TypeInfo, TypeProperty, TypeRepr,
    Words, define_array_ty,
};
use crate::runtime::ArrayObject;
use crate::runtime::lock::ObjectLock;

/// Fixed-size header at the beginning of each class object.
#[repr(C)]
pub(crate) struct ClassHeader {
    pub meta: *const u8,
    pub lock: ObjectLock,
}

/// Fixed-size header at the beginning of each struct object.
#[repr(C)]
pub(crate) struct StructHeader {
    pub lock: ObjectLock,
}

#[derive(Clone, Debug)]
pub struct Layouts {
    classes: HashMap<ClassId, ClassLayout>,
    class_ptr_types: HashMap<ClassId, TypeId>,
    class_queue: Vec<ClassId>,
    structs: HashMap<StructId, ClassLayout>,
    struct_ptr_types: HashMap<StructId, TypeId>,
    struct_queue: Vec<StructId>,
    array_types: HashMap<Type, TypeId>,
    array_ptr_types: HashMap<Type, TypeId>,
}

impl Layouts {
    pub fn new(res: &Resources) -> Self {
        let mut array_types = HashMap::new();
        let mut array_ptr_types = HashMap::new();

        for (ty, id) in [
            (Type::None, res.primitives.none),
            (Type::Bool, res.primitives.bool),
            (Type::Int, res.primitives.int),
            (Type::Float, res.primitives.float),
            (Type::String, res.primitives.string),
            (Type::Var, res.primitives.var),
        ] {
            let array_ty = res.types.declare();
            let array_ptr_ty = res.types.declare();
            define_array_ty(res, id, array_ty, array_ptr_ty);
            array_types.insert(ty.clone(), array_ty);
            array_ptr_types.insert(ty, array_ptr_ty);

            res.papyrus
                .type_to_array_ptr
                .lock()
                .insert(id, array_ptr_ty);
            res.papyrus.arrays_to_elem.lock().insert(array_ty, id);
        }

        Self {
            classes: HashMap::new(),
            class_ptr_types: HashMap::new(),
            class_queue: Vec::new(),
            structs: HashMap::new(),
            struct_ptr_types: HashMap::new(),
            struct_queue: Vec::new(),
            array_types,
            array_ptr_types,
        }
    }

    pub fn class(&self, id: ClassId) -> &ClassLayout {
        self.classes.get(&id).unwrap()
    }

    pub fn struct_(&self, id: StructId) -> &ClassLayout {
        self.structs.get(&id).unwrap()
    }

    pub fn array(&self, array: &Type) -> TypeId {
        *self.array_types.get(array).unwrap()
    }

    pub fn compute_class(&mut self, res: &Resources, reg: &Registry, class: &ClassDef) {
        if self.classes.contains_key(&class.id) {
            return;
        }

        self.class_queue.push(class.id);
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
                // Compile the parent class before the sub class.
                let class = reg.class(id).unwrap();
                if let Some(parent) = class.parent
                    && !self.classes.contains_key(&parent)
                {
                    self.class_queue.push(id);
                    self.class_queue.push(parent);
                    continue;
                }

                self.compute_class_single(res, class);
            }

            while let Some(id) = self.struct_queue.pop() {
                let struct_ = reg.struct_(id).unwrap();
                self.compute_struct_single(res, struct_);
            }
        }
    }

    fn compute_class_single(&mut self, res: &Resources, class: &ClassDef) {
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
            let (ty, _) = self.resolve_ty_and_default_value(res, &field.1.ty);

            properties.insert(
                (class.name.clone(), name.clone()),
                TypeProperty { field: *index, ty },
            );
        }

        debug_assert!(!self.classes.contains_key(&class.id));
        let id = res.types.declare();

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
                    lock: Some(offset_of!(ClassHeader, lock)),
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
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: ptr_ty,
                            field: 0,
                        },
                    )]),
                    allows_null: true,
                    lock: None,
                },
            },
        );

        let array_ty = *self
            .array_types
            .entry(Type::Class(class.id))
            .or_insert_with(|| res.types.declare());
        let array_ptr_ty = *self
            .array_ptr_types
            .entry(Type::Class(class.id))
            .or_insert_with(|| res.types.declare());
        define_array_ty(res, id, array_ty, array_ptr_ty);
        // TODO: Only register the ptr_ty here when we move over to only using
        // pointer types for HIR codegen.
        res.papyrus
            .type_to_array_ptr
            .lock()
            .insert(id, array_ptr_ty);
        res.papyrus
            .type_to_array_ptr
            .lock()
            .insert(ptr_ty, array_ptr_ty);
        res.papyrus.arrays_to_elem.lock().insert(array_ty, ptr_ty);

        self.classes.insert(
            class.id,
            ClassLayout {
                ty: id,
                ptr_ty,
                offsets,
            },
        );
    }

    fn compute_struct_single(&mut self, res: &Resources, st: &StructDef) {
        if self.structs.contains_key(&st.id) {
            return;
        }

        let mut offset = size_of::<StructHeader>();
        let mut align = align_of::<StructHeader>();
        let mut members = vec![CompositeTypeMember {
            ty: res.primitives.opaqueptr,
            offset: offset_of!(StructHeader, lock),
            size: size_of::<StructHeader>(),
            default_value: None,
        }];
        let mut offsets = HashMap::new();
        let mut properties = HashMap::new();

        for (name, ty) in &st.fields {
            let member_size = ty.to_jit().size();
            let member_align = ty.to_jit().align();

            offset = (offset + member_align - 1) & !(member_align - 1);

            let (ty_id, default_value) = self.resolve_ty_and_default_value(res, &ty);

            let index = members.len();
            properties.insert(
                (st.name.clone(), name.clone()),
                TypeProperty {
                    field: index,
                    ty: ty_id,
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

        debug_assert!(!self.structs.contains_key(&st.id));
        let id = res.types.declare();

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
                    lock: Some(offset_of!(StructHeader, lock)),
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
                    properties: HashMap::from([(
                        (Lowercase::new(""), Lowercase::new("")),
                        TypeProperty {
                            ty: ptr_ty,
                            field: 0,
                        },
                    )]),
                    allows_null: true,
                    lock: None,
                },
            },
        );

        let array_ty = *self
            .array_types
            .entry(Type::Struct(st.id))
            .or_insert_with(|| res.types.declare());
        let array_ptr_ty = *self
            .array_ptr_types
            .entry(Type::Struct(st.id))
            .or_insert_with(|| res.types.declare());
        define_array_ty(res, id, array_ty, array_ptr_ty);
        // TODO: Only register the ptr_ty here when we move over to only using
        // pointer types for HIR codegen.
        res.papyrus
            .type_to_array_ptr
            .lock()
            .insert(id, array_ptr_ty);
        res.papyrus
            .type_to_array_ptr
            .lock()
            .insert(ptr_ty, array_ptr_ty);
        res.papyrus.arrays_to_elem.lock().insert(array_ty, ptr_ty);

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
            Type::String | Type::Var => (res.primitives.opaqueptr, Words::nullptr()),
            Type::Class(id) => {
                let ty = self.class_ptr_types.entry(*id).or_insert_with(|| {
                    let ty = res.types.declare();
                    self.class_queue.push(*id);
                    ty
                });

                (*ty, Words::nullptr())
            }
            Type::Struct(id) => {
                let ty = self.struct_ptr_types.entry(*id).or_insert_with(|| {
                    let ty = res.types.declare();
                    self.struct_queue.push(*id);
                    ty
                });

                (*ty, Words::nullptr())
            }
            Type::Array(ty) => {
                let ty = self
                    .array_ptr_types
                    .entry((**ty).clone())
                    .or_insert_with(|| {
                        match &**ty {
                            Type::Class(id) => self.class_queue.push(*id),
                            Type::Struct(id) => self.struct_queue.push(*id),
                            _ => {}
                        }

                        res.types.declare()
                    });

                (*ty, Words::nullptr())
            }
        }
    }
}

fn build_class_header(res: &Resources) -> resources::Type {
    resources::Type {
        repr: TypeRepr::Composite(CompositeType {
            members: vec![
                // meta
                CompositeTypeMember {
                    ty: res.primitives.opaqueptr,
                    offset: offset_of!(ClassHeader, meta),
                    default_value: None,
                    size: size_of::<*const ()>(),
                },
                // lock
                CompositeTypeMember {
                    ty: res.primitives.opaqueptr,
                    offset: offset_of!(ClassHeader, lock),
                    default_value: None,
                    size: size_of::<ObjectLock>(),
                },
            ],
            align: align_of::<ClassHeader>(),
            size: size_of::<ClassHeader>(),
        }),
        info: TypeInfo::default(),
    }
}

#[derive(Clone, Debug)]
pub struct ClassLayout {
    pub ty: TypeId,
    pub ptr_ty: TypeId,
    pub offsets: HashMap<Lowercase<String>, usize>,
}
