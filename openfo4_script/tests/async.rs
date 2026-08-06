use std::assert_matches;
use std::pin::pin;
use std::sync::Arc;
use std::task::{self, Poll, Wake, Waker};

use futures_lite::future::{block_on, yield_now};
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::func::TypedFunc;
use openfo4_script::runtime::{Caller, Context, IntoFunc, ObjectAddr};
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

        assert_matches!(fut.as_mut().poll(&mut cx), Poll::Pending);
        assert_matches!(fut.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
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

        {
            let mut fut = pin!(func.call_async(&mut ctx, ()));
            assert_matches!(fut.as_mut().poll(&mut cx), Poll::Pending);
        }

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

        assert_matches!(fut.poll(&mut cx), Poll::Pending);

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
            assert_matches!(fut.as_mut().poll(&mut cx,), Poll::Pending);
        }

        assert_matches!(fut.poll(&mut cx), Poll::Ready(Ok(())));
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

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunc").unwrap();

    let func = func.typed::<(), ()>().unwrap();

    let waker = Waker::noop();
    let cx = task::Context::from_waker(waker);

    test(&engine, func, cx);
}

#[test]
fn suspend_in_callstack() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    async fn f(mut caller: Caller<'_, i32>) -> i32 {
        let data = caller.data_mut();

        for _ in 0..3 {
            dbg!("call");
            yield_now().await;
            *data += 1;
        }

        *data
    }

    let module = assemble(
        r#"
        class test:

        function F() -> int static native:

        class A:

        function X() -> int:
        local res: int
        res = call_static test F
        return res

        class B:

        function Y() -> int:
        local res: int
        res = call_static test F
        return res

        class C:

        function Z(a: A, b: B) -> int:
        local x: int
        local y: int
        local z: int
        x = call_method X a
        y = call_method Y b
        z = iadd x y
        return z
        "#,
    );

    engine.register(&module);
    engine.register_native_function("test", "F", f).unwrap();

    block_on(engine.compile("A", &Options::default())).unwrap();
    block_on(engine.compile("B", &Options::default())).unwrap();
    let instance = block_on(engine.compile("C", &Options::default())).unwrap();
    let func = instance.get_func("Z").unwrap();

    let a = engine.get_type("A").unwrap();
    let b = engine.get_type("B").unwrap();
    let c = engine.get_type("C").unwrap();

    let a = engine.object_pool().insert(a).finalize();
    let b = engine.object_pool().insert(b).finalize();
    let c = engine.object_pool().insert(c).finalize();

    let func = func
        .typed::<(ObjectAddr, ObjectAddr, ObjectAddr), i32>()
        .unwrap();

    let waker = Waker::noop();
    let mut cx = task::Context::from_waker(waker);

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, 0);

    let mut fut = pin!(func.call_async(&mut ctx, (c.addr(), a.addr(), b.addr())));
    for _ in 0..6 {
        assert_matches!(fut.as_mut().poll(&mut cx), Poll::Pending);
    }
    assert_matches!(fut.poll(&mut cx), Poll::Ready(Ok(9)));
}
