use std::any::Any;
use std::marker::PhantomData;

use openfo4_jit::{Signature, Type};

mod object;

pub type NativeFnPtr = unsafe extern "C" fn(*const HostState, usize, *mut RawVal);

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
    pub fn call(&self) {}

    pub unsafe fn call_unchecked(&self, state: &HostState, values: &mut [RawVal]) {
        unsafe {
            (self.ptr)(state, 0, values.as_mut_ptr());
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
    pub call_table: CallTable,
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
        #[allow(unused_parens)]
        #[allow(unused_mut)]
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

pub unsafe extern "C" fn call_host_trampoline(
    ctx: *const HostState,
    index: usize,
    args: *mut RawVal,
) {
    let ctx = unsafe { &*ctx };
    let func = &ctx.call_table.functions[index];
    func(ctx, args);
}
