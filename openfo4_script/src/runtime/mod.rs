pub mod func;
pub mod object;
mod stack;

pub(crate) mod host_functions;

use std::any::Any;
use std::fmt::Display;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::task::{Poll, Waker};

use openfo4_common::cell::UnsafeRefCell;
use openfo4_jit::func::{HostFunc, HostType, IntoHostFunc};
use openfo4_jit::{Signature, Type};

pub use object::{ArrayObject, ObjectAddr, ObjectPool};
pub use stack::Stack;

use crate::compiler::PscSignature;
use crate::compiler::resources::TypeId;
use crate::engine::{Engine, EngineShared};
use crate::runtime::host_functions::{enter_span, exit_span};
use crate::runtime::object::ObjectHandleRef;

pub(crate) const RESULT_OK: i32 = 0;
pub(crate) const RESULT_ERROR: i32 = 1;
pub(crate) const RESULT_SUSPEND: i32 = 2;

/// Context passed to guest functions.
pub struct Context<'a, T> {
    engine: Arc<EngineShared>,
    stack: &'a mut Stack,
    data: T,
}

impl<'a, T> Context<'a, T> {
    pub fn new(engine: &Engine, stack: &'a mut Stack, data: T) -> Self {
        assert!(Arc::ptr_eq(&engine.shared, &stack.engine));

        Self {
            engine: engine.shared.clone(),
            stack,
            data,
        }
    }

    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn into_data(self) -> T {
        self.data
    }
}

// The stack pointer must be at offset 0.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct RawContext {
    pub stack_ptr: RawStack,
    pub state: *const HostState,
    pub data: *mut (dyn Any + Send),
    pub stack: *mut Stack,
    panic: PanicMsg,
    waker: *const Waker,
    error: UnsafeRefCell<Option<Box<Error>>>,
}

unsafe impl Send for RawContext {}

#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct RawStack {
    pub ptr: *mut u8,
    pub len: usize,
}

#[derive(Debug, Default)]
struct PanicMsg {
    #[cfg(panic = "unwind")]
    msg: Option<Box<dyn Any + Send>>,
}

impl PanicMsg {
    /// Sets the panic message.
    #[cfg(panic = "unwind")]
    #[inline]
    fn set(&mut self, msg: Box<dyn Any + Send>) {
        self.msg = Some(msg);
    }

    /// Resumes the caught panic on the current thread.
    #[inline]
    fn resume(&mut self) {
        #[cfg(panic = "unwind")]
        if let Some(msg) = self.msg.take() {
            std::panic::resume_unwind(msg);
        }
    }
}

/// A union over all values that can be used in Papyrus functions.
#[derive(Copy, Clone)]
#[repr(C)]
pub union RawVal {
    /// A boolean type.
    ///
    /// This is the equivalent of the builtin `bool` type.
    pub bool: bool,
    /// An integer type.
    ///
    /// This is the equivalent of the builtin `int` type.
    pub i32: i32,
    /// A floating-point type.
    ///
    /// This is the equivalent of the builtin `float` type.
    pub f32: f32,
    /// An object pointer.
    ///
    /// This is the equivalent of class objects, structs, arrays and the builtin types `string`
    /// and `var`.
    pub object: ObjectAddr,
}

/// A value can be used in Papyrus functions.
#[derive(Copy, Clone, Debug)]
pub enum Val {
    /// A boolean type.
    ///
    /// This is the equivalent of the builtin `bool` type.
    Bool(bool),
    /// An integer type.
    ///
    /// This is the equivalent of the builtin `int` type.
    I32(i32),
    /// A floating-point type.
    ///
    /// This is the equivalent of the builtin `float` type.
    F32(f32),
    /// An object pointer.
    ///
    /// This is the equivalent of class objects, structs, arrays and the builtin types `string`
    /// and `var`.
    Object(ObjectAddr),
}

impl Val {
    const fn into_raw(self) -> RawVal {
        match self {
            Self::Bool(bool) => RawVal { bool },
            Self::I32(i32) => RawVal { i32 },
            Self::F32(f32) => RawVal { f32 },
            Self::Object(object) => RawVal { object },
        }
    }

