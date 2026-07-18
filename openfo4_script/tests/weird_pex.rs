use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn pex_missing_return() {
    compile_test(
        r#"
        class MyClass:

        function MyFunc() -> none static:
        "#,
        "MyClass",
    );
}

#[test]
fn pex_instruction_after_return() {
    compile_test(
        r#"
        class MyClass:

        function MyFunc() -> int static:
        return 1
        branch 0
        return 2
        "#,
        "MyClass",
    );
}

#[test]
fn pex_instruction_after_jump() {
    compile_test(
        r#"
        class MyClass:

        function MyFunc() -> int static:
        branch 2
        return 1
        branch -2
        return 2
        "#,
        "MyClass",
    );
}

#[test]
fn pex_out_of_bounds_jump() {
    compile_test(
        r#"
        class MyClass:

        function MyFunc() -> int static:
        branch 4
        return 1
        "#,
        "MyClass",
    );
}

fn compile_test(module: &str, class: &str) {
    let module = assemble(module);

    let mut engine = Engine::new(COMPILER_OPTIONS);
    engine.register(&module);
    block_on(engine.compile(class, &Options::default())).unwrap();
}
