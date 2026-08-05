use std::alloc::Layout;
use std::fmt::{self, Debug, Formatter};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use allocator_api2::alloc::{Allocator, Global};

use crate::collections::IsZst;

/// A pointer to a heap allocation that is aligned to a runtime value.
///
/// This type is equivalent to a [`Box`], except that the underlying heap allocation can be aligned
/// to a custom alignment.
pub struct AlignedBox<T, A: Allocator = Global>
where
    T: ?Sized,
    A: Allocator,
{
    ptr: NonNull<T>,
    // This is only `ManuallyDrop` so that we can take out the allocator,
    // without calling `Drop` and deallocating our pointer.
    alloc: ManuallyDrop<A>,
    align: usize,
}

impl<T> AlignedBox<T> {
    /// Creates a new box with the given value.
    ///
    /// The allocation is aligned up to the next multiple of `align`. If `T` already requires a
    /// larger alignment, `align` is ignored.
    pub fn new(value: T, align: usize) -> Self {
        Self::new_in(value, align, Global)
    }
}

impl<T, A> AlignedBox<T, A>
where
    A: Allocator,
{
    /// Creates a new box with the given value.
    ///
    /// The allocation is aligned up to the next multiple of `align`. If `T` already requires a
    /// larger alignment, `align` is ignored.
    pub fn new_in(value: T, align: usize, alloc: A) -> Self {
        let mut boxed = Self::new_uninit_in(align, alloc);
        boxed.write(value);

        // SAFETY: We have just written the value.
        unsafe { boxed.assume_init() }
    }

    /// Creates a new box with uninitialized contents.
    ///
    /// The allocation is aligned up to the next multiple of `align`. If `T` already requires a
    /// larger alignment, `align` is ignored.
    pub fn new_uninit_in(align: usize, alloc: A) -> AlignedBox<MaybeUninit<T>, A> {
        let layout = Layout::new::<T>().align_to(align).unwrap();

        let ptr = alloc.allocate(layout).unwrap().cast::<MaybeUninit<T>>();

        AlignedBox {
            ptr,
            alloc: ManuallyDrop::new(alloc),
            align,
        }
    }
}

impl<T, A> AlignedBox<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Returns a pointer to the allocated value.
    ///
    /// The pointer is guaranteed to be a multiple of the `align` used during the creation of this
    /// `AlignedBox`.
    pub fn as_ptr(this: &Self) -> *const T {
        this.ptr.as_ptr()
    }

    /// Returns a mutable pointer to the allocated value.
    ///
    /// The pointer is guaranteed to be a multiple of the `align` used during the creation of this
    /// `AlignedBox`.
    pub fn as_mut_ptr(this: &mut Self) -> *mut T {
        this.ptr.as_ptr()
    }

    fn into_raw_parts(mut self) -> (NonNull<T>, usize, A) {
        let ptr = self.ptr;
        let align = self.align;
        let alloc = unsafe { ManuallyDrop::take(&mut self.alloc) };
        core::mem::forget(self);
        (ptr, align, alloc)
    }

    unsafe fn from_raw_parts(ptr: NonNull<T>, align: usize, alloc: A) -> Self {
        Self {
            ptr,
            align,
            alloc: ManuallyDrop::new(alloc),
        }
    }
}

impl<T, A> AlignedBox<MaybeUninit<T>, A>
where
    A: Allocator,
{
    /// Converts to `AlignedBox<T, A>`, assuming that the value is initialized.
    ///
    /// # Safety
    ///
    /// The contained value must be initialized, see [`MaybeUninit::assume_init`].
    pub unsafe fn assume_init(self) -> AlignedBox<T, A> {
        unsafe {
            let (ptr, align, alloc) = self.into_raw_parts();
            let ptr = ptr.cast::<T>();
            AlignedBox::from_raw_parts(ptr, align, alloc)
        }
    }

    fn alloc_new(layout: Layout, align: usize, alloc: A) -> Self {
        let layout = layout.align_to(align).unwrap();

        let ptr = if T::IS_ZST {
            layout.dangling_ptr()
        } else {
            alloc.allocate(layout).unwrap().cast()
        };

        Self {
            ptr: ptr.cast::<MaybeUninit<T>>(),
            align,
            alloc: ManuallyDrop::new(alloc),
        }
    }
}

impl<T> AlignedBox<[T]> {
    /// Creates a new boxed slice with uninitialized contents.
    ///
    /// The allocation is aligned up to the next multiple of `align`. If `T` already requires a
    /// larger alignment, `align` is ignored.
    pub fn new_uninit_slice(len: usize, align: usize) -> AlignedBox<[MaybeUninit<T>]> {
        Self::new_uninit_slice_in(len, align, Global)
    }
}