    fn get_type_id(self, pool: &ObjectPool) -> TypeRef {
        let res = pool.resources();
        match self {
            Self::Bool(_) => TypeRef::Concrete(res.primitives.bool),
            Self::I32(_) => TypeRef::Concrete(res.primitives.int),
            Self::F32(_) => TypeRef::Concrete(res.primitives.float),
            Self::Object(addr) => {
                if addr == ObjectAddr::NULL {
                    return TypeRef::Null;
                }

                match pool.get(addr) {
                    Some(object) => TypeRef::Concrete(object.type_id()),
                    None => TypeRef::Invalid,
                }
            }
        }
    }
}

/// Context passed to host functions called from the guest.
pub struct Caller<'a, T>
where
    T: 'static,
{
    ctx: &'a RawContext,
    _marker: PhantomData<T>,
}

impl<'a, T> Caller<'a, T> {
    pub fn data(&self) -> &T {
        unsafe { &*self.ctx.data }.downcast_ref().unwrap()
    }

    pub fn data_mut(&mut self) -> &mut T {
        let data_ref = unsafe { &mut *self.ctx.data };
        data_ref.downcast_mut().unwrap()
    }

    pub unsafe fn array<U>(&self, addr: ObjectAddr) -> &[U] {
        if addr.is_null() {
            return &[];
        }

        unsafe {
            let object = &*addr.to_raw().cast::<ArrayObject>();

            // Array with len 0 MAY have a zero pointer that we cannot use,
            // even for a zero-sized slice.
            if object.len == 0 {
                &[]
            } else {
                core::slice::from_raw_parts(object.ptr.cast::<U>(), object.len)
            }
        }
    }

    pub unsafe fn array_mut<U>(&mut self, addr: ObjectAddr) -> &mut [U] {
        if addr.is_null() {
            return &mut [];
        }

        unsafe {
            let object = &*addr.to_raw().cast::<ArrayObject>();

            // Array with len 0 MAY have a zero pointer that we cannot use,
            // even for a zero-sized slice.
            if object.len == 0 {
                &mut []
            } else {
                core::slice::from_raw_parts_mut(object.ptr.cast::<U>(), object.len)
            }
        }
    }

    /// Views the contents of a string object.
    ///
    /// # Panics
    ///
    /// - Panics if the [`ObjectAddr`] is not allocated from this [`ObjectPool`] (allocated in a
    ///   different [`Engine`] or already deallocated).
    /// - Panics if the [`ObjectAddr`] is not a string object.
    pub fn string(&self, addr: ObjectAddr) -> &str {
        let obj = self.object_pool().get(addr).expect("object not allocated");
        if obj.type_id() != self.object_pool().resources().primitives.string {
            panic!("cannot view object as string");
        }

        // SAFETY: We have asserted that the object address is allocated and points
        // to a string object.
        unsafe {
            let data = self.array::<u8>(addr);
            std::str::from_utf8_unchecked(data)
        }
    }

    pub unsafe fn copy_array_of_structs<U>(&self, addr: ObjectAddr) -> Vec<U> {
        let addrs = unsafe { self.array::<ObjectAddr>(addr) };

        let mut objects = Vec::with_capacity(addrs.len());
        for addr in addrs {
            let object = unsafe { addr.to_raw().cast::<U>().read() };
            objects.push(object);
        }

        objects
    }

    pub fn object_pool(&self) -> &ObjectPool {
        unsafe { &(*self.ctx.state).pool }
    }
}

// Caller<'_, T> wraps a &'_ RawContext. RawContext is Send, but not Sync,
// however semantically we behave as if we wrapped an &'_ mut RawContext,
// which is Send.
// Note that we don't actually care about `T` here at all, since we don't
// know if we even have a `T` until a runtime typecheck.
// If the typecheck fails we never actually have a `T` value whose Send
// properties could be violated.
// If the typecheck suceeds, we know that the type is already guaranteed
// to be `Send`, as we only allow `Any + Send` types.
unsafe impl<T> Send for Caller<'_, T> {}

// Caller<'_, T> behaves as if we wrapped an &'_ mut RawContext, however
// all functions that access fields mutable require an exclusive reference
// to the Caller.
unsafe impl<T> Sync for Caller<'_, T> {}

