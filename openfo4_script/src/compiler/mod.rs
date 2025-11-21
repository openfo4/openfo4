mod codegen;
mod validation;

use openfo4_jit::{CompiledFunction, Signature};
use openfo4_pex::Module;
use thiserror::Error;

use crate::compiler::codegen::CodegenPass;
use crate::compiler::validation::ValidationPass;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Validation(validation::Error),
    #[error(transparent)]
    Codegen(codegen::Error),
}

#[derive(Clone, Debug)]
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

    pub fn compile(&mut self, class: &str, func: &str) -> Result<CompiledFunction, Error> {
        self.validation.validate().map_err(Error::Validation)?;
        let res = self
            .codegen
            .compile_fn(class, func)
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
