use openfo4_script::assembler::assemble;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Caller, Context, Exception, ObjectAddr, Stack};

#[test]
fn int_var() {
    let mut engine = Engine::new();

    fn assert_valid_result(_caller: Caller<'_, ()>, v: i32) -> Result<(), Exception> {
        assert_eq!(v, 1);
        Ok(())
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

    engine.register(&module);
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).unwrap();
}

#[test]
fn class_var() {
    let mut engine = Engine::new();

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

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();
    let layout = engine.get_type("MyClass").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let input = engine.object_pool().alloc(layout, 1).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).unwrap();
    assert_eq!(input, output);
}

#[test]
fn struct_var() {
    let mut engine = Engine::new();

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

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();
    let layout = engine.get_type("MyClass#MyStruct").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let input = engine.object_pool().alloc(layout, 1).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).unwrap();
    assert_eq!(input, output);
}
