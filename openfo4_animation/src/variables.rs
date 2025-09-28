use std::collections::HashMap;

use slab::Slab;

use crate::Value;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VariableId(usize);

#[derive(Clone, Debug, Default)]
pub struct VariableSet {
    variables: Slab<Value>,
    variables_by_name: HashMap<String, VariableId>,
    character_property_by_name: HashMap<String, VariableId>,
}

impl VariableSet {
    pub fn get(&self, id: VariableId) -> Option<&Value> {
        self.variables.get(id.0)
    }

    pub fn get_mut(&mut self, id: VariableId) -> Option<&mut Value> {
        self.variables.get_mut(id.0)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Value> {
        let index = *self.variables_by_name.get(name)?;
        Some(self.variables.get(index.0).unwrap())
    }

    pub fn get_mut_by_name(&mut self, name: &str) -> Option<&mut Value> {
        let index = *self.variables_by_name.get(name)?;
        Some(self.variables.get_mut(index.0).unwrap())
    }

    pub fn lookup_variable_name(&self, name: &str) -> Option<VariableId> {
        self.variables_by_name.get(name).copied()
    }

    pub fn lookup_character_property_name(&self, name: &str) -> Option<VariableId> {
        self.character_property_by_name.get(name).copied()
    }

    pub fn insert(&mut self, kind: VariableKind, name: String, value: Value) -> VariableId {
        let id = VariableId(self.variables.insert(value));

        match kind {
            VariableKind::Variable => {
                self.variables_by_name.insert(name, id);
            }
            VariableKind::CharacterProperty => {
                self.character_property_by_name.insert(name, id);
            }
        }

        id
    }
}

#[derive(Clone, Debug)]
pub enum VariableKind {
    Variable,
    CharacterProperty,
}
