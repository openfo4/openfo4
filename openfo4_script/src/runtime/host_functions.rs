use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use game_common::utils::vec_ext::VecExt;
use tracing::span::EnteredSpan;

use crate::compiler::resources::TypeId;
use crate::runtime::{ObjectAddr, RawContext, RawVal};

pub unsafe extern "C" fn call_host_trampoline(
    ctx: *mut RawContext,
    index: usize,
    params: *const RawVal,
    returns: *mut RawVal,
) -> i32 {
    match catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: We have locked the call table before any guest function is called.
        let call_table = unsafe { (*(*ctx).state).call_table.borrow() };
        let func = &call_table.functions[index];
        func(ctx, params, returns)
    })) {
        Ok(v) => v,
        #[cfg(panic = "unwind")]
        Err(msg) => {
            let ctx = unsafe { &mut *ctx };
            ctx.panic.set(msg);
            // Value does not matter, except that it must be non-zero,
            // but we set it to a value that cannot be interpreted as any other status.
            i32::MAX
        }
        // Cannot unwind when aborting.
        #[cfg(not(panic = "unwind"))]
        Err(_) => unreachable!(),
    }
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

pub unsafe extern "C" fn free_stack_frame(ctx: *mut RawContext, size: usize) -> *mut u8 {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };
        unsafe { (*ctx.stack).free(size) }
    })
}

pub unsafe extern "C" fn bool_to_string(ctx: *mut RawContext, value: bool) -> ObjectAddr {
    abort_on_unwind(|| {
        let ctx = unsafe { &mut *ctx };

        let s = value.to_string();
        unsafe { (*ctx.state).pool.insert_string(&s).addr() }
    })
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

thread_local! {
    static SPANS: RefCell<Vec<EnteredSpan>> = RefCell::new(Vec::new());
}

pub unsafe extern "C" fn enter_span(_ctx: *mut RawContext, ptr: *const u8, len: usize) -> usize {
    abort_on_unwind(|| unsafe {
        let bytes = core::slice::from_raw_parts(ptr, len);
        let s = core::str::from_utf8_unchecked(bytes);

        let span = tracing::trace_span!("jit function", jit_function_name = s);
        let span = span.entered();

        let index = SPANS.with_borrow_mut(|spans| spans.push_and_index(span));

        index
    })
}

pub unsafe extern "C" fn exit_span(_ctx: *mut RawContext, index: usize) {
    abort_on_unwind(|| {
        SPANS.with_borrow_mut(|spans| {
            if index >= spans.len() {
                panic!("out of order span removal");
            }

            spans.pop();
        })
    });
}

pub unsafe extern "C" fn stub() {
    abort_on_unwind(|| {
        panic!("Entered stub function");
    })
}

fn abort_on_unwind<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    if cfg!(panic = "abort") {
        f()
    } else {
        match catch_unwind(AssertUnwindSafe(|| f())) {
            Ok(v) => v,
            Err(_) => std::process::abort(),
        }
    }
}
