use std::any::Any;
use std::marker::{PhantomData, PhantomPinned};
use std::mem::MaybeUninit;
use std::pin::{Pin, pin};
use std::sync::{Arc, Weak};
use std::task::{self, Poll, Waker, ready};

use futures_lite::FutureExt;
use openfo4_common::cell::UnsafeRefCell;
use openfo4_jit::{CompiledFunction, Signature, Type};
use pin_project::pin_project;
use thiserror::Error;

use crate::compiler::{CompilationResult, PscSignature};
use crate::engine::EngineShared;
use crate::runtime::{
    Context, Error, NativeTuple, ObjectPool, PanicMsg, RESULT_ERROR, RESULT_OK, RESULT_SUSPEND,
    RawContext, RawStack, RawVal, TypeArray, TypeOfVal, Val, new_assignment_error,
};

#[derive(Clone, Debug)]
pub struct Func {
    func: CompiledFunction,
    signature: Signature,
    psc_sig: PscSignature,
    engine: Weak<EngineShared>,
}

impl Func {
    /// # Safety
    ///
    /// - The function pointer must point to a [`NativeFnPtr`] function.
    pub(crate) unsafe fn new(res: &CompilationResult, engine: Weak<EngineShared>) -> Self {
        // Should always match the ABI of `host_call_trampoline`.
        debug_assert_eq!(
            res.function.signature(),
            &Signature {
                params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
                returns: vec![Type::I32]
            }
        );

        Self {
            func: res.function.clone(),
            signature: res.signature.clone(),
            psc_sig: res.psc_signature.clone(),
            engine,
        }
    }

    unsafe fn call_in(
        &self,
        ctx: *mut RawContext,
        params: *const RawVal,
        returns: *mut RawVal,
    ) -> i32 {
        let args = &[ctx.cast::<u8>(), params.cast::<u8>(), returns.cast::<u8>()];
        let mut res: i32 = 0;

        unsafe {
            self.func
                .call(args.as_ptr().cast::<u8>(), (&raw mut res).cast::<u8>());
        }

        res
    }

