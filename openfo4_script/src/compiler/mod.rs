mod codegen;
mod validation;

use std::sync::Arc;

use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{CompiledFunction, Signature};
use openfo4_pex::Module;
use thiserror::Error;

use crate::compiler::codegen::CodegenPass;
use crate::compiler::validation::ValidationPass;
use crate::engine::Options;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Validation(validation::Error),
    #[error(transparent)]
    Codegen(codegen::Error),
}

pub struct Context {
    validation: ValidationPass,
    codegen: CodegenPass,
}

impl Context {
    pub fn new() -> Self {
        Self {
            validation: ValidationPass::default(),
            codegen: CodegenPass::new(),
        }
    }

    pub fn register_module(&mut self, module: &Module) -> Result<(), Error> {
        self.validation.register_module(module);
        self.codegen.register_module(module);
        Ok(())
    }

    pub fn define_native_function(&mut self, class: &str, func: &str, f: NativeFunction) {
        self.codegen.define_native_function(class, func, f);
    }

    pub fn compute_class_layouts(
        &mut self,
        class: &str,
    ) -> Result<HashMap<Lowercase<String>, TypeLayout>, Error> {
        self.codegen
            .compute_class_layouts(class)
            .map_err(Error::Codegen)
    }

    pub fn compile(
        &mut self,
        class: &str,
        func: &str,
        options: &Options,
    ) -> Result<CompilationResult, Error> {
        self.validation.validate().map_err(Error::Validation)?;
        let res = self
            .codegen
            .compile_fn(class, func, options)
            .map_err(Error::Codegen)?;
        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct NativeFunction {
    pub ptr: *const u8,
    pub index: usize,
    pub signature: Signature,
}

// SAFETY: We carry a function pointer which is always read-only
// and can be called from any thread.
unsafe impl Send for NativeFunction {}
unsafe impl Sync for NativeFunction {}

#[derive(Clone, Debug)]
pub struct CompilationResult {
    pub function: CompiledFunction,
    pub stack_size: usize,
    pub signature: Signature,
}

#[derive(Clone, Debug)]
pub struct TypeLayout {
    pub header: Option<Arc<[usize]>>,
    pub fields: Vec<ObjectField>,
    pub properties: Vec<ObjectProperty>,
    /// The class names of the hierarchy of this type.
    ///
    /// Same order as `header`, index 0 is the "root" class (`ScriptObject`)
    /// and the last index is the most concrete class.
    pub classes: Option<Arc<[String]>>,
}

impl TypeLayout {
    /// Returns the size in bytes required for the object.
    pub fn size(&self) -> usize {
        let mut size = if self.header.is_some() {
            FieldType::Pointer.size()
        } else {
            0
        };

        for field in &self.fields {
            size = size.max(field.offset + field.ty.size());
        }

        size
    }

    pub fn align(&self) -> usize {
        let mut align = if self.header.is_some() {
            FieldType::Pointer.size()
        } else {
            1
        };

        for field in &self.fields {
            align = align.max(field.ty.align());
        }

        align
    }

    pub fn size_padded(&self) -> usize {
        let size = self.size();
        let align = self.align();
        (size + align - 1) & !(align - 1)
    }
}

#[derive(Clone, Debug)]
pub struct ObjectField {
    pub class: Lowercase<String>,
    pub name: Lowercase<String>,
    pub offset: usize,
    pub ty: FieldType,
    pub ty_name: Lowercase<String>,
    pub default_value: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct ObjectProperty {
    pub class: Lowercase<String>,
    pub name: Lowercase<String>,
    pub var: usize,
}

#[derive(Clone, Debug)]
pub enum FieldType {
    Bool,
    I8,
    I32,
    F32,
    String,
    Isize,
    Pointer,
}

impl FieldType {
    pub fn size(&self) -> usize {
        match self {
            Self::Bool => 4,
            Self::I8 => 1,
            Self::I32 => 4,
            Self::F32 => 4,
            Self::Isize => 8,
            Self::String | Self::Pointer => 8,
        }
    }

    pub fn align(&self) -> usize {
        match self {
            Self::Bool => 4,
            Self::I8 => 1,
            Self::I32 => 4,
            Self::F32 => 4,
            Self::Isize => 8,
            Self::String | Self::Pointer => align_of::<usize>(),
        }
    }
}