pub(crate) struct HostState {
    pub pool: ObjectPool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FnMarker {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FalliableFnMarker {}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AsyncFnMarker<'a>(PhantomData<fn(&'a ())>);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FalliableAsyncFnMarker<'a>(PhantomData<fn(&'a ())>);

/// A native host function that can be called from the guest.
pub trait IntoFunc<T, Marker, Params, Returns> {
    #[doc(hidden)]
    fn into_func(self, signature: PscSignature, name: String) -> HostFunc;

    #[doc(hidden)]
    fn signature() -> Signature;
}

macro_rules! impl_into_func {
    ($($params:ident),* => $($returns:ident),*) => {
        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<'a, F, T, $($params,)* $($returns,)*> IntoFunc<T, FnMarker, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'_, T>, $($params),*) -> Result<($($returns),*), Error> + Send + Sync + 'static,
            T: 'static,
            $(
                $params: FromGuest<'a>,
            )*
            $(
                $returns: NativeType,
            )*
        {
            fn into_func(self, signature: PscSignature, name: String) -> HostFunc {
                IntoHostFunc::into_host_func(move |ctx: *const RawContext, $($params: <$params as FromGuest<'a>>::JitType),*| -> (i32 $(,MaybeUninit<$returns>)*) {
                    let _span = Span::new(ctx.cast_mut(), &name);

                    try_call!(ctx, $($returns),* => || {
                        let ctx = unsafe { &*ctx };

                        let caller = Caller {
                            ctx,
                            _marker: PhantomData,
                        };

                        $(
                            let $params = unsafe { $params::from_raw(ctx, $params) };
                        )*

                        match self(caller, $($params),*) {
                            Ok(($($returns),*)) => {
                                validate_return_value_types(ctx, &($($returns,)*), &signature);

                                (RESULT_OK $(,MaybeUninit::<$returns>::new($returns))*)
                            },
                            Err(err) => {
                                unsafe {
                                    *ctx.error.borrow_mut() = Some(Box::new(err));
                                }

                                (RESULT_ERROR $(,MaybeUninit::<$returns>::zeroed())*)
                            },
                        }
                    })
                })
            }

            fn signature() -> Signature {
                let mut params = Vec::new();
                $(
                    params.push($params::TYPE);
                )*

                let mut returns = Vec::new();
                $(
                    returns.push($returns::TYPE);
                )*

                Signature { params, returns }
            }
        }


        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<'a, F, T, $($params,)* $($returns,)*> IntoFunc<T, FalliableFnMarker, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'_, T>, $($params),*) -> ($($returns),*) + Send + Sync + 'static,
            T: 'static,
            $(
                $params: FromGuest<'a>,
            )*
            $(
                $returns: NativeType,
            )*
        {
            fn into_func(self, signature: PscSignature, name: String) -> HostFunc {
                IntoHostFunc::into_host_func(move |ctx: *const RawContext, $($params: <$params as FromGuest<'a>>::JitType),*| -> (i32 $(,MaybeUninit<$returns>)*) {
                    let _span = Span::new(ctx.cast_mut(), &name);

                    try_call!(ctx, $($returns),* => || {
                        let ctx = unsafe { &*ctx };

                        let caller = Caller {
                            ctx,
                            _marker: PhantomData,
                        };

                        $(
                            let $params = unsafe { $params::from_raw(ctx, $params) };
                        )*

                        let ($($returns),*) =  self(caller, $($params),*);
                        validate_return_value_types(ctx, &($($returns,)*), &signature);

                        (RESULT_OK $(,MaybeUninit::<$returns>::new($returns))*)
                    })
                })
            }

            fn signature() -> Signature {
                let mut params = Vec::new();
                $(
                    params.push($params::TYPE);
                )*

                let mut returns = Vec::new();
                $(
                    returns.push($returns::TYPE);
                )*

                Signature { params, returns }
            }
        }

        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<'a, F, Fut, T, $($params,)* $($returns,)*> IntoFunc<T, AsyncFnMarker<'a>, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'a, T>, $($params),*) -> Fut + Send + Sync + 'static,
            Fut: IntoFuture<Output = ($($returns),*)> + Send + 'a,
            T: 'static,
            $(
                $params: FromGuest<'a>,
            )*
            $(
                $returns: NativeType,
            )*
        {
            fn into_func(self, signature: PscSignature, name: String) -> HostFunc {
                IntoHostFunc::into_host_func(move |ctx: *const RawContext, $($params: <$params as FromGuest<'a>>::JitType),*| -> (i32 $(,MaybeUninit<$returns>)*) {
                    let _span = Span::new(ctx.cast_mut(), &name);

                    try_call!(ctx, $($returns),* => || {
                        let ctx = unsafe { &*ctx };
                        let stack = unsafe { &mut *ctx.stack };

                        let waker = unsafe { &*ctx.waker };
                        let mut cx = std::task::Context::from_waker(waker);

                        if !stack.future.is_init() {
                            let caller = Caller {
                                ctx,
                                _marker: PhantomData,
                            };

                            $(
                                let $params = unsafe { $params::from_raw(ctx, $params) };
                            )*

                            let future = self(caller, $($params),*).into_future();
                            stack.future.set(future);
                        }

                        match unsafe { stack.future.poll::<($($returns),*), Fut::IntoFuture>(&mut cx) } {
                            Poll::Ready(($($returns),*)) => {
                                validate_return_value_types(ctx, &($($returns,)*), &signature);

                                (RESULT_OK $(,MaybeUninit::<$returns>::new($returns))*)
                            }
                            Poll::Pending => {
                                (RESULT_SUSPEND $(,MaybeUninit::<$returns>::zeroed())*)
                            }
                        }
                    })
                })
            }

            fn signature() -> Signature {
                let mut params = Vec::new();
                $(
                    params.push($params::TYPE);
                )*

                let mut returns = Vec::new();
                $(
                    returns.push($returns::TYPE);
                )*

                Signature { params, returns }
            }
        }

        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<'a, F, Fut, T, $($params,)* $($returns,)*> IntoFunc<T, FalliableAsyncFnMarker<'a>, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'a, T>, $($params),*) -> Fut + Send + Sync + 'static,
            Fut: IntoFuture<Output = Result<($($returns),*), Error>> + Send + 'a,
            T: 'static,
            $(
                $params: FromGuest<'a>,
            )*
            $(
                $returns: NativeType,
            )*
        {
            fn into_func(self, signature: PscSignature, name: String) -> HostFunc {
                IntoHostFunc::into_host_func(move |ctx: *const RawContext, $($params: <$params as FromGuest<'a>>::JitType),*| -> (i32 $(,MaybeUninit<$returns>)*) {
                    let _span = Span::new(ctx.cast_mut(), &name);

                    try_call!(ctx, $($returns),* => || {
                        let ctx = unsafe { &*ctx };
                        let stack = unsafe { &mut *ctx.stack };

                        let waker = unsafe { &*ctx.waker };
                        let mut cx = std::task::Context::from_waker(waker);

                        if !stack.future.is_init() {
                            let caller = Caller {
                                ctx,
                                _marker: PhantomData,
                            };

                            $(
                                let $params = unsafe { $params::from_raw(ctx, $params) };
                            )*

                            let future = self(caller, $($params),*).into_future();
                            stack.future.set(future);
                        }

                        match unsafe { stack.future.poll::<Result<($($returns),*), Error>, Fut::IntoFuture>(&mut cx) } {
                            Poll::Ready(Ok(($($returns),*))) => {
                                validate_return_value_types(ctx, &($($returns,)*), &signature);

                                (RESULT_OK $(,MaybeUninit::<$returns>::new($returns))*)
                            }
                            Poll::Ready(Err(err)) => {
                                unsafe {
                                    *ctx.error.borrow_mut() = Some(Box::new(err));
                                }

                                (RESULT_ERROR $(,MaybeUninit::<$returns>::zeroed())*)
                            }
                            Poll::Pending => {
                                (RESULT_SUSPEND $(,MaybeUninit::<$returns>::zeroed())*)
                            }
                        }
                    })
                })
            }

            fn signature() -> Signature {
                let mut params = Vec::new();
                $(
                    params.push($params::TYPE);
                )*

                let mut returns = Vec::new();
                $(
                    returns.push($returns::TYPE);
                )*

                Signature { params, returns }
            }
        }
    };
}

