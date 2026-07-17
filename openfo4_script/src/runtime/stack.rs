use std::alloc::Layout;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use crate::engine::EngineShared;

#[derive(Debug)]
pub struct Stack {
    pub(crate) engine: Arc<EngineShared>,
    /// The top-level guest function that created the callstack.
    ///
    /// If `None` the callstack is empty.
    pub(crate) func: Option<*const u8>,
    pub(crate) memory: Vec<u8>,
    len: usize,
    pub(crate) future: ReusableFutureBox,
    frames: Vec<Frame>,
}

impl Stack {
    pub(crate) fn new_with_engine(engine: Arc<EngineShared>) -> Self {
        Self {
            engine,
            func: None,
            memory: Vec::new(),
            len: 0,
            future: ReusableFutureBox::new(),
            frames: Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.len
    }

    pub fn as_ptr(&mut self) -> *mut u8 {
        self.memory.as_mut_ptr()
    }

    pub(crate) fn alloc(&mut self, size: usize) -> *mut u8 {
        let offset = self.memory.len();
        self.memory.resize(offset + size, 0);
        self.len += size;

        self.frames.push(Frame { len: size });

        self.memory.as_mut_ptr()
    }

    pub(crate) fn free(&mut self, size: usize) -> *mut u8 {
        let Some(frame) = self.frames.pop() else {
            panic!("invalid free: no frame allocated");
        };

        if frame.len != size {
            panic!(
                "invalid free: attempted to free {} bytes, top stack frame is {} bytes",
                size, frame.len
            );
        }

        self.len -= size;
        self.memory.as_mut_ptr()
    }
}

// SAFETY: All stack contents are plain-old-data types or pointers
// to objects that are allowed to be accessed from any thread.
unsafe impl Send for Stack {}

// SAFETY: Any access that reads or writes the Stack requires mutable access.
unsafe impl Sync for Stack {}

#[derive(Copy, Clone, Debug)]
struct Frame {
    len: usize,
}

/// A reusable allocation for dynamic futures.
#[derive(Debug)]
pub(crate) struct ReusableFutureBox {
    // If `None`, there is no future contained.
    vtable: Option<Vtable>,
    /// The data pointer to the future data.
    // When self.layout.size() == 0, then this is dangling.
    data: NonNull<()>,
    /// Layout of the allocation. If size == 0, then no allocation exists.
    layout: Layout,
}

impl ReusableFutureBox {
    pub fn new() -> Self {
        Self {
            vtable: None,
            data: NonNull::dangling(),
            layout: Layout::new::<()>(),
        }
    }

    /// Returns `true` if the box contains a future.
    pub fn is_init(&self) -> bool {
        self.vtable.is_some()
    }