    pub fn call<'a, 'b, T>(&'a self, ctx: &'a mut Context<'b, T>, params: &[Val]) -> Call<'a, 'b, T>
    where
        T: Send + 'static,
    {
        assert_eq!(
            self.psc_sig.params.len(),
            params.len(),
            "function takes {} parameters, {} were given",
            self.psc_sig.params.len(),
            params.len()
        );
        validate_parameter_types(&ctx.engine.state.pool, params, &self.psc_sig);

        let mut r_params = vec![MaybeUninit::<RawVal>::uninit(); params.len()];
        let mut r_returns = vec![MaybeUninit::<RawVal>::uninit(); self.psc_sig.returns.len()];

        // We don't need logic to drop written `MaybeUninit<RawVal>` values.
        const {
            assert!(!core::mem::needs_drop::<RawVal>());
        }

        for (index, param) in params.iter().enumerate() {
            r_params[index].write(param.into_raw());
        }

        // SAFETY:
        // - When we have constructed this `TypedFunc` we have asserted that the signature
        //   matches.
        // - We have filled all params according to the function signature.
        // - We have allocated enough capacity for all return values according to the function
        //   signature.
        let inner = unsafe {
            self.call_unchecked(
                ctx,
                r_params.as_ptr().cast::<RawVal>(),
                r_returns.as_mut_ptr().cast::<RawVal>(),
            )
        };

        Call {
            inner,
            r_params,
            r_returns,
        }
    }

    /// Calls the guest function.
    ///
    /// # Panics
    ///
    /// - Panics if the provided [`Context`] was created from a different [`Engine`] than this
    ///   `Func`.
    /// - Panics if the [`Stack`] of the provided [`Context`] is not empty and contains contents
    ///   for a different `Func`.
    ///
    /// # Safety
    ///
    /// - The values in `params` must match the function signature.
    /// - The `returns` allocation must be large enough for the return parameters of the function
    ///   signature.
    pub unsafe fn call_unchecked<'a, 'b, T>(
        &'a self,
        ctx: &'a mut Context<'b, T>,
        params: *const RawVal,
        returns: *mut RawVal,
    ) -> CallUnchecked<'a, 'b, T>
    where
        T: Send + 'static,
    {
        // The guest function will read from the stack and on suspension assume that
        // current contents match the contents of the previous writes.
        // We must ensure that we only resume using a stack that is compatible with
        // function we are calling, otherwise the guest function will begin reading
        // arbitrary memory and reinterpret memory incorrectly.
        assert!(
            core::ptr::addr_eq(Arc::as_ptr(&ctx.engine), Weak::as_ptr(&self.engine)),
            "attempted to call func with context from another Engine"
        );
        if let Some(stack_fn_ptr) = ctx.stack.func {
            let fn_ptr = self.func.as_ptr();
            assert!(
                core::ptr::addr_eq(fn_ptr, stack_fn_ptr),
                "attempted to reuse stack for incompatible func"
            );
        }

        CallUnchecked {
            ctx,
            func: self,
            raw_ctx: MaybeUninit::uninit(),
            raw_ctx_init: false,
            params,
            returns,
            _pinned: PhantomPinned,
        }
    }

    pub fn typed<Params, Returns>(
        &self,
    ) -> Result<TypedFunc<'_, Params, Returns>, FuncSignatureMismatch>
    where
        Params: NativeTuple,
        Returns: NativeTuple,
    {
        if &self.signature.params[1..] != Params::TYPES
            || &self.signature.returns[1..] != Returns::TYPES
        {
            let expected = Signature {
                params: Params::TYPES.to_vec(),
                returns: Returns::TYPES.to_vec(),
            };
            let found = self.signature.clone();

            return Err(FuncSignatureMismatch { expected, found });
        }

        Ok(TypedFunc {
            func: self,
            _marker: PhantomData,
        })
    }
}

#[derive(Clone, Debug, Error)]
#[error("expected {expected:?} found {found:?}")]
pub struct FuncSignatureMismatch {
    expected: Signature,
    found: Signature,
}

/// A statically typed function.
pub struct TypedFunc<'a, Params, Returns> {
    func: &'a Func,
    _marker: PhantomData<fn(Params) -> Returns>,
}

impl<'a, Params, Returns> TypedFunc<'a, Params, Returns>
where
    Params: NativeTuple,
    Returns: NativeTuple,
{
    pub fn call<'s, 'b, T>(
        &'s self,
        ctx: &'s mut Context<'b, T>,
        params: Params,
    ) -> TypedCall<'s, 'b, T, Params, Returns>
    where
        T: Send + 'static,
    {
        validate_parameter_types(&ctx.engine.state.pool, &params, &self.func.psc_sig);

        // TODO: When const_generics is stabilized we can avoid the heap allocation
        // and use a regular array.
        let mut r_params = vec![MaybeUninit::<RawVal>::uninit(); Params::SIZE];
        let mut r_returns = vec![MaybeUninit::<RawVal>::uninit(); Returns::SIZE];

        // We don't need logic to drop written `MaybeUninit<RawVal>` values.
        const {
            assert!(!core::mem::needs_drop::<RawVal>());
        }

        unsafe {
            params.write_raw(r_params.as_mut_ptr().cast::<RawVal>());
        }

        // SAFETY:
        // - When we have constructed this `TypedFunc` we have asserted that the signature
        //   matches.
        // - We have filled all params according to the function signature.
        // - We have allocated enough capacity for all return values according to the function
        //   signature.
        let inner = unsafe {
            self.func.call_unchecked(
                ctx,
                r_params.as_ptr().cast::<RawVal>(),
                r_returns.as_mut_ptr().cast::<RawVal>(),
            )
        };

        TypedCall {
            inner,
            r_params,
            r_returns,
            _marker: PhantomData,
        }
    }
}