fn validate_return_value_types<T>(ctx: &RawContext, values: &T, signature: &PscSignature)
where
    T: NativeTuple,
{
    let pool = unsafe { &(*ctx.state).pool };
    let res = pool.resources();

    for (is, expect) in values.type_ids(pool).zip(&signature.returns) {
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
                panic!("returning invalid value");
            }
        }
    }
}

impl_into_func!(=>);
impl_into_func!(=> R0);
impl_into_func!(A0 =>);
impl_into_func!(A0 => R0);
impl_into_func!(A0, A1 =>);
impl_into_func!(A0, A1 => R0);
impl_into_func!(A0, A1, A2 =>);
impl_into_func!(A0, A1, A2 => R0);
impl_into_func!(A0, A1, A2, A3 =>);
impl_into_func!(A0, A1, A2, A3 => R0);
impl_into_func!(A0, A1, A2, A3, A4 =>);
impl_into_func!(A0, A1, A2, A3, A4 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10 => R0);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11 =>);
impl_into_func!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11 => R0);

/// A type that can be passed to a host function from a guest function.
pub unsafe trait FromGuest<'a> {
    #[doc(hidden)]
    const TYPE: Type;

    #[doc(hidden)]
    type JitType: HostType;

    #[doc(hidden)]
    #[expect(private_interfaces)]
    unsafe fn from_raw(ctx: &RawContext, val: Self::JitType) -> Self;
}

