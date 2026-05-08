mod codegen;
mod validation;

pub mod resources;

use std::sync::Arc;

use game_common::string::Lowercase;
use hashbrown::HashMap;
use openfo4_jit::{CompiledFunction, CompilerConfig, Signature};
use openfo4_pex::Module;
use thiserror::Error;

use crate::compiler::codegen::CodegenPass;
use crate::compiler::resources::Resources;
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
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            validation: ValidationPass::default(),
            codegen: CodegenPass::new(config),
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

    pub fn compute_type_layouts(&mut self) {
        self.codegen.compute_type_layouts();
    }

    pub fn compile(&mut self, class: &str, options: &Options) -> Result<CompiledClass, Error> {
        self.validation.validate().map_err(Error::Validation)?;
        self.codegen.compile(class, options).map_err(Error::Codegen)
    }

    pub(crate) fn resources(&self) -> &Arc<Resources> {
        self.codegen.resources()
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
    pub signature: Signature,
}

#[derive(Clone, Debug)]
pub struct CompiledClass {
    pub functions: HashMap<Lowercase<String>, CompilationResult>,
}

#[derive(Clone, Debug, Default)]
pub struct CompilerOptions {
    pub validation: bool,
    pub debug: bool,
}
