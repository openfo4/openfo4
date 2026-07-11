use std::pin::pin;
use std::sync::Arc;
use std::task::{self, Poll, Wake, Waker};

use futures_lite::future::yield_now;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::func::TypedFunc;
use openfo4_script::runtime::{Caller, Context, IntoFunc};
use parking_lot::Mutex;

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn suspend_async() {
    async fn yield_once(_caller: Caller<'_, ()>) {
        yield_now().await;
    }

    run_test(yield_once, |engine, func, mut cx| {
        let mut stack = engine.create_stack();
        let mut ctx = Context::new(&engine, &mut stack, false);

        let mut fut = pin!(func.call_async(&mut ctx, ()));

        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
    });
}

#[test]
fn drop_async_fn() {
    struct OnDrop(Arc<Mutex<bool>>);

    impl Drop for OnDrop {
        fn drop(&mut self) {
            *self.0.lock() = true;
        }
    }

    async fn f(caller: Caller<'_, Arc<Mutex<bool>>>) {
        let _on_drop = OnDrop(Arc::clone(&*caller.data()));
        yield_now().await;
    }

    run_test(f, |engine, func, mut cx| {
        let dropped = Arc::new(Mutex::new(false));

        let mut stack = engine.create_stack();
        let mut ctx = Context::new(&engine, &mut stack, Arc::clone(&dropped));

        let mut fut = pin!(func.call_async(&mut ctx, ()));

        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending);

        drop(stack);
        assert_eq!(*dropped.lock(), true);
    });
}

#[test]
fn async_fn_waker() {
    struct MyWaker(Arc<Mutex<bool>>);

    impl Wake for MyWaker {
        fn wake(self: Arc<Self>) {
            *self.0.lock() = true;
        }
    }

    async fn f(_caller: Caller<'_, ()>) {
        yield_now().await;
    }

    run_test(f, |engine, func, _| {
        let woken = Arc::new(Mutex::new(false));
        let waker = Waker::from(Arc::new(MyWaker(Arc::clone(&woken))));
        let mut cx = task::Context::from_waker(&waker);

        let mut stack = engine.create_stack();
        let mut ctx = Context::new(&engine, &mut stack, ());

        let fut = pin!(func.call_async(&mut ctx, ()));

        assert_eq!(fut.poll(&mut cx), Poll::Pending);

        assert_eq!(*woken.lock(), true);
    });
}

#[test]
fn async_fn_use_data_across_await_point() {
    async fn f(mut caller: Caller<'_, i32>) {
        let data = caller.data_mut();

        for _ in 0..3 {
            yield_now().await;
            *data += 1;
        }
    }

    run_test(f, |engine, func, mut cx| {
        let mut stack = engine.create_stack();
        let mut ctx = Context::new(&engine, &mut stack, 0);

        let mut fut = pin!(func.call_async(&mut ctx, ()));

        for _ in 0..3 {
            assert_eq!(fut.as_mut().poll(&mut cx,), Poll::Pending);
        }

        assert_eq!(fut.poll(&mut cx), Poll::Ready(Ok(())));
    });
}

fn run_test<T, F, M, U>(f: F, test: U)
where
    F: IntoFunc<T, M, (), ()>,
    U: for<'a> FnOnce(&Engine, TypedFunc<'a, (), ()>, task::Context<'_>),
{
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class test:

        function F() static native:

        class MyClass:

        function MyFunc() static:
        local nonevar: none
        nonevar = call_static test F
        return nonevar
        "#,
    );

    engine.register(&module);
    engine.register_native_function("test", "F", f).unwrap();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunc").unwrap();

    let func = func.typed::<(), ()>().unwrap();

    let waker = Waker::noop();
    let cx = task::Context::from_waker(waker);

    test(&engine, func, cx);
}
