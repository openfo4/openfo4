use openfo4_script::{
    assembler::assemble,
    engine::{Engine, Options},
    runtime::{Caller, Context, Exception, ObjectAddr, Stack},
};

#[test]
fn string_constant() {
    let mut engine = Engine::new();

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) -> Result<(), Exception> {
        let s = unsafe { caller.string(addr) };
        assert_eq!(s, "Hello World!");
        Ok(())
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(s: string) static native:

        class MyClass:

        function MyFunction() static:
        local nonevar: none
        nonevar = call_static test assert_valid_result "Hello World!"
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
fn string_concat() {
    let mut engine = Engine::new();

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) -> Result<(), Exception> {
        let s = unsafe { caller.string(addr) };
        assert_eq!(s, "Hello World");
        Ok(())
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(s: string) static native:

        class MyClass:

        function MyFunction(a: string, b: string) static:
        local res: string
        local nonevar: none
        res = string_concat a b
        nonevar = call_static test assert_valid_result res
        "#,
    );

    engine.register(&module);
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let a = engine.object_pool().insert_string("Hello");
    let b = engine.object_pool().insert_string(" World");

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(ObjectAddr, ObjectAddr), ()>().unwrap();
    func.call(&mut ctx, (a.addr(), b.addr())).unwrap();
}
