use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Context, Exception};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn call_method_on_null() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyMethod():
        local nonevar: none
        return nonevar

        function MyFunction() static:
        local nonevar: none
        local object: MyClass
        object = store null
        nonevar = call_method MyMethod object
        return nonevar
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    assert_eq!(func.call(&mut ctx, ()), Err(Exception::Error));
}

#[test]
fn array_get_on_null() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyFunction() static:
        local index: int
        local arr: int[]
        local res: int
        local nonevar: none
        res = array_get arr index
        return nonevar
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    assert_eq!(func.call(&mut ctx, ()), Err(Exception::Error));
}

#[test]
fn array_get_out_of_bounds() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyFunction() static:
        local index: int
        local arr: int[]
        local res: int
        local nonevar: none
        arr = array_create 2
        index = store 3
        res = array_get arr index
        return nonevar
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    assert_eq!(func.call(&mut ctx, ()), Err(Exception::Error));
}
