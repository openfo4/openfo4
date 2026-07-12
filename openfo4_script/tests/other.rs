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
fn pass_null_param() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        function F(a: A) static:
        "#,
    );
    engine.register(&module);

    let instance = block_on(engine.compile("A", &Options::default()));
    let func = instance.get_func("F").unwrap();
    let func = func.typed::<ObjectAddr, ()>().unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    func.call(&mut ctx, ObjectAddr::NULL).unwrap();
}

#[test]
fn host_function_returns_null() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn f(_caller: Caller<'_, ()>) -> ObjectAddr {
        ObjectAddr::NULL
    }

    let module = assemble(
        r#"
        class A:

        function F() -> A static native:

        function G() -> A static:
        local res: A
        res = call_static A F
        return res
        "#,
    );

    engine.register(&module);
    engine.register_native_function("A", "F", f).unwrap();

    let instance = block_on(engine.compile("A", &Options::default()));
    let func = instance.get_func("G").unwrap();
    let func = func.typed::<(), ObjectAddr>().unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    assert_eq!(func.call(&mut ctx, ()).unwrap(), ObjectAddr::NULL);
}
