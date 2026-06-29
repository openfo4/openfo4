use openfo4_script::{
    assembler::assemble,
    compiler::CompilerOptions,
    engine::{Engine, Options},
    runtime::{Caller, Context, ObjectAddr},
};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn string_constant() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) {
        let s = caller.string(addr);
        assert_eq!(s, "Hello World!");
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

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).unwrap();
}

#[test]
fn string_concat() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) {
        let s = caller.string(addr);
        assert_eq!(s, "Hello World");
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

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(ObjectAddr, ObjectAddr), ()>().unwrap();
    func.call(&mut ctx, (a.addr(), b.addr())).unwrap();
}

#[test]
fn string_cmpeq() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function IsHello(lhs: string) -> bool static:
        local res: bool
        local rhs: string
        rhs = store "Hello"
        res = cmpeq lhs rhs
        return res
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("IsHello").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, bool>().unwrap();

    let input = engine.object_pool().insert_string("Hello").addr();
    assert!(func.call(&mut ctx, input).unwrap());

    let input = engine.object_pool().insert_string("NotHello").addr();
    assert!(!func.call(&mut ctx, input).unwrap());
}
