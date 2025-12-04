use std::cell::OnceCell;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;

use crate::classes::{Float, Int, State};
use crate::runtime::{Caller, Exception};

static X: OnceLock<()> = OnceLock::new();

pub fn show(
    caller: Caller<'_, State>,
    self_ptr: isize,
    arg1: Float,
    arg2: Float,
    arg3: Float,
    arg4: Float,
    arg5: Float,
    arg6: Float,
    arg7: Float,
    arg8: Float,
    arg9: Float,
) -> Result<Int, Exception> {
    if X.get().is_some() {
        Ok(0)
    } else {
        X.set(()).unwrap();

        Err(Exception::Suspend)
    }
}
