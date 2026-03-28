pub mod host_functions;
pub mod object;
mod stack;

use std::any::Any;
use std::marker::PhantomData;
use std::mem::{MaybeUninit, transmute};
use std::sync::Arc;

use game_common::cell::UnsafeRefCell;
use openfo4_jit::{Signature, Type};
use thiserror::Error;

pub use object::{ArrayObject, Object, ObjectAddr, ObjectPool, ObjectRef, StringObject};
pub use stack::Stack;

use crate::compiler::CompilationResult;
use crate::engine::{Engine, EngineShared};

pub(crate) const RESULT_OK: i32 = 0;
pub(crate) const RESULT_ERROR: i32 = 1;
pub(crate) const RESULT_SUSPEND: i32 = 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Exception {
    Error = RESULT_ERROR,
    Suspend = RESULT_SUSPEND,
}

pub type NativeFnPtr = unsafe extern "C" fn(*const RawContext, usize, *mut RawVal);

/// Context passed to guest functions.
pub struct Context<'a, T> {
    engine: Arc<EngineShared>,
    stack: &'a mut Stack,
    data: T,
}

impl<'a, T> Context<'a, T> {
    pub fn new(engine: &Engine, stack: &'a mut Stack, data: T) -> Self {
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
#[derive(Clone, Debug)]
#[repr(C)]
pub struct RawContext {
    pub stack_ptr: *mut u8,
    pub state: *const HostState,
    pub data: *mut dyn Any,
    pub stack: *mut Stack,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union RawVal {
    pub i8: i8,
    pub i16: i16,
    pub i32: i32,
    pub i64: i64,
    pub isize: isize,
    pub f32: f32,
    pub f64: f64,
    pub ptr: *mut u8,
}

#[derive(Copy, Clone)]
pub enum Val {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Isize(isize),
    F32(f32),
    F64(f64),
    Ptr(*mut u8),
}

impl Val {
    fn ty(&self) -> Type {
        match self {
            Self::I8(_) => Type::I8,
            Self::I16(_) => Type::I16,
            Self::I32(_) => Type::I32,
            Self::I64(_) => Type::I64,
            Self::F32(_) => Type::F32,
            Self::F64(_) => Type::F64,
            Self::Isize(_) | Self::Ptr(_) => Type::Pointer,
        }
    }

    fn into_raw(self) -> RawVal {
        match self {
            Self::I8(v) => RawVal { i8: v },
            Self::I16(v) => RawVal { i16: v },
            Self::I32(v) => RawVal { i32: v },
            Self::I64(v) => RawVal { i64: v },
            Self::F32(v) => RawVal { f32: v },
            Self::F64(v) => RawVal { f64: v },
            Self::Isize(v) => RawVal { isize: v },
            Self::Ptr(v) => RawVal { ptr: v },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    pub(crate) ptr: NativeFnPtr,
    pub(crate) signature: Signature,
    pub(crate) stack_size: usize,
}

impl Func {
    /// # Safety
    ///
    /// - The function pointer must point to a [`NativeFnPtr`] function.
    pub(crate) unsafe fn new(res: &CompilationResult) -> Self {
        // Should always match the ABI of `host_call_trampoline`.
        debug_assert_eq!(
            res.function.signature(),
            &Signature {
                params: vec![Type::Pointer, Type::Pointer, Type::Pointer],
                returns: vec![]
            }
        );

        Self {
            // SAFETY: The caller guarantees that the function is a valid
            // function pointer of type `NativeFnPtr`.
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(res.function.as_ptr()) },
            signature: res.signature.clone(),
            stack_size: res.stack_size,
        }
    }

    /// Returns the minimum stack size required to call this function.
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }

    pub fn call<T>(&self, ctx: &mut Context<'_, T>, params: &[Val]) -> Result<Vec<Val>, Exception>
    where
        T: 'static,
    {
        assert_eq!(self.signature.params.len() - 1, params.len());
        for (lhs, rhs) in self.signature.params[1..].iter().zip(params) {
            assert_eq!(*lhs, rhs.ty());
        }

        let mut values = vec![
            MaybeUninit::<RawVal>::uninit();
            1 + self.signature.params.len() + self.signature.returns.len()
        ];

        for (index, param) in params.iter().enumerate() {
            values[index].write(param.into_raw());
        }

        // SAFETY:
        // - We have validated that the given parameters are the same as the parameters
        //   of the function signature.
        // - We have filled the first N values with initialized values according to the
        //   function signature.
        unsafe {
            self.call_unchecked(ctx, values.as_mut_ptr().cast::<RawVal>());
        }

        let result = unsafe {
            &*values
                .as_ptr()
                .add(params.len() + self.signature.returns.len() - 1)
                .cast::<RawVal>()
        };

        let result = unsafe { <i32 as NativeType>::from_raw(result) };

        if result != RESULT_OK {
            // SAFETY: The result parameter is only created as RESULT_OK from inside
            // compiled functions. All other values are only produced by host functions
            // that return a Exception, so we are doing a type roundtrip.
            let exception = unsafe { transmute::<i32, Exception>(result) };
            return Err(exception);
        }

        let mut returns = Vec::with_capacity(self.signature.returns.len() - 1);
        for (index, ty) in self.signature.returns[..self.signature.returns.len() - 1]
            .iter()
            .enumerate()
        {
            let raw = unsafe { values[self.signature.params.len() + index].assume_init_ref() };

            unsafe {
                returns.push(match ty {
                    Type::I8 => Val::I8(<u8 as NativeType>::from_raw(raw) as i8),
                    Type::I16 => Val::I16(<u16 as NativeType>::from_raw(raw) as i16),
                    Type::I32 => Val::I32(<i32 as NativeType>::from_raw(raw)),
                    Type::I64 => todo!(),
                    Type::F32 => Val::F32(<f32 as NativeType>::from_raw(raw)),
                    Type::F64 => todo!(),
                    Type::Pointer => Val::Ptr(<isize as NativeType>::from_raw(raw) as *mut u8),
                });
            }
        }

        Ok(returns)
    }

    pub unsafe fn call_unchecked<T>(&self, ctx: &mut Context<'_, T>, values: *mut RawVal)
    where
        T: 'static,
    {
        assert!(ctx.stack.size() >= self.stack_size);

        // We must lock the call table before we call the guest function.
        let _lock = ctx.engine.call_table_lock.read();

        let ctx = RawContext {
            stack_ptr: ctx.stack.as_ptr(),
            state: &ctx.engine.state,
            data: &mut ctx.data as *mut dyn Any,
            stack: ctx.stack,
        };

        unsafe {
            (self.ptr)(&ctx, 0, values);
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
    pub fn call<T>(&self, ctx: &mut Context<'_, T>, params: Params) -> Result<Returns, Exception>
    where
        T: 'static,
    {
        // TODO: When const_generics is stabilized we can avoid the heap allocation
        // and use a regular array.
        let mut values = vec![MaybeUninit::<RawVal>::uninit(); Params::SIZE + Returns::SIZE + 1];

        unsafe {
            params.write_raw(values.as_mut_ptr().cast::<RawVal>());
        }

        // SAFETY:
        // - When we have constructed this `TypedFunc` we have asserted that the signature
        //   matches.
        // - We have filled the first N values with initialized values according to the
        //   function signature.
        unsafe {
            self.func
                .call_unchecked(ctx, values.as_mut_ptr().cast::<RawVal>());
        }

        let result = unsafe {
            &*values
                .as_ptr()
                .add(Params::SIZE + Returns::SIZE)
                .cast::<RawVal>()
        };

        let result = unsafe { <i32 as NativeType>::from_raw(result) };

        if result != RESULT_OK {
            // SAFETY: The result parameter is only created as RESULT_OK from inside
            // compiled functions. All other values are only produced by host functions
            // that return a Exception, so we are doing a type roundtrip.
            let exception = unsafe { transmute::<i32, Exception>(result) };
            return Err(exception);
        }

        unsafe {
            let returns = values.as_ptr().add(Params::SIZE).cast::<RawVal>();
            Ok(Returns::from_raw(returns))
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

    pub unsafe fn string(&self, addr: ObjectAddr) -> &str {
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

pub struct HostState {
    pub call_table: UnsafeRefCell<CallTable>,
    pub pool: ObjectPool,
}

#[derive(Default)]
pub struct CallTable {
    functions: Vec<Box<dyn Fn(*const RawContext, *mut RawVal) + Send + Sync>>,
}

impl CallTable {
    pub fn insert<F>(&mut self, func: F) -> usize
    where
        F: Fn(*const RawContext, *mut RawVal) + Send + Sync + 'static,
    {
        let index = self.functions.len();
        self.functions.push(Box::new(func));
        index
    }
}

pub trait IntoFunc<T, Params, Returns> {
    fn into_func(self, call_table: &mut CallTable) -> usize;

    fn signature() -> Signature;
}

macro_rules! impl_into_func {
    ($($params:ident),* => $($returns:ident),*) => {
        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F, T, $($params,)* $($returns,)*> IntoFunc<T, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'_, T>, $($params),*) -> Result<($($returns),*), Exception> + Send + Sync + 'static,
            T: 'static,
            $(
                $params: NativeType,
            )*
            $(
                $returns: NativeType,
            )*
        {
            fn into_func(self, call_table: &mut CallTable) -> usize {
                let callback = move |ctx: *const RawContext, args: *mut RawVal| {
                    let ctx = unsafe { &*ctx };

                    let mut index = 0;
                    $(
                        let $params = unsafe { $params::from_raw(&*args.add(index)) };
                        index += 1;
                    )*

                    let caller = Caller {
                        ctx,
                        _marker: PhantomData,
                    };


                    let ($($returns),*) = match self(caller, $($params),*) {
                        Ok(($($returns),*)) => {
                            $(
                                unsafe {
                                    args.add(index).write($returns.to_raw());
                                    index += 1;
                                }
                            )*

                            unsafe {
                                args.add(index).write(RESULT_OK.to_raw());
                            }
                        },
                        Err(err) => {
                            // Count all return values.
                            {
                                $(
                                    let $returns = 0;
                                    index += 1;
                                )*
                            }

                            // Write the error value.
                            // The other return values are left uninitialized.
                            unsafe {
                                args.add(index).write((err as i32).to_raw());
                            }
                        }
                    };
                };

                call_table.insert(callback)
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

pub unsafe trait NativeType: Copy {
    const TYPE: Type;

    unsafe fn from_raw(val: &RawVal) -> Self;

    fn to_raw(self) -> RawVal;
}

unsafe impl NativeType for u8 {
    const TYPE: Type = Type::I8;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.i8 as u8 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { i8: self as i8 }
    }
}

unsafe impl NativeType for u16 {
    const TYPE: Type = Type::I16;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.i16 as u16 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { i16: self as i16 }
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
}

unsafe impl NativeType for i32 {
    const TYPE: Type = Type::I32;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.i32 }
    }

    fn to_raw(self) -> RawVal {
        RawVal { i32: self }
    }
}

unsafe impl NativeType for isize {
    const TYPE: Type = Type::Pointer;

    unsafe fn from_raw(val: &RawVal) -> Self {
        unsafe { val.isize }
    }

    fn to_raw(self) -> RawVal {
        RawVal { isize: self }
    }
}

unsafe impl NativeType for bool {
    const TYPE: Type = Type::I32;

    unsafe fn from_raw(val: &RawVal) -> Self {
        let v = unsafe { <i32 as NativeType>::from_raw(val) };
        v != 0
    }

    fn to_raw(self) -> RawVal {
        (self as i32).to_raw()
    }
}

unsafe impl NativeType for ObjectAddr {
    const TYPE: Type = Type::Pointer;

    unsafe fn from_raw(val: &RawVal) -> Self {
        Self::from_raw(unsafe { val.ptr })
    }

    fn to_raw(self) -> RawVal {
        RawVal { ptr: self.to_raw() }
    }
}

pub unsafe trait NativeTuple {
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
