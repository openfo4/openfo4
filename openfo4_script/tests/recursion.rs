use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::Context;

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn recursion() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function factorial(n: int) -> int static:
        local break: bool
        local res: int
        local next_n: int
        break = cmple n 1
        branch_if_false break 2
        return 1
        next_n = isub n 1
        res = call_static MyClass factorial next_n
        res = imul res n
        return res
        "#,
    );

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("factorial").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<i32, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, 10).now_or_never().unwrap(), 3628800);
}

#[test]
fn recursion_multiple_functions() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"

        class MyClass:

        function A(n: int) -> int static:
        local break: bool
        local res: int
        local next_n: int
        break = cmple n 1
        branch_if_false break 2
        return 1
        next_n = isub n 1
        res = call_static MyClass B next_n
        res = imul res n
        return res

        function B(n: int) -> int static:
        local break: bool
        local res: int
        local next_n: int
        break = cmple n 1
        branch_if_false break 2
        return 1
        next_n = isub n 1
        res = call_static MyClass A next_n
        res = imul res n
        return res
        "#,
    );
    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("A").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<i32, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, 10).now_or_never().unwrap(), 3628800);
}
