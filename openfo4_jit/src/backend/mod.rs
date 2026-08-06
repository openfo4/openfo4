use std::range::Range;

use slab::Slab;

use crate::backend::interpreter::Interpreter;
use crate::func::HostFunc;
use crate::{BackendKind, BlockProps, CompilerConfig, FuncId, Instruction, Signature, SourceLoc};

#[cfg(feature = "cranelift")]
pub mod cranelift;

pub mod interpreter;

pub trait Backend {
    type CompiledFunc;

    fn new(config: CompilerConfig) -> Self;

    fn declare_function(&mut self, signature: &Signature) -> FuncId;

    fn define_function(&mut self, id: FuncId, definition: FunctionDefinitionDesc);

    fn define_host_function(&mut self, id: FuncId, func: HostFunc);

    fn compile_function(&mut self, id: FuncId) -> Self::CompiledFunc;
}

pub enum BackendEnum {
    Interpreter(Interpreter),
    Cranelift(cranelift::CraneliftModule),
}

impl Backend for BackendEnum {
    type CompiledFunc = CompiledFunc;

    fn new(config: CompilerConfig) -> Self {
        match config.backend {
            BackendKind::Interpreter => Self::Interpreter(Interpreter::new(config)),
            BackendKind::Cranelift => Self::Cranelift(cranelift::CraneliftModule::new(config)),
        }
    }

    fn declare_function(&mut self, signature: &Signature) -> FuncId {
        match self {
            Self::Interpreter(backend) => backend.declare_function(signature),
            Self::Cranelift(backend) => backend.declare_function(signature),
        }
    }

    fn define_host_function(&mut self, id: FuncId, func: HostFunc) {
        match self {
            Self::Interpreter(backend) => backend.define_host_function(id, func),
            Self::Cranelift(backend) => backend.define_host_function(id, func),
        }
    }

    fn define_function(&mut self, id: FuncId, definition: FunctionDefinitionDesc) {
        match self {
            Self::Interpreter(backend) => backend.define_function(id, definition),
            Self::Cranelift(backend) => backend.define_function(id, definition),
        }
    }

    fn compile_function(&mut self, id: FuncId) -> Self::CompiledFunc {
        match self {
            Self::Interpreter(backend) => CompiledFunc::Intepreter(backend.compile_function(id)),
            Self::Cranelift(backend) => CompiledFunc::Cranelift(backend.compile_function(id)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FunctionDefinitionDesc {
    pub instructions: Vec<Instruction>,
    pub num_values: usize,
    pub num_variables: usize,
    pub stack_slots: Slab<StackSlotDesc>,
    pub srcloc: SourceLocMap,
    pub blocks: Vec<BlockProps>,
}

#[derive(Copy, Clone, Debug)]
pub struct StackSlotDesc {
    pub size: usize,
    pub align: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SourceLocMap {
    // FIXME: We can use a range map like thing here.
    locs: Vec<Option<SourceLoc>>,
}

impl SourceLocMap {
    pub fn insert(&mut self, start: usize, end: usize, loc: SourceLoc) {
        self.locs.resize(end, None);
        for index in start..end {
            self.locs[index] = Some(loc);
        }
    }

    pub fn get(&self, index: usize) -> Option<SourceLoc> {
        self.locs.get(index).copied().flatten()
    }

    pub fn iter(&self) -> SourceLocMapIter<'_> {
        SourceLocMapIter {
            index: 0,
            map: self,
        }
    }
}

impl<'a> IntoIterator for &'a SourceLocMap {
    type Item = <SourceLocMapIter<'a> as Iterator>::Item;
    type IntoIter = SourceLocMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug)]
pub struct SourceLocMapIter<'a> {
    index: usize,
    map: &'a SourceLocMap,
}

impl<'a> Iterator for SourceLocMapIter<'a> {
    type Item = (Range<usize>, SourceLoc);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = 0;
        let mut loc = None;
        for (i, l) in self.map.locs[self.index..].iter().enumerate() {
            loc = *l;
            start = i;
            if loc.is_some() {
                break;
            }
        }

        start = self.index + start;
        let loc = loc?;

        for (i, l) in self.map.locs[start..].iter().enumerate() {
            if *l != Some(loc) {
                let end = start + i;
                self.index = end;
                return Some((Range { start, end }, loc));
            }
        }

        self.index = self.map.locs.len();
        Some((
            Range {
                start,
                end: self.map.locs.len(),
            },
            loc,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct TextWithSrcLocs<T> {
    pub text: T,
    pub srclocs: SourceLocMap,
}

#[derive(Clone)]
pub enum CompiledFunc {
    Intepreter(interpreter::CompiledFunc),
    Cranelift(cranelift::CompiledFunction),
}

impl CompiledFunc {
    pub unsafe fn call(&self, params: *const u8, returns: *mut u8) {
        unsafe {
            match self {
                Self::Intepreter(func) => func.call(params, returns),
                Self::Cranelift(func) => func.call(params, returns),
            }
        }
    }

    pub fn as_ptr(&self) -> *const u8 {
        match self {
            Self::Intepreter(func) => core::ptr::without_provenance(func.index()),
            Self::Cranelift(func) => func.as_ptr(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Intepreter(_) => &[],
            Self::Cranelift(func) => func.as_bytes(),
        }
    }

    pub fn clif(&self) -> Option<&TextWithSrcLocs<String>> {
        match self {
            Self::Cranelift(func) => func.clif(),
            _ => None,
        }
    }

    pub fn srclocs(&self) -> &SourceLocMap {
        match self {
            Self::Intepreter(func) => func.srclocs(),
            Self::Cranelift(func) => func.srclocs(),
        }
    }
}
