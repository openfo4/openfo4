use std::collections::HashMap;

use crate::compiler::codegen::ClassDef;

#[derive(Clone, Debug)]
pub struct ClassLayout {
    variable_offsets: HashMap<String, usize>,
}

impl ClassLayout {
    fn get_offset(&self, class: &str, variable: &str) {}
}
