use std::collections::HashMap;

use game_common::collections::arena::{Arena, Key};
use indexmap::IndexMap;

#[derive(Clone, Debug, Default)]
pub struct Types {
    types: Arena<Type>,
    name_to_type: HashMap<String, Key>,
}

impl Types {
    pub fn insert(&mut self, name: &str, desc: TypeDescriptor) {
        let key = match desc {
            TypeDescriptor::None => self.types.insert(Type {
                size: 4,
                align: 4,
                data: TypeData::None,
            }),
            TypeDescriptor::Bool => self.types.insert(Type {
                size: 4,
                align: 4,
                data: TypeData::Bool,
            }),
            TypeDescriptor::Int => self.types.insert(Type {
                size: 4,
                align: 4,
                data: TypeData::Int,
            }),
            TypeDescriptor::Float => self.types.insert(Type {
                size: 4,
                align: 4,
                data: TypeData::Float,
            }),
            TypeDescriptor::String => self.types.insert(Type {
                size: size_of::<*const ()>(),
                align: align_of::<*const ()>(),
                data: TypeData::String,
            }),
            TypeDescriptor::Struct { members } => {
                let mut st_members = IndexMap::new();

                let mut align = 1;
                let mut offset = 0;

                for (name, &ty) in &members {
                    let typ = &self.types.get(ty).unwrap();

                    // Round up offset to type alignment.
                    debug_assert!(typ.align.is_power_of_two());
                    dbg!(offset);
                    offset = (offset + (align - 1)) & !(align - 1);

                    st_members.insert(name.clone(), Member { ty, offset });

                    align = usize::max(align, typ.align);
                    offset += typ.size;
                }

                self.types.insert(Type {
                    size: offset,
                    align,
                    data: TypeData::Struct(StructType {
                        members: st_members,
                    }),
                })
            }
        };

        self.name_to_type.insert(name.to_lowercase(), key);
    }

    pub fn key(&self, name: &str) -> Option<Key> {
        self.name_to_type.get(&name.to_lowercase()).copied()
    }

    pub fn get(&self, name: &str) -> Option<&Type> {
        let key = self.name_to_type.get(&name.to_lowercase())?;
        self.types.get(*key)
    }
}

#[derive(Clone, Debug)]
pub enum TypeDescriptor {
    None,
    Bool,
    Int,
    Float,
    String,
    Struct { members: HashMap<String, Key> },
}

#[derive(Clone, Debug)]
pub struct Type {
    pub size: usize,
    pub align: usize,
    pub data: TypeData,
}

#[derive(Clone, Debug)]
pub enum TypeData {
    None,
    Bool,
    Int,
    Float,
    String,
    Struct(StructType),
}

#[derive(Clone, Debug)]
pub struct StructType {
    members: IndexMap<String, Member>,
}

#[derive(Copy, Clone, Debug)]
struct Member {
    /// The type of the member.
    ty: Key,
    /// The offset from the start of the compound type.
    offset: usize,
}
