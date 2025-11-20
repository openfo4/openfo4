mod compiler;

use std::collections::{HashMap, HashSet};

use game_common::collections::arena::{Arena, Key};
use openfo4_pex::Class;

use crate::compiler::Context;

#[derive(Clone, Debug)]
pub enum Error {
    UndeclaredType(String),
    DefaultStateMissing(String),
    UndeclaredMethod { class: String, function: String },
}

pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug)]
pub struct ClassId(pub String);

#[derive(Debug)]
pub struct Registry {
    classes: HashMap<String, Class>,
    native_functions: Vec<(String, String, *const u8)>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            native_functions: Vec::new(),
        }
    }

    pub fn register(&mut self, module: &openfo4_pex::Module) -> Result<Vec<ClassId>, Error> {
        let mut classes = Vec::new();

        for class in &module.classes {
            self.classes
                .insert(class.name.to_lowercase(), class.clone());

            classes.push(ClassId(class.name.to_lowercase()));
        }

        Ok(classes)
    }

    pub fn define_native_function(&mut self, class: &str, func: &str, ptr: *const u8) {
        self.native_functions
            .push((class.to_lowercase(), func.to_lowercase(), ptr));
    }

    pub fn compile(&self, class_id: ClassId) -> Result<(), Error> {
        let mut ctx = Context::new();

        for name in self.classes.keys() {
            ctx.declare_class(&name);
        }

        for class in self.classes.values() {
            if !class.parent.is_empty() {
                ctx.declare_parent_class(&class.name, &class.parent);
            }
        }

        for (name, class) in &self.classes {
            for var in &class.variables {
                ctx.declare_class_field(&name, var);
            }

            for state in &class.states {
                for func in &state.functions {
                    ctx.declare_class_function(&name, &func);
                }
            }
        }

        for (class, func, ptr) in &self.native_functions {
            ctx.define_native_function(&class, &func, *ptr);
        }

        let class = self.classes.get(&class_id.0).unwrap();

        for s in &class.states {
            for f in &s.functions {
                ctx.compile_fn(&class_id.0, &f).unwrap();
            }
        }

        todo!()
    }
}
