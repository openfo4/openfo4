use std::any::Any;
use std::marker::PhantomData;
use std::mem::{MaybeUninit, transmute};
use std::sync::Arc;

use openfo4_jit::{CompiledFunction, Signature, Type};
use thiserror::Error;

use crate::{Engine, EngineShared};

mod object;

pub type NativeFnPtr = unsafe extern "C" fn(*const HostState, usize, *mut RawVal);

pub struct Context<T> {
    engine: Arc<EngineShared>,
    data: T,
}

impl<T> Context<T> {
    pub fn new(engine: &Engine, data: T) -> Self {
        Self {
            engine: engine.shared.clone(),
            data,
        }
    }
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
    pub ptr: *const u8,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub ptr: NativeFnPtr,
    pub signature: Signature,
}

impl Func {
    /// # Safety
    ///
    /// - The function pointer must point to a [`NativeFnPtr`] function.
    pub(crate) unsafe fn new(f: &CompiledFunction) -> Self {
        Self {
            // SAFETY: The caller guarantees that the function is a valid
            // function pointer of type `NativeFnPtr`.
            ptr: unsafe { transmute::<*const u8, NativeFnPtr>(f.as_ptr()) },
            signature: f.signature().clone(),
        }
    }

    pub fn call(&self) {}

    pub unsafe fn call_unchecked<T>(&self, ctx: &Context<T>, values: *mut RawVal) {
        let _lock = ctx.engine.state_lock.read();
        let state = unsafe { ctx.engine.state.borrow() };

        unsafe {
            (self.ptr)(&*state, 0, values);
        }
    }

    pub fn typed<Params, Returns>(
        &self,
    ) -> Result<TypedFunc<'_, Params, Returns>, FuncSignatureMismatch>
    where
        Params: NativeTuple,
        Returns: NativeTuple,
    {
        if &self.signature.params[1..] != Params::TYPES || &self.signature.returns != Returns::TYPES
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
    pub fn call<T>(&self, ctx: &Context<T>, params: Params) -> Returns {
        // TODO: When const_generics is stabilized we can avoid the heap allocation
        // and use a regular array.
        let mut values = vec![MaybeUninit::<RawVal>::uninit(); Params::SIZE + Returns::SIZE];

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

        unsafe {
            let returns = values.as_ptr().add(Params::SIZE).cast::<RawVal>();
            Returns::from_raw(returns)
        }
    }
}

pub struct Caller<'a, T>
where
    T: 'static,
{
    state: &'a HostState,
    _marker: PhantomData<T>,
}

impl<'a, T> Caller<'a, T> {
    pub fn data(&self) -> &T {
        self.state
            .data
            .as_ref()
            .unwrap()
            .downcast_ref::<T>()
            .unwrap()
    }
}

#[derive(Default)]
pub struct HostState {
    data: Option<Box<dyn Any>>,
    call_table: CallTable,
}

#[derive(Default)]
pub struct CallTable {
    functions: Vec<Box<dyn Fn(*const HostState, *mut RawVal)>>,
}

impl CallTable {
    pub fn insert<F>(&mut self, func: F) -> usize
    where
        F: Fn(*const HostState, *mut RawVal) + Send + Sync + 'static,
    {
        let index = self.functions.len();
        self.functions.push(Box::new(func));
        index
    }
}

pub trait IntoFunc<T, Params, Returns> {
    fn into_func(self, ctx: &mut HostState) -> usize;

    fn signature() -> Signature;
}

macro_rules! impl_into_func {
    ($($params:ident),* => $($returns:ident),*) => {
        #[allow(unused_assignments)]
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F, T, $($params,)* $($returns,)*> IntoFunc<T, ($($params,)*), ($($returns,)*)> for F
        where
            F: Fn(Caller<'_, T>, $($params),*) -> ($($returns),*) + Send + Sync + 'static,
            T: 'static,
            $(
                $params: NativeType,
            )*
            $(
                $returns: NativeType,
            )*
        {
            fn into_func(self, ctx: &mut HostState) -> usize {
                let callback = move |ctx: *const HostState, args: *mut RawVal| {
                    let ctx = unsafe { &*ctx };

                    let mut index = 0;
                    $(
                        let $params = unsafe { $params::from_raw(&*args.add(index)) };
                        index += 1;
                    )*

                    let caller = Caller {
                        state: ctx,
                        _marker: PhantomData,
                    };

                    let ($($returns),*) = self(caller, $($params),*);

                    $(
                        unsafe {
                            args.add(index).write($returns.to_raw());
                            index += 1;
                        }
                    )*

                };

                ctx.call_table.insert(callback)
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

pub unsafe extern "C" fn call_host_trampoline(
    ctx: *const HostState,
    index: usize,
    args: *mut RawVal,
) {
    let ctx = unsafe { &*ctx };
    let func = &ctx.call_table.functions[index];
    func(ctx, args);
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
