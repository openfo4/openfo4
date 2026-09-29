use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::compiler::resources::TypeId;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Caller, Context, ObjectAddr, Val};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
#[should_panic]
fn call_with_incorrect_object_type() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        function F(a: A) static:
        local nonevar: none
        return nonevar

        class B:
        "#,
    );

    engine.register(&module);

    block_on(engine.compile("B", &Options::default())).unwrap();
    let b = engine.get_type("B").unwrap();

    let a = block_on(engine.compile("A", &Options::default())).unwrap();
    let func = a.get_func("F").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let b = engine.object_pool().insert(b).finalize().addr();

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, b).now_or_never().unwrap();
}

#[test]
#[should_panic]
fn call_null_object() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:
        function F(a: A):
        "#,
    );

    engine.register(&module);

    let instance = block_on(engine.compile("A", &Options::default())).unwrap();

    let func = instance.get_func("F").unwrap();
    let func = func.typed::<ObjectAddr, ()>().unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    func.call(&mut ctx, ObjectAddr::NULL)
        .now_or_never()
        .unwrap();
}

#[test]
#[should_panic]
fn return_wrong_type_from_native_func() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    struct State {
        a: TypeId,
    }

    fn f(caller: Caller<'_, State>) -> ObjectAddr {
        caller.object_pool().insert(caller.data().a).addr()
    }

    let module = assemble(
        r#"
        class test:

        function F() -> B static native:

        class A:

        class B:

        function MyFunc() static:
        local res: B
        local nonevar: none
        res = call_static test F
        return nonevar
        "#,
    );

    engine.register(&module);
    engine.register_native_function("test", "F", f).unwrap();

    block_on(engine.compile("A", &Options::default())).unwrap();
    let b = block_on(engine.compile("B", &Options::default())).unwrap();
    let func = b.get_func("MyFunc").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(
        &engine,
        &mut stack,
        State {
            a: engine.get_type("A").unwrap(),
        },
    );

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).now_or_never().unwrap();
}

#[test]
#[should_panic]
fn call_with_incorrect_parameter_count() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        function F(a: float, b: int) static:
        local nonevar: none
        return nonevar
        "#,
    );

    engine.register(&module);

    let a = block_on(engine.compile("A", &Options::default())).unwrap();
    let func = a.get_func("F").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    func.call(&mut ctx, &[Val::F32(0.0)])
        .now_or_never()
        .unwrap();
}

#[test]
#[should_panic]
fn return_null_non_nullable_object() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn f(_caller: Caller<'_, ()>) -> ObjectAddr {
        ObjectAddr::NULL
    }

    let module = assemble(
        r#"
        class A:

        function F() -> string static native:

        function G() -> string static:
        local res: string
        res = call_static A F
        return res
        "#,
    );

    engine.register(&module);
    engine.register_native_function("A", "F", f).unwrap();

    let instance = block_on(engine.compile("A", &Options::default())).unwrap();
    let func = instance.get_func("G").unwrap();
    let func = func.typed::<(), ObjectAddr>().unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    func.call(&mut ctx, ()).now_or_never().unwrap();
}