fn validate_parameter_types<T>(pool: &ObjectPool, params: &T, signature: &PscSignature)
where
    T: ?Sized + TypeArray,
{
    let res = pool.resources();

    for (index, (is, expect)) in params.type_ids(pool).zip(&signature.params).enumerate() {
        match is {
            TypeOfVal::Concrete(is) => {
                if is == *expect {
                    continue;
                }

                let is_ty = res.types.get(is).unwrap();
                if !is_ty.info.is_coercible(*expect) {
                    panic!(
                        "{}",
                        new_assignment_error(res, TypeOfVal::Concrete(is), *expect)
                    )
                }
            }
            TypeOfVal::Null => {
                let expect = res.types.get(*expect).unwrap();

                // For non-static functions the first argument is the callee object,
                // which cannot be null.
                let is_callee_object = !signature.is_static && index == 0;

                if !expect.info.allows_null || is_callee_object {
                    panic!("type {:?} is null, but the type is non-nullable", expect);
                }
            }
            TypeOfVal::Invalid => {
                panic!("passing invalid value");
            }
        }
    }
}

pub struct CallUnchecked<'a, 'b, T> {
    func: &'a Func,
    ctx: &'a mut Context<'b, T>,
    params: *const RawVal,
    returns: *mut RawVal,
    raw_ctx: MaybeUninit<RawContext>,
    raw_ctx_init: bool,
    // We must not move the RawContext.
    _pinned: PhantomPinned,
}

impl<'a, 'b, T> CallUnchecked<'a, 'b, T> {
    /// Calls the function once, consuming the future.
    ///
    /// If the future yielded `Poll::Ready(..)` this function returns the result, otherwise if
    /// the callee suspended this function returns `Err(..)`.
    pub fn now_or_never(self) -> Result<(), Error>
    where
        T: Send + 'static,
    {
        let fut = pin!(self);
        let waker = Waker::noop();
        let mut cx = task::Context::from_waker(waker);
        match fut.poll(&mut cx) {
            Poll::Ready(res) => res,
            Poll::Pending => Err(Error {
                msg: "suspended".to_owned(),
            }),
        }
    }
}

impl<'a, 'b, T> Future for CallUnchecked<'a, 'b, T>
where
    T: Send + 'static,
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        if !this.raw_ctx_init {
            this.raw_ctx_init = true;
            debug_assert_eq!(this.ctx.stack.size(), 0);

            this.ctx.stack.func = Some(this.func.func.as_ptr());

            // Allocate enough stack space for the callee prologue.
            this.ctx.stack.alloc(size_of::<usize>());

            let ctx = RawContext {
                stack_ptr: RawStack {
                    ptr: this.ctx.stack.as_ptr(),
                    len: 0,
                },
                state: &this.ctx.engine.state,
                data: &mut this.ctx.data as *mut (dyn Any + Send),
                stack: this.ctx.stack,
                panic: PanicMsg::default(),
                waker: core::ptr::null(),
                error: UnsafeRefCell::new(None),
                lock: None,
            };

            this.raw_ctx.write(ctx);
        }

        let raw_ctx = unsafe { this.raw_ctx.assume_init_mut() };

        raw_ctx.stack_ptr.len = 0;
        raw_ctx.waker = cx.waker();
        raw_ctx.lock = None;

        let res = unsafe { this.func.call_in(raw_ctx, this.params, this.returns) };

        // If the callee calls into a native host function that panics,
        // we will resume the panic here.
        // The following code will not be executed.
        raw_ctx.panic.resume();

        match res {
            RESULT_SUSPEND => {
                if let Some(lock) = &mut raw_ctx.lock {
                    match lock.poll(cx) {
                        Poll::Pending => {}
                        // It is possible that the lock has become availabe since attempted
                        // to acquire it.
                        // In this case we need to immediately reregister out future.
                        Poll::Ready(()) => {
                            cx.waker().wake_by_ref();
                            raw_ctx.lock = None;
                        }
                    }
                }

                Poll::Pending
            }
            RESULT_OK => {
                this.ctx.stack.free(size_of::<usize>());
                debug_assert_eq!(this.ctx.stack.size(), 0);

                this.ctx.stack.func = None;

                Poll::Ready(Ok(()))
            }
            RESULT_ERROR => {
                let err = raw_ctx.error.get_mut().take().unwrap();
                return Poll::Ready(Err(*err));
            }
            _ => unreachable!(),
        }
    }
}

