use std::any::Any;
use std::marker::{PhantomData, PhantomPinned};
use std::mem::{MaybeUninit, transmute};
use std::pin::Pin;
use std::sync::{Arc, Weak};
use std::task::{self, Poll, ready};

use game_common::cell::UnsafeRefCell;
use openfo4_jit::{Signature, Type};
use pin_project::pin_project;
use thiserror::Error;

use crate::compiler::{CompilationResult, PscSignature};
use crate::engine::EngineShared;
use crate::runtime::{
    Context, Error, NativeFnPtr, NativeTuple, NativeType, ObjectAddr, ObjectPool, PanicMsg,
    RESULT_ERROR, RESULT_OK, RESULT_SUSPEND, RawContext, RawStack, RawVal, TypeArray, TypeRef, Val,
};

#[derive(Clone, Debug)]
pub struct Func {
    ptr: NativeFnPtr,
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
            // SAFETY: The caller guarantees that the function is a valid
            // function pointer of type `NativeFnPtr`.
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(res.function.as_ptr()) },
            signature: res.signature.clone(),
            psc_sig: res.psc_signature.clone(),
            engine,
        }
    }

    pub fn call<T>(&self, ctx: &mut Context<'_, T>, params: &[Val]) -> Result<Vec<Val>, Error>
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
        let mut r_returns = vec![MaybeUninit::<RawVal>::uninit(); self.signature.returns.len()];

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
        unsafe {
            self.call_unchecked(
                ctx,
                r_params.as_ptr().cast::<RawVal>(),
                r_returns.as_mut_ptr().cast::<RawVal>(),
            )?;
        }

        let mut returns = Vec::with_capacity(self.signature.returns.len());
        for (index, ty) in self.signature.returns.iter().enumerate() {
            let raw = unsafe { r_returns[index].assume_init_ref() };

            unsafe {
                returns.push(match ty {
                    Type::I8 => Val::Bool(<bool as NativeType>::from_raw(raw)),
                    Type::I32 => Val::I32(<i32 as NativeType>::from_raw(raw)),
                    Type::F32 => Val::F32(<f32 as NativeType>::from_raw(raw)),
                    Type::Pointer => Val::Object(<ObjectAddr as NativeType>::from_raw(raw)),
                    _ => unreachable!(),
                });
            }
        }

        Ok(returns)
    }

    pub fn call_async<'a, 'b, T>(
        &'a self,
        ctx: &'a mut Context<'b, T>,
        params: &[Val],
    ) -> Call<'a, 'b, T>
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
        let mut r_returns = vec![MaybeUninit::<RawVal>::uninit(); self.signature.returns.len()];

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
            self.call_unchecked_async(
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
    pub unsafe fn call_unchecked<T>(
        &self,
        ctx: &mut Context<'_, T>,
        params: *const RawVal,
        returns: *mut RawVal,
    ) -> Result<(), Error>
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
            let fn_ptr = self.ptr as *const u8;
            assert!(
                core::ptr::addr_eq(fn_ptr, stack_fn_ptr),
                "attempted to reuse stack for incompatible func"
            );
        }

        ctx.stack.func = Some(self.ptr as *const u8);

        // Allocate enough stack space for the callee prologue.
        if ctx.stack.size() == 0 {
            ctx.stack.alloc(size_of::<usize>());
        }

        // We must lock the call table before we call the guest function.
        let _lock = ctx.engine.call_table_lock.read();

        let mut raw_ctx = RawContext {
            stack_ptr: RawStack {
                ptr: ctx.stack.as_ptr(),
                len: 0,
            },
            state: &ctx.engine.state,
            data: &mut ctx.data as *mut (dyn Any + Send),
            stack: ctx.stack,
            panic: PanicMsg::default(),
            waker: task::Waker::noop(),
            error: UnsafeRefCell::new(None),
        };

        let res = unsafe { (self.ptr)(&raw_ctx, params, returns) };

        // Once we drop `raw_ctx`, the future becomes invalid.
        // We must drop it before before that happens.
        // The future is only maintained for the call_*_async functions.
        ctx.stack.future.clear();

        // If the callee calls into a native host function that panics,
        // we will resume the panic here.
        // The following code will not be executed.
        raw_ctx.panic.resume();

        match res {
            RESULT_SUSPEND => {
                panic!("suspended on non-async call");
            }
            RESULT_OK => {
                ctx.stack.free(size_of::<usize>());
                debug_assert_eq!(ctx.stack.size(), 0);

                ctx.stack.func = None;

                Ok(())
            }
            RESULT_ERROR => {
                let err = raw_ctx.error.get_mut().take().unwrap();
                return Err(*err);
            }
            _ => unreachable!(),
        }
    }

    pub unsafe fn call_unchecked_async<'a, 'b, T>(
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
            let fn_ptr = self.ptr as *const u8;
            assert!(
                core::ptr::addr_eq(fn_ptr, stack_fn_ptr),
                "attempted to reuse stack for incompatible func"
            );
        }

        CallUnchecked {
            ctx,
            func: self,
            raw_ctx: None,
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
            || &self.signature.returns[..self.signature.returns.len() - 1] != Returns::TYPES
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
    pub fn call<T>(&self, ctx: &mut Context<'_, T>, params: Params) -> Result<Returns, Error>
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
        unsafe {
            self.func.call_unchecked(
                ctx,
                r_params.as_ptr().cast::<RawVal>(),
                r_returns.as_mut_ptr().cast::<RawVal>(),
            )?;
        }

        let returns = unsafe { Returns::from_raw(r_returns.as_ptr().cast::<RawVal>()) };
        Ok(returns)
    }

    pub fn call_async<'s, 'b, T>(
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
            self.func.call_unchecked_async(
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

    for (is, expect) in params.type_ids(pool).zip(&signature.params) {
        match is {
            TypeRef::Concrete(is) => {
                if is == *expect {
                    continue;
                }

                let is_ty = res.types.get(is).unwrap();
                if !is_ty.info.classes.contains(expect) {
                    panic!("type {:?} is not compatible with {:?}", is, expect);
                }
            }
            TypeRef::Null => {
                let expect = res.types.get(*expect).unwrap();
                if !expect.info.allows_null {
                    panic!("type {:?} is null, but the type is non-nullable", expect);
                }
            }
            TypeRef::Invalid => {
                panic!("passing invalid value");
            }
        }
    }
}

pub struct CallUnchecked<'a, 'b, T> {
    func: &'a Func,
    ctx: &'a mut Context<'b, T>,
    raw_ctx: Option<RawContext>,
    params: *const RawVal,
    returns: *mut RawVal,
    // We must not move the RawContext.
    _pinned: PhantomPinned,
}

impl<'a, 'b, T> Future for CallUnchecked<'a, 'b, T>
where
    T: Send + 'static,
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        let raw_ctx = this.raw_ctx.get_or_insert_with(|| {
            debug_assert_eq!(this.ctx.stack.size(), 0);

            this.ctx.stack.func = Some(this.func.ptr as *const u8);

            // Allocate enough stack space for the callee prologue.
            this.ctx.stack.alloc(size_of::<usize>());

            RawContext {
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
            }
        });

        raw_ctx.stack_ptr.len = 0;
        raw_ctx.waker = cx.waker();

        // We must lock the call table before we call the guest function.
        let _lock = this.ctx.engine.call_table_lock.read();

        let res = unsafe { (this.func.ptr)(raw_ctx, this.params, this.returns) };

        // If the callee calls into a native host function that panics,
        // we will resume the panic here.
        // The following code will not be executed.
        raw_ctx.panic.resume();

        match res {
            RESULT_SUSPEND => Poll::Pending,
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

impl<'a, 'b, T> Future for Call<'a, 'b, T>
where
    T: Send + 'static,
{
    type Output = Result<Vec<Val>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        ready!(this.inner.as_mut().poll(cx))?;

        let mut returns = Vec::with_capacity(this.inner.func.signature.returns.len());
        for (index, ty) in this.inner.func.signature.returns.iter().enumerate() {
            let raw = unsafe { this.r_returns[index].assume_init_ref() };

            unsafe {
                returns.push(match ty {
                    Type::I8 => Val::Bool(<bool as NativeType>::from_raw(raw)),
                    Type::I32 => Val::I32(<i32 as NativeType>::from_raw(raw)),
                    Type::F32 => Val::F32(<f32 as NativeType>::from_raw(raw)),
                    Type::Pointer => Val::Object(<ObjectAddr as NativeType>::from_raw(raw)),
                    _ => unreachable!(),
                });
            }
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
