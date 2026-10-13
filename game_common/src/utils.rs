pub mod exclusive;
pub mod vec_ext;

mod duration;

use std::fmt::{Arguments, Debug};
use std::pin::Pin;
use std::range::Range;
use std::task::{Context, Poll};

use glam::{Quat, Vec2, Vec3, Vec4};
use num_traits::Float;

pub use duration::DurationExt;
use pin_project::{pin_project, pinned_drop};

/// Floating-point approximate equality comparssion.
///
/// This will expand to the following check:
/// ```
/// # const EPSILON: f32 = f32::EPSILON;
/// #
/// # fn approx_eq(lhs: f32, rhs: f32) -> bool {
/// (lhs - rhs).abs() <= EPSILON
/// # }
/// ```
#[macro_export]
macro_rules! assert_approx_eq {
    ($lhs:expr, $rhs:expr, $epsilon:expr $(,)?) => {{
        $crate::utils::approx_eq_assertion_eps($lhs, $rhs, $epsilon, ::core::option::Option::None);
    }};
    ($lhs:expr, $rhs:expr, $epsilon:expr, $($arg:tt)+) => {{
        $crate::utils::approx_eq_assertion_eps($lhs, $rhs, $epsilon, ::core::option::Option::Some(
            ::core::format_args!($($arg)+)
        ));
    }};
    // Note that we're going through a function so rustc can infer what
    // type `$lhs` and `$rhs` is. This is necessary to get the epsilon value
    // of the type.
    ($lhs:expr, $rhs:expr $(,)?) => {{
        $crate::utils::approx_eq_assertion($lhs, $rhs, ::core::option::Option::None);
    }};
    ($lhs:expr, $rhs:expr, $($arg:tt)+) => {{
        $crate::utils::approx_eq_assertion($lhs, $rhs, ::core::option::Option::Some(
            ::core::format_args!($($arg)+)
        ));
    }};
}

pub trait ApproxFloatEq {
    type Primitive: Float;
    type ElementIter: Iterator<Item = Self::Primitive>;

    fn elements(&self) -> Self::ElementIter;
}

impl ApproxFloatEq for f32 {
    type Primitive = f32;
    type ElementIter = F32Iter;

    fn elements(&self) -> Self::ElementIter {
        F32Iter { value: Some(*self) }
    }
}

impl ApproxFloatEq for f64 {
    type Primitive = f64;
    type ElementIter = F64Iter;

    fn elements(&self) -> Self::ElementIter {
        F64Iter { value: Some(*self) }
    }
}

pub struct F32Iter {
    value: Option<f32>,
}

impl Iterator for F32Iter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.take()
    }
}

pub struct F64Iter {
    value: Option<f64>,
}

impl Iterator for F64Iter {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.take()
    }
}

macro_rules! impl_vec {
    ($vec:ty,$iter:ident, $dim:expr) => {
        impl ApproxFloatEq for $vec {
            type Primitive = f32;
            type ElementIter = $iter;

            fn elements(&self) -> $iter {
                $iter {
                    values: self.to_array(),
                    index: 0,
                }
            }
        }

        pub struct $iter {
            values: [f32; $dim],
            index: usize,
        }

        impl Iterator for $iter {
            type Item = f32;

            fn next(&mut self) -> Option<Self::Item> {
                let val = *self.values.get(self.index)?;
                self.index += 1;
                Some(val)
            }
        }
    };
}

impl_vec!(Vec2, Vec2Iter, 2);
impl_vec!(Vec3, Vec3Iter, 3);
impl_vec!(Vec4, Vec4Iter, 4);
impl_vec!(Quat, QuatIter, 4);

