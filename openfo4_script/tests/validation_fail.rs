use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

fn run_test(module: &str, class: &str) {
    let module = assemble(module);

    let engine = Engine::new(COMPILER_OPTIONS);
    block_on(engine.register(&module));

    block_on(engine.compile(class, &Options::default())).unwrap_err();
}

#[test]
fn class_inheritance_cycle() {
    run_test(
        r#"
        class A:
        parent: B

        class B:
        parent: A
        "#,
        "A",
    );
}

#[test]
fn class_inheritance_cycle_indirect() {
    run_test(
        r#"
        class A:
        parent: B

        class B:
        parent: C

        class C:
        parent: D

        class D:
        parent: B
        "#,
        "A",
    );
}

#[test]
fn undefined_local() {
    run_test(
        r#"
        class A:

        function F():
        local a: int
        b = iadd a a
        "#,
        "A",
    );
}

#[test]
fn call_undefined_function() {
    run_test(
        r#"
        class A:

        function F():
        local nonevar: none
        nonevar = call_method G self
        "#,
        "A",
    );
}

#[test]
fn call_invalid_parameter_count() {
    run_test(
        r#"
        class A:

        function F(a: int, b: int):

        function G():
        local nonevar: none
        nonevar = call_method F self 1 2 3
        "#,
        "A",
    );
}

#[test]
fn call_invalid_parameter_type() {
    run_test(
        r#"
        class A:

        function F(a: int, b: float):

        function G():
        local nonevar: none
        nonevar = call_method F self 1 2
        "#,
        "A",
    );
}

#[test]
fn call_method_on_static() {
    run_test(
        r#"
        class A:

        function F() static:

        function G():
        local nonevar: none
        nonevar = call_method F self
        "#,
        "A",
    );
}

#[test]
fn call_static_on_method() {
    run_test(
        r#"
        class A:

        function F():

        function G() static:
        local nonevar: none
        nonevar = call_static A F
        "#,
        "A",
    );
}

#[test]
fn call_parent_on_static() {
    run_test(
        r#"
        class A:

        function F() static:

        class B:
        parent: A

        function G():
        local nonevar: none
        nonevar = call_parent F
        "#,
        "B",
    );
}

#[test]
fn call_parent_from_static() {
    run_test(
        r#"
        class A:

        function F():

        class B:
        parent: A

        function G() static:
        local nonevar: none
        nonevar = call_parent F
        "#,
        "B",
    );
}

#[test]
fn call_parent_from_root() {
    run_test(
        r#"
        class A:

        function F():
        local nonevar: none
        nonevar = call_parent F
        "#,
        "A",
    );
}

#[test]
fn property_get_unknown_property() {
    run_test(
        r#"
        class A:

        function F():
        local a: int
        a = property_get self F
        "#,
        "A",
    );
}
