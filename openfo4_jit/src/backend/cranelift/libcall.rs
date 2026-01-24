use core::cmp::Ordering;
use core::ffi::{c_int, c_void};

use cranelift_codegen::ir::LibCall;

pub fn get_address(libcall: LibCall) -> *const u8 {
    match libcall {
        LibCall::Probestack => todo!(),
        LibCall::CeilF32 => ceil_f32 as *const u8,
        LibCall::CeilF64 => ceil_f64 as *const u8,
        LibCall::FloorF32 => floor_f32 as *const u8,
        LibCall::FloorF64 => floor_f64 as *const u8,
        LibCall::TruncF32 => trunc_f32 as *const u8,
        LibCall::TruncF64 => trunc_f64 as *const u8,
        LibCall::NearestF32 => nearest_f32 as *const u8,
        LibCall::NearestF64 => nearest_f64 as *const u8,
        LibCall::FmaF32 => fma_f32 as *const u8,
        LibCall::FmaF64 => fma_f64 as *const u8,
        LibCall::Memcpy => memcpy as *const u8,
        LibCall::Memmove => memmove as *const u8,
        LibCall::Memset => memset as *const u8,
        LibCall::Memcmp => memcmp as *const u8,
        LibCall::ElfTlsGetAddr => todo!(),
        LibCall::ElfTlsGetOffset => todo!(),
        LibCall::X86Pshufb => {
            todo!()
        }
    }
}

unsafe extern "C" fn ceil_f32(v: f32) -> f32 {
    v.ceil()
}

unsafe extern "C" fn ceil_f64(v: f64) -> f64 {
    v.ceil()
}

unsafe extern "C" fn floor_f32(v: f32) -> f32 {
    v.floor()
}

unsafe extern "C" fn floor_f64(v: f64) -> f64 {
    v.floor()
}

unsafe extern "C" fn trunc_f32(v: f32) -> f32 {
    v.trunc()
}

unsafe extern "C" fn trunc_f64(v: f64) -> f64 {
    v.trunc()
}

unsafe extern "C" fn nearest_f32(v: f32) -> f32 {
    v.round_ties_even()
}

unsafe extern "C" fn nearest_f64(v: f64) -> f64 {
    v.round_ties_even()
}

unsafe extern "C" fn fma_f32(a: f32, b: f32, c: f32) -> f32 {
    a.mul_add(b, c)
}

unsafe extern "C" fn fma_f64(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

unsafe extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, count: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, count);
    }
}

unsafe extern "C" fn memmove(dst: *mut c_void, src: *const c_void, count: usize) {
    unsafe {
        core::ptr::copy(src, dst, count);
    }
}

unsafe extern "C" fn memset(dst: *mut c_void, c: c_int, count: usize) -> *mut c_void {
    unsafe {
        let byte = c as u8;
        core::ptr::write_bytes(dst, byte, count);
        dst
    }
}

unsafe extern "C" fn memcmp(lhs: *const c_void, rhs: *const c_void, count: usize) -> c_int {
    unsafe {
        let lhs = core::slice::from_raw_parts(lhs.cast::<u8>(), count);
        let rhs = core::slice::from_raw_parts(rhs.cast::<u8>(), count);
        match lhs.cmp(rhs) {
            Ordering::Equal => 0,
            Ordering::Greater => 1,
            Ordering::Less => -1,
        }
    }
}