#[track_caller]
#[doc(hidden)]
pub fn approx_eq_assertion<T>(lhs: T, rhs: T, args: Option<Arguments<'_>>)
where
    T: ApproxFloatEq + Debug,
{
    for (lhs_elem, rhs_elem) in lhs.elements().zip(rhs.elements()) {
        if (lhs_elem - rhs_elem).abs() > <T::Primitive as Float>::epsilon() {
            approx_eq_assertion_failed(lhs, rhs, args);
        }
    }
}

#[track_caller]
#[doc(hidden)]
pub fn approx_eq_assertion_eps<T, P>(lhs: T, rhs: T, eps: P, args: Option<Arguments<'_>>)
where
    T: ApproxFloatEq<Primitive = P> + Debug,
    P: Float,
{
    for (lhs_elem, rhs_elem) in lhs.elements().zip(rhs.elements()) {
        if (lhs_elem - rhs_elem).abs() > eps {
            approx_eq_assertion_failed(lhs, rhs, args);
        }
    }
}

#[track_caller]
#[doc(hidden)]
fn approx_eq_assertion_failed<T>(lhs: T, rhs: T, args: Option<Arguments<'_>>) -> !
where
    T: Debug,
{
    match args {
        Some(args) => {
            panic!(
                r#"assertion failed: `(left == right)`
                left: `{:?}`
                right: `{:?}`: {}
                "#,
                lhs, rhs, args,
            );
        }
        None => {
            panic!(
                r#"assertion failed: `(left == right)`
                left: `{:?}`
                right: `{:?}`
                "#,
                lhs, rhs
            );
        }
    }
}

pub trait RangeExt {
    /// Returns `true` if the two ranges are overlapping.
    fn is_overlapping(&self, other: &Self) -> bool;
}

impl<T> RangeExt for Range<T>
where
    T: Ord,
{
    fn is_overlapping(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

// FIXME: The same functionality is provided by the nightly feature uint_gather_scatter_bits.
// We can remove this once https://github.com/rust-lang/rust/issues/149069 is stable.
pub trait ExtractBits {
    /// Extracts all bits from `a` that are set in `mask` and moves them into a contigous region.
    ///
    /// This can be used to create an integer from a non-contigous bitmask. This is the same operation
    /// as the x86 `pext_u32` instruction.
    ///
    /// See also:
    /// - https://www.felixcloutier.com/x86/pext
    ///
    fn extract_bits_(self, mask: Self) -> Self;
}

macro_rules! impl_extract_bits {
    ($t:ty) => {
        impl ExtractBits for $t {
            fn extract_bits_(self, mask: Self) -> Self {
                cfg_select! {
                    all(target_arch= "x86_64", target_feature = "bmi2") => {
                        // Since we are zero-extending our mask, the result will always fit
                        // within our integer type.
                        // SAFETY: We just asserted that the bmi2 feature is enabled.
                        unsafe { core::arch::x86_64::_pext_u64(self as u64, mask as u64) as $t }
                    }
                    _ => {
                        let src = self & mask;
                        let mut dst = 0;

                        let mut next_bit = 0;
                        for index in 0..Self::BITS {
                            let bit = (src & (1 << index)) >> index;
                            let mask_bit = (mask & (1 << index)) >> index;
                            dst |= bit << next_bit;
                            next_bit += mask_bit;
                        }
                        dst
                    }
                }
            }
        }
    };
}

impl_extract_bits!(u8);
impl_extract_bits!(u16);
impl_extract_bits!(u32);
impl_extract_bits!(u64);
impl_extract_bits!(usize);

/// Wrapper for a [`Future`] that runs the given closure when it is dropped before becoming ready.
///
/// When `F` returns [`Poll::Ready`] the closure is dropped without being executed.
#[pin_project(PinnedDrop)]
pub struct AbortOnDrop<F, A>
where
    F: Future,
    A: FnOnce(),
{
    #[pin]
    future: F,
    abort: Option<A>,
}

impl<F, A> AbortOnDrop<F, A>
where
    F: Future,
    A: FnOnce(),
{
    /// Creates a new `AbortOnDrop` wrapping the given [`Future`] `F` and abort closure `A`.
    pub fn new(future: F, abort: A) -> Self {
        Self {
            future,
            abort: Some(abort),
        }
    }
}

impl<F, A> Future for AbortOnDrop<F, A>
where
    F: Future,
    A: FnOnce(),
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => {
                *this.abort = None;
                Poll::Ready(val)
            }
        }
    }
}

#[pinned_drop]
impl<F, A> PinnedDrop for AbortOnDrop<F, A>
where
    F: Future,
    A: FnOnce(),
{
    fn drop(self: Pin<&mut Self>) {
        if let Some(abort) = self.project().abort.take() {
            abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExtractBits;

    #[test]
    fn assert_approx_eq() {
        let lhs = (-1.0) + ((1.0 - -1.0) * 400.0 / 1000.0);
        assert_approx_eq!(lhs, -0.2);
    }

    #[test]
    #[should_panic]
    fn assert_approx_eq_failure() {
        let lhs = (-1.0) + ((1.0 - -1.0) * 400.0 / 1000.0);
        assert_approx_eq!(lhs, -0.2 - f64::EPSILON);
    }

    // #[test]
    // fn assert_approx_eq_args() {
    //     assert_approx_eq!(1.0, 1.0, "oh no!");
    // }

    #[test]
    fn extract_bits() {
        let a: u8 = 0b11_11_11_11;
        let b: u8 = 0b00_00_00_00;
        let c: u8 = 0b00_00_00_00;
        assert_eq!(a.extract_bits_(b), c);

        let a: u8 = 0b10_11_01_00;
        let b: u8 = 0b11_11_11_11;
        let c: u8 = 0b10_11_01_00;
        assert_eq!(a.extract_bits_(b), c);

        let a: u32 = 0b01_00_00_10_01_10_00_00_11_00_01_01_01_01_00_01;
        let b: u32 = 0b01_11_01_11_10_01_01_10_01_11_01_00_11_01_10_00;
        let c: u32 = 0b00_00_00_00_00_00_00_10_00_10_00_00_10_01_01_10;
        assert_eq!(a.extract_bits_(b), c);
    }
}
