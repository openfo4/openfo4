use std::panic::AssertUnwindSafe;

use crate::compiler::resources::TypeId;
use crate::runtime::{ObjectAddr, RawContext, RawVal};

pub unsafe extern "C" fn call_host_trampoline(
    ctx: *const RawContext,
    index: usize,
    args: *mut RawVal,
) {
    abort_on_unwind(|| {
        // SAFETY: We have locked the call table before any guest function is called.
        let call_table = unsafe { (*(*ctx).state).call_table.borrow() };
        let func = &call_table.functions[index];
        func(ctx, args);
    })
}

pub unsafe extern "C" fn alloc_zeroed(
    ctx: *mut RawContext,
    ty: TypeId,
    array_len: usize,
) -> *mut u8 {
    abort_on_unwind(|| {
        let object_ref = unsafe { (*(*ctx).state).pool.alloc(ty, array_len) };
        object_ref.addr().to_raw() as *mut u8
    })
}

pub unsafe extern "C" fn alloc_stack_frame(ctx: *mut RawContext, size: usize) -> *mut u8 {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };
        let ptr = unsafe { (*ctx.stack).alloc(size) };
        ptr
    })
}

pub unsafe extern "C" fn free_stack_frame(ctx: *mut RawContext, size: usize) {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };
        unsafe {
            (*ctx.stack).free(size);
        }
    });
}

pub unsafe extern "C" fn i32_to_string(ctx: *mut RawContext, value: i32) -> ObjectAddr {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };

        let s = value.to_string();
        unsafe { (*ctx.state).pool.insert_string(&s).addr() }
    })
}

pub unsafe extern "C" fn f32_to_string(ctx: *mut RawContext, value: f32) -> ObjectAddr {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };

        let s = value.to_string();
        unsafe { (*ctx.state).pool.insert_string(&s).addr() }
    })
}

pub unsafe extern "C" fn object_to_string(ctx: *mut RawContext, addr: *mut u8) -> ObjectAddr {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };

        let s = format!("{:x}", addr.addr());
        unsafe { (*ctx.state).pool.insert_string(&s).addr() }
    })
}

fn abort_on_unwind<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    if cfg!(panic = "abort") {
        f()
    } else {
        match std::panic::catch_unwind(AssertUnwindSafe(|| f())) {
            Ok(v) => v,
            Err(_) => std::process::abort(),
        }
    }
}
