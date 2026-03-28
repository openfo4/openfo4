use openfo4_jit::{Compiler, FuncId, Signature, Type};

use crate::runtime::host_functions;

#[derive(Copy, Clone, Debug)]
pub struct RuntimeFunctions {
    /// (ctx: *mut RawContext, layout: TypeId, array_len: usize) -> *mut u8
    pub alloc_zeroed: FuncId,
    /// (ctx: *mut RawContext, size: usize) -> *mut u8
    pub stack_frame_alloc: FuncId,
    /// (ctx: *mut RawContext, size: usize)
    pub stack_frame_free: FuncId,
    /// (ctx: *mut RawContext, value: i32) -> *mut u8
    pub i32_to_string: FuncId,
    /// (ctx: *mut RawContext, value: f32) -> *mut u8
    pub f32_to_string: FuncId,
    /// (ctx: *mut RawContext, value: *mut u8) -> *mut u8
    pub object_to_string: FuncId,
}

impl RuntimeFunctions {
    pub fn new(compiler: &mut Compiler) -> Self {
        let alloc_zeroed = declare_and_define(
            compiler,
            Signature {
                params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
                returns: vec![Type::Pointer],
            },
            host_functions::alloc_zeroed as *const u8,
        );
        let stack_frame_alloc = declare_and_define(
            compiler,
            Signature {
                params: vec![Type::Pointer, Type::Pointer],
                returns: vec![Type::Pointer],
            },
            host_functions::alloc_stack_frame as *const u8,
        );
        let stack_frame_free = declare_and_define(
            compiler,
            Signature {
                params: vec![Type::Pointer, Type::Pointer],
                returns: vec![],
            },
            host_functions::free_stack_frame as *const u8,
        );
        let i32_to_string = declare_and_define(
            compiler,
            Signature {
                params: vec![Type::Pointer, Type::I32],
                returns: vec![Type::Pointer],
            },
            host_functions::i32_to_string as *const u8,
        );
        let f32_to_string = declare_and_define(
            compiler,
            Signature {
                params: vec![Type::Pointer, Type::F32],
                returns: vec![Type::Pointer],
            },
            host_functions::f32_to_string as *const u8,
        );
        let object_to_string = declare_and_define(
            compiler,
            Signature {
                params: vec![Type::Pointer, Type::Pointer],
                returns: vec![Type::Pointer],
            },
            host_functions::object_to_string as *const u8,
        );

        Self {
            alloc_zeroed,
            stack_frame_alloc,
            stack_frame_free,
            i32_to_string,
            f32_to_string,
            object_to_string,
        }
    }
}

fn declare_and_define(compiler: &mut Compiler, signature: Signature, ptr: *const u8) -> FuncId {
    let id = compiler.declare_function(signature);
    compiler.define_native_function(id, ptr);
    id
}
