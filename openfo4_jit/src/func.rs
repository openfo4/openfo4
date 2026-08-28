use std::fmt::{self, Debug, Formatter};
use std::hint::assert_unchecked;
use std::mem::MaybeUninit;
use std::sync::Arc;

pub trait IntoHostFunc<Params, Returns> {
    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn into_host_func(self) -> HostFunc;
}

macro_rules! impl_into_func {
    ($($params:ident),* => $($returns:ident),*) => {
        #[allow(unused_parens)]
        #[allow(nonstandard_style)]
        impl<F, $($params,)* $($returns,)*> IntoHostFunc<($($params),*), ($($returns),*)> for F
        where
            F: Fn($($params),*) -> ($($returns),*) + Send + Sync + 'static,
            $(
                $params: HostType,
            )*
            $(
                $returns: HostType,
            )*
        {
            #[allow(private_interfaces)]
            #[allow(unused)]
            fn into_host_func(self) -> HostFunc {
                HostFunc::new(move |params, mut returns| {
                    let mut params = params.cast_mut();

                    // Both pointers are always aligned to the maximum of all parameters.
                    // Guaranteeing this here allows all following alignment offsets
                    // to become constants.
                    $(
                        unsafe {
                            assert_unchecked(params.align_offset(align_of::<$params>()) == 0);
                        }
                    )*
                    $(
                        unsafe {
                            assert_unchecked(returns.align_offset(align_of::<$returns>()) == 0);
                        }
                    )*


                    $(
                        unsafe { align_up::<$params>(&mut params) };
                        let $params = unsafe { <$params>::read(params) };
                        unsafe { params = params.add($params::SIZE); }
                    )*

                    let ($($returns),*) = self($($params),*);

                    $(
                        unsafe {
                            align_up::<$returns>(&mut returns);
                            returns.cast::<$returns>().write($returns);
                            returns = returns.add($returns::SIZE);
                        }
                    )*
                })
            }
        }
    };
}

macro_rules! impl_into_func_params {
    ($($params:ident),*) => {
        impl_into_func!($($params),* =>);
        impl_into_func!($($params),* => R0);
        impl_into_func!($($params),* => R0, R1);
        impl_into_func!($($params),* => R0, R1, R2);
        impl_into_func!($($params),* => R0, R1, R2, R3);
    };
}

impl_into_func_params!();
impl_into_func_params!(A0);
impl_into_func_params!(A0, A1);
impl_into_func_params!(A0, A1, A2);
impl_into_func_params!(A0, A1, A2, A3);
impl_into_func_params!(A0, A1, A2, A3, A4);
impl_into_func_params!(A0, A1, A2, A3, A4, A5);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7, A8);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12);
impl_into_func_params!(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13);
impl_into_func_params!(
    A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14
);
impl_into_func_params!(
    A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15
);

#[derive(Clone)]
pub struct HostFunc {
    func: Arc<dyn Fn(*const u8, *mut u8) + Send + Sync + 'static>,
}

impl HostFunc {
    pub(crate) fn new<F>(func: F) -> Self
    where
        F: Fn(*const u8, *mut u8) + Send + Sync + 'static,
    {
        Self {
            func: Arc::new(func),
        }
    }

    /// Call the host function.
    ///
    /// # Safety
    ///
    /// - `params` and `returns` must both be aligned to [`MAX_ALIGN`].
    pub(crate) unsafe fn call(&self, params: *const u8, returns: *mut u8) {
        (self.func)(params, returns);
    }
}

impl Debug for HostFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostFunc")
            .field("func", &core::ptr::from_ref(&*self.func))
            .finish()
    }
}

impl IntoHostFunc<(), ()> for HostFunc {
    fn into_host_func(self) -> HostFunc {
        self
    }
}

pub unsafe trait HostType: Copy + Sized + 'static {
    const SIZE: usize = size_of::<Self>();
    const ALIGN: usize = align_of::<Self>();

    unsafe fn read(ptr: *const u8) -> Self {
        unsafe { ptr.cast::<Self>().read() }
    }
}

unsafe impl HostType for bool {}
unsafe impl HostType for u8 {}
unsafe impl HostType for u16 {}
unsafe impl HostType for u32 {}
unsafe impl HostType for u64 {}
unsafe impl HostType for usize {}
unsafe impl HostType for i8 {}
unsafe impl HostType for i16 {}
unsafe impl HostType for i32 {}
unsafe impl HostType for i64 {}
unsafe impl HostType for isize {}
unsafe impl HostType for f32 {}
unsafe impl HostType for f64 {}
unsafe impl<T: 'static> HostType for *const T {}
unsafe impl<T: 'static> HostType for *mut T {}

// `MaybeUninit` has the same layout as `T`.
// In some cases the callee may want to return an uninit value,
// however the caller is required to never read it.
unsafe impl<T> HostType for MaybeUninit<T> where T: HostType {}

unsafe fn align_up<T>(ptr: &mut *mut u8)
where
    T: HostType,
{
    let offset = ptr.align_offset(T::ALIGN);
    *ptr = unsafe { ptr.add(offset) };
}