impl<T, A> AlignedBox<[T], A>
where
    A: Allocator,
{
    /// Creates a new boxed slice with uninitialized contents.
    ///
    /// The allocation is aligned up to the next multiple of `align`. If `T` already requires a
    /// larger alignment, `align` is ignored.
    pub fn new_uninit_slice_in(
        len: usize,
        align: usize,
        alloc: A,
    ) -> AlignedBox<[MaybeUninit<T>], A> {
        let layout = Layout::array::<T>(len).unwrap();
        let boxed = AlignedBox::<MaybeUninit<T>, A>::alloc_new(layout, align, alloc);
        let (ptr, align, alloc) = boxed.into_raw_parts();

        let ptr = NonNull::slice_from_raw_parts(ptr, len);

        unsafe { AlignedBox::from_raw_parts(ptr, align, alloc) }
    }
}

impl<T, A> AlignedBox<[MaybeUninit<T>], A>
where
    A: Allocator,
    T: Sized,
{
    /// Converts to `AlignedBox<[T]>`, assuming that all values are initialized.
    ///
    /// # Safety
    ///
    /// All values of the slice must be initialized, see [`MaybeUninit::assume_init`].
    pub unsafe fn assume_init(self) -> AlignedBox<[T], A> {
        let (ptr, align, alloc) = self.into_raw_parts();
        let ptr: NonNull<[T]> = NonNull::slice_from_raw_parts(ptr.cast(), ptr.len());
        unsafe { AlignedBox::from_raw_parts(ptr, align, alloc) }
    }
}

impl<T, A> Clone for AlignedBox<T, A>
where
    T: Clone,
    A: Allocator + Clone,
{
    fn clone(&self) -> Self {
        let value: T = self.deref().clone();
        let alloc: A = self.alloc.deref().clone();
        Self::new_in(value, self.align, alloc)
    }
}

impl<T, A> Clone for AlignedBox<[T], A>
where
    T: Clone,
    A: Allocator + Clone,
{
    fn clone(&self) -> Self {
        let alloc: A = self.alloc.deref().clone();
        let mut boxed = Self::new_uninit_slice_in(self.len(), self.align, alloc);

        for index in 0..self.len() {
            boxed[index].write(self[index].clone());
        }

        // SAFETY: We have initialized every element of the slice.
        unsafe { boxed.assume_init() }
    }
}

impl<T, A> Deref for AlignedBox<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T, A> DerefMut for AlignedBox<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

unsafe impl<T, A> Send for AlignedBox<T, A>
where
    T: ?Sized + Send,
    A: Allocator + Send,
{
}
unsafe impl<T, A> Sync for AlignedBox<T, A>
where
    T: ?Sized + Sync,
    A: Allocator + Sync,
{
}

impl<T, A> Drop for AlignedBox<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    fn drop(&mut self) {
        let value: &T = self;
        let layout = Layout::for_value(value).align_to(self.align).unwrap();

        unsafe {
            self.ptr.drop_in_place();
        }

        if layout.size() != 0 {
            unsafe {
                self.alloc.deallocate(self.ptr.cast(), layout);
            }
        }

        unsafe {
            ManuallyDrop::drop(&mut self.alloc);
        }
    }
}

impl<T, A> Debug for AlignedBox<T, A>
where
    T: ?Sized + Debug,
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::aligned_box::AlignedBox;

    #[test]
    fn aligned_box() {
        let contents = String::from("Hello World!");
        let boxed = AlignedBox::new(contents.clone(), 256);
        assert_eq!(AlignedBox::as_ptr(&boxed).align_offset(256), 0);
        assert_eq!(*boxed, contents);
    }

    #[test]
    fn aligned_box_uninit_slice() {
        let mut boxed = AlignedBox::<[String]>::new_uninit_slice(3, 256);
        boxed[0].write(String::from("A"));
        boxed[1].write(String::from("B"));
        boxed[2].write(String::from("C"));

        let boxed = unsafe { boxed.assume_init() };

        assert_eq!(boxed[0], "A");
        assert_eq!(boxed[1], "B");
        assert_eq!(boxed[2], "C");
    }

    #[test]
    fn aligned_box_slice_clone() {
        let mut boxed = AlignedBox::<[i32]>::new_uninit_slice(3, 256);
        boxed[0].write(0);
        boxed[1].write(1);
        boxed[2].write(2);

        let boxed = unsafe { boxed.assume_init() };
        let other = boxed.clone();

        assert_eq!(boxed[0], other[0]);
        assert_eq!(boxed[1], other[1]);
        assert_eq!(boxed[2], other[2]);
    }
}