unsafe impl<'a> FromGuest<'a> for Option<ObjectHandleRef<'a>> {
    const TYPE: Type = Type::Pointer;
    type JitType = ObjectAddr;

    #[expect(private_interfaces)]
    unsafe fn from_raw(ctx: &RawContext, addr: Self::JitType) -> Self {
        if addr.is_null() {
            None
        } else {
            unsafe {
                let pool = &(&*ctx.state).pool;
                Some(ObjectHandleRef::new(pool, addr))
            }
        }
    }
}

unsafe impl<'a, T> FromGuest<'a> for T
where
    T: NativeType,
{
    const TYPE: Type = T::TYPE;
    type JitType = T;

    #[expect(private_interfaces)]
    unsafe fn from_raw(_ctx: &RawContext, val: Self::JitType) -> Self {
        val
    }
}

/// A type that can be send to Papyrus functions.
pub unsafe trait NativeType: Copy + HostType {
    const TYPE: Type;

    unsafe fn from_raw(val: &RawVal) -> Self;

    fn to_raw(self) -> RawVal;

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef;
}

unsafe impl NativeType for bool {
    const TYPE: Type = Type::I8;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.bool }
    }

    fn to_raw(self) -> RawVal {
        RawVal { bool: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::Bool(*self).get_type_id(pool)
    }
}

unsafe impl NativeType for i32 {
    const TYPE: Type = Type::I32;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.i32 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { i32: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::I32(*self).get_type_id(pool)
    }
}

unsafe impl NativeType for f32 {
    const TYPE: Type = Type::F32;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.f32 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { f32: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::F32(*self).get_type_id(pool)
    }
}

unsafe impl NativeType for ObjectAddr {
    const TYPE: Type = Type::Pointer;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.object }
    }

    fn to_raw(self) -> RawVal {
        RawVal { object: self }
    }

    #[doc(hidden)]
    #[expect(private_interfaces)]
    fn get_type_id(&self, pool: &ObjectPool) -> TypeRef {
        Val::Object(*self).get_type_id(pool)
    }
}

#[expect(private_bounds)]
pub unsafe trait NativeTuple: TypeArray {
    const SIZE: usize;
    const TYPES: &[Type];

    unsafe fn write_raw(self, ptr: *mut RawVal);

    unsafe fn from_raw(ptr: *const RawVal) -> Self;
}

unsafe impl NativeTuple for () {
    const SIZE: usize = 0;
    const TYPES: &[Type] = &[];

    unsafe fn from_raw(_ptr: *const RawVal) -> Self {
        ()
    }