    /// Places a new future in the box.
    pub fn set<T, F>(&mut self, f: F)
    where
        F: Future<Output = T>,
    {
        debug_assert!(self.vtable.is_none());

        unsafe fn poll_fn<T, F>(data: NonNull<()>, cx: &mut Context<'_>) -> Poll<()>
        where
            F: Future<Output = T>,
        {
            let stage = unsafe { data.cast::<Stage<T, F>>().as_mut() };

            let future = unsafe { &mut stage.future };

            let fut_pin: Pin<&mut F> = unsafe { Pin::new_unchecked(future) };
            let output = ready!(fut_pin.poll(cx));

            unsafe {
                ManuallyDrop::drop(future);
                stage.output = ManuallyDrop::new(output);
            }

            Poll::Ready(())
        }

        unsafe fn drop_fn<T, F>(data: NonNull<()>) {
            let data = unsafe { data.cast::<Stage<T, F>>().as_mut() };
            unsafe { ManuallyDrop::drop(&mut data.future) };
        }

        // Reuse current allocation if possible.
        let layout = Layout::new::<Stage<T, F>>();
        if layout.size() > self.layout.size() || layout.align() > self.layout.align() {
            if self.layout.size() != 0 {
                unsafe {
                    std::alloc::dealloc(self.data.as_ptr().cast::<u8>(), self.layout);
                }
            }

            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            self.layout = layout;
            self.data = NonNull::new(ptr).unwrap().cast::<()>();
        }

        self.vtable = Some(Vtable {
            poll: poll_fn::<T, F>,
            drop: drop_fn::<T, F>,
        });

        unsafe {
            self.data.cast::<Stage<T, F>>().write(Stage {
                future: ManuallyDrop::new(f),
            });
        }
    }

    /// Polls the future.
    ///
    /// If this function returns [`Ready`], the currently contained future is consumed.
    ///
    /// # Safety
    ///
    /// - The box must currently contain a future.
    /// - The `T` and `F` types must be compatible with the `T` and `F` types used during the last
    ///   [`set`] call.
    ///
    /// [`Ready`]: Poll::Ready
    /// [`set`]: Self::set
    pub unsafe fn poll<T, F>(&mut self, cx: &mut Context<'_>) -> Poll<T>
    where
        F: Future<Output = T>,
    {
        // SAFETY: The caller guarantees that we have a future currently.
        let vtable = unsafe { self.vtable.unwrap_unchecked() };

        // SAFETY: The caller guarantees that:
        // - T and F are the types used to construct the last future.
        // - The future has no not yet yielded Poll::Ready.
        unsafe { ready!((vtable.poll)(self.data, cx)) };

        self.vtable = None;

        unsafe {
            let stage = self.data.cast::<Stage<T, F>>().as_mut();
            let output = ManuallyDrop::take(&mut stage.output);
            Poll::Ready(output)
        }
    }

    /// Drops the current future without deallocating the backing memory.
    pub fn clear(&mut self) {
        if let Some(vtable) = self.vtable.take() {
            unsafe {
                (vtable.drop)(self.data);
            }
        }
    }
}

impl Drop for ReusableFutureBox {
    fn drop(&mut self) {
        self.clear();

        if self.layout.size() != 0 {
            unsafe {
                std::alloc::dealloc(self.data.as_ptr().cast::<u8>(), self.layout);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Vtable {
    poll: unsafe fn(NonNull<()>, cx: &mut Context<'_>) -> Poll<()>,
    drop: unsafe fn(NonNull<()>),
}

union Stage<T, F> {
    output: ManuallyDrop<T>,
    future: ManuallyDrop<F>,
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    use futures_lite::future::{YieldNow, yield_now};

    use crate::runtime::stack::ReusableFutureBox;

    #[test]
    fn reusable_future_box_drop_empty() {
        let b = ReusableFutureBox::new();
        drop(b);
    }

    #[test]
    fn reusable_future_box_set_and_drop() {
        let mut b = ReusableFutureBox::new();
        b.set(yield_now());
        drop(b);
    }

    #[test]
    fn reusable_future_box_poll_pending_drop() {
        let mut b = ReusableFutureBox::new();
        b.set(yield_now());

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let res = unsafe { b.poll::<(), YieldNow>(&mut cx) };
        assert_eq!(res, Poll::Pending);

        drop(b);
    }

    #[test]
    fn reusable_future_box_poll_ready_drop() {
        let mut b = ReusableFutureBox::new();
        b.set(yield_now());

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let res = unsafe { b.poll::<(), YieldNow>(&mut cx) };
        assert_eq!(res, Poll::Pending);

        let res = unsafe { b.poll::<(), YieldNow>(&mut cx) };
        assert_eq!(res, Poll::Ready(()));

        assert!(!b.is_init());

        drop(b);
    }

    #[test]
    fn reusable_future_box_grow_size() {
        struct SmallFut(u8);

        impl Future for SmallFut {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                let _ = self.0;
                Poll::Ready(())
            }
        }

        struct BigFut([u8; 256]);

        impl Future for BigFut {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                let _ = self.0;
                Poll::Ready(())
            }
        }

        let mut b = ReusableFutureBox::new();
        b.set(SmallFut(0));

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let res = unsafe { b.poll::<(), SmallFut>(&mut cx) };
        assert_eq!(res, Poll::Ready(()));

        b.set(BigFut([0; 256]));

        let res = unsafe { b.poll::<(), BigFut>(&mut cx) };
        assert_eq!(res, Poll::Ready(()));
    }

    #[test]
    fn reusable_future_box_grow_align() {
        #[repr(align(8))]
        struct SmallAlignedFut([u8; 256]);

        impl Future for SmallAlignedFut {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                let _ = self.0;
                Poll::Ready(())
            }
        }

        #[repr(align(256))]
        struct BigAlignedFut([u8; 256]);

        impl Future for BigAlignedFut {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                let _ = self.0;
                Poll::Ready(())
            }
        }

        let mut b = ReusableFutureBox::new();
        b.set(SmallAlignedFut([0; 256]));

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let res = unsafe { b.poll::<(), SmallAlignedFut>(&mut cx) };
        assert_eq!(res, Poll::Ready(()));

        b.set(BigAlignedFut([0; 256]));

        let res = unsafe { b.poll::<(), BigAlignedFut>(&mut cx) };
        assert_eq!(res, Poll::Ready(()));
    }

    #[test]
    fn reusable_future_box_zero_sized_future() {
        struct Fut;

        impl Future for Fut {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        let mut b = ReusableFutureBox::new();
        b.set(Fut);

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let res = unsafe { b.poll::<(), Fut>(&mut cx) };
        assert_eq!(res, Poll::Ready(()));
    }
}
