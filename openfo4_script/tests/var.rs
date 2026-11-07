use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Caller, Context, ObjectAddr};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn int_var() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(_caller: Caller<'_, ()>, v: i32) {
        assert_eq!(v, 1);
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(value: int) static native:

        class MyClass:

        function MyFunction() static:
        local a: int
        local b: var
        local c: int
        local nonevar: none
        a = store 1
        b = cast a
        c = cast b
        nonevar = call_static test assert_valid_result c
        "#,
    );

    block_on(engine.register(&module));
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).now_or_never().unwrap();
}

#[test]
fn string_var() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyFunction(a: string) -> string static:
        local b: var
        local c: string
        b = cast a
        c = cast b
        return c
        "#,
    );

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let input = engine.object_pool().insert_string("Hello World").addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).now_or_never().unwrap();
    assert_eq!(input, output);
}

#[test]
fn class_var() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyFunction(a: MyClass) -> MyClass static:
        local b: var
        local c: MyClass
        b = cast a
        c = cast b
        return c
        "#,
    );

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();
    let class = engine.get_type("MyClass").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let input = engine.object_pool().insert(class.id()).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).now_or_never().unwrap();
    assert_eq!(input, output);
}

#[test]
fn struct_var() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        struct MyStruct:
            field a: int
            field b: float

        function MyFunction(a: MyClass#MyStruct) -> MyClass#MyStruct static:
        local b: var
        local c: MyClass#MyStruct
        b = cast a
        c = cast b
        return c
        "#,
    );

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();
    let struct_ = engine.get_type("MyClass#MyStruct").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let input = engine.object_pool().insert(struct_.id()).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).now_or_never().unwrap();
    assert_eq!(input, output);
}