    unsafe fn write_raw(self, _ptr: *mut RawVal) {}
}

impl TypeArray for () {
    fn type_ids(&self, _pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
        std::iter::empty()
    }
}

unsafe impl<T> NativeTuple for T
where
    T: NativeType,
{
    const SIZE: usize = Self::TYPES.len();
    const TYPES: &[Type] = &[T::TYPE];

    unsafe fn from_raw(ptr: *const RawVal) -> Self {
        unsafe { T::from_raw(&*ptr) }
    }

    unsafe fn write_raw(self, ptr: *mut RawVal) {
        unsafe {
            ptr.write(self.to_raw());
        }
    }
}

impl<T> TypeArray for T
where
    T: NativeType,
{
    fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
        std::iter::once(self.get_type_id(pool))
    }
}

macro_rules! impl_native_tuple {
    ($($t:ident),*) => {
        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        unsafe impl<$($t,)*> NativeTuple for ( $($t,)* )
        where
            $(
                $t: NativeType,
            )*
        {
            const TYPES: &[Type] = &[ $($t::TYPE,)* ];
            const SIZE: usize = Self::TYPES.len();

            unsafe fn from_raw(ptr: *const RawVal) -> Self {
                let mut offset = 0;
                $(
                    let $t = unsafe { $t::from_raw(&*ptr.add(offset)) };
                    offset += 1;
                )*

                ($($t,)*)
            }

            unsafe fn write_raw(self, ptr: *mut RawVal) {
                let ( $($t,)* ) = self;

                let mut offset = 0;
                $(
                    unsafe { ptr.add(offset).write($t.to_raw()); }
                    offset += 1;
                )*
            }
        }

        #[allow(non_snake_case)]
        impl<$($t,)*> TypeArray for ( $($t,)* )
        where
            $(
                $t: NativeType,
            )*
        {
            fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
                let ( $($t,)* ) = self;
                [
                    $(
                        $t.get_type_id(pool),
                    )*
                ]
                .into_iter()
            }
        }
    };
}

impl_native_tuple!(T0);
impl_native_tuple!(T0, T1);
impl_native_tuple!(T0, T1, T2);
impl_native_tuple!(T0, T1, T2, T3);
impl_native_tuple!(T0, T1, T2, T3, T4);
impl_native_tuple!(T0, T1, T2, T3, T4, T5);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_native_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum TypeRef {
    Concrete(TypeId),
    Null,
    /// Invalid type reference.
    Invalid,
}

trait TypeArray {
    fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef>;
}

impl TypeArray for [Val] {
    fn type_ids(&self, pool: &ObjectPool) -> impl Iterator<Item = TypeRef> {
        self.iter().map(|v| v.get_type_id(pool))
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    msg: String,
}

impl Error {
    pub fn msg<T>(msg: T) -> Self
    where
        T: ToString,
    {
        Self {
            msg: msg.to_string(),
        }
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error,
{
    fn from(value: E) -> Self {
        Self {
            msg: value.to_string(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.msg, f)
    }
}

macro_rules! try_call {
    ($ctx:expr, $($returns:ty),* => $f:expr) => {{
        match catch_unwind(AssertUnwindSafe(|| $f())) {
            Ok(v) => v,
            #[cfg(panic = "unwind")]
            Err(msg) => {
                let ctx = unsafe { &mut *$ctx.cast_mut() };
                ctx.panic.set(msg);
                // Value does not matter, except that it must be non-zero,
                // but we set it to a value that cannot be interpreted as any other status.
                (i32::MAX $(, MaybeUninit::<$returns>::zeroed())*)
            }
            // Cannot unwind when aborting.
            #[cfg(not(panic = "unwind"))]
            Err(_) => unreachable!(),
        }
    }};
}

use try_call;

struct Span {
    ctx: *mut RawContext,
    // The returned id must be dropped on the same thread,
    // i.e. we are not Send/Sync.
    id: usize,
}

impl Span {
    fn new(ctx: *mut RawContext, s: &str) -> Self {
        let id = enter_span(ctx, s.as_ptr(), s.len());

        Self { ctx, id }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        exit_span(self.ctx, self.id);
    }
}
