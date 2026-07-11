use thiserror::Error;

use crate::compiler::PscSignature;

pub mod assembler;
pub mod compiler;
pub mod engine;
pub mod runtime;

#[derive(Clone, Debug, Error)]
pub enum RegisterNativeError {
    #[error(transparent)]
    UnknownFunction(UnknownFunctionError),
    #[error("incompatible signature {found:?}, script expected {expected:?}")]
    IncompatibleSignature {
        expected: PscSignature,
        found: PscSignature,
    },
}

#[derive(Clone, Debug, Error)]
#[error("unknown function {class}.{func}")]
pub struct UnknownFunctionError {
    class: String,
    func: String,
}