impl<'a, 'b, T> Drop for CallUnchecked<'a, 'b, T> {
    fn drop(&mut self) {
        if self.raw_ctx_init {
            unsafe {
                self.raw_ctx.assume_init_drop();
            }
        }
    }
}

// All our fields are Send, only `*const/mut RawVal` must be sendable,
// which is an unsafe requirement to construct this future.
unsafe impl<'a, 'b, T> Send for CallUnchecked<'a, 'b, T> where T: Send {}

// We only expose a &mut self method. No data can be accessed using a
// shared reference, so the `Sync` is safe to implement unconditionally.
unsafe impl<'a, 'b, T> Sync for CallUnchecked<'a, 'b, T> {}

#[pin_project]
pub struct Call<'a, 'b, T> {
    #[pin]
    inner: CallUnchecked<'a, 'b, T>,
    r_params: Vec<MaybeUninit<RawVal>>,
    r_returns: Vec<MaybeUninit<RawVal>>,
}

impl<'a, 'b, T> Call<'a, 'b, T> {
    /// Calls the function once, consuming the future.
    ///
    /// If the future yielded `Poll::Ready(..)` this function returns the result, otherwise if
    /// the callee suspended this function returns `Err(..)`.
    pub fn now_or_never(self) -> Result<Vec<Val>, Error>
    where
        T: Send + 'static,
    {
        let fut = pin!(self);
        let waker = Waker::noop();
        let mut cx = task::Context::from_waker(waker);
        match fut.poll(&mut cx) {
            Poll::Ready(res) => res,
            Poll::Pending => Err(Error {
                msg: "suspended".to_owned(),
            }),
        }
    }
}

impl<'a, 'b, T> Future for Call<'a, 'b, T>
where
    T: Send + 'static,
{
    type Output = Result<Vec<Val>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        ready!(this.inner.as_mut().poll(cx))?;

        let signature = &this.inner.func.psc_sig;

        debug_assert_eq!(signature.returns.len(), this.r_returns.len());

        let mut returns = Vec::with_capacity(this.inner.func.signature.returns.len());
        for (index, ty) in signature.returns.iter().enumerate() {
            let raw = unsafe { this.r_returns[index].assume_init_ref() };
            let val = unsafe { Val::from_raw(&this.inner.ctx.engine.state.pool, raw, *ty) };
            returns.push(val);
        }

        Poll::Ready(Ok(returns))
    }
}

#[pin_project]
pub struct TypedCall<'a, 'b, T, Params, Returns> {
    #[pin]
    inner: CallUnchecked<'a, 'b, T>,
    r_params: Vec<MaybeUninit<RawVal>>,
    r_returns: Vec<MaybeUninit<RawVal>>,
    _marker: PhantomData<fn(Params) -> Returns>,
}

impl<'a, 'b, T, Params, Returns> TypedCall<'a, 'b, T, Params, Returns> {
    /// Calls the function once, consuming the future.
    ///
    /// If the future yielded `Poll::Ready(..)` this function returns the result, otherwise if
    /// the callee suspended this function returns `Err(..)`.
    pub fn now_or_never(self) -> Result<Returns, Error>
    where
        T: Send + 'static,
        Returns: NativeTuple,
    {
        let fut = pin!(self);
        let waker = Waker::noop();
        let mut cx = task::Context::from_waker(waker);
        match fut.poll(&mut cx) {
            Poll::Ready(res) => res,
            Poll::Pending => Err(Error {
                msg: "suspended".to_owned(),
            }),
        }
    }
}

impl<'a, 'b, T, Params, Returns> Future for TypedCall<'a, 'b, T, Params, Returns>
where
    T: Send + 'static,
    Returns: NativeTuple,
{
    type Output = Result<Returns, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.inner.poll(cx))?;

        let returns = unsafe { Returns::from_raw(this.r_returns.as_ptr().cast::<RawVal>()) };
        Poll::Ready(Ok(returns))
    }
}
