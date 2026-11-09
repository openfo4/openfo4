use futures_lite::future::block_on;
use openfo4_common::string::Lowercase;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn class_reflection() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        global a: int = 0
        global b: float = 0f
        global c: MyClass = null
        global d: MyClass[] = null
        property a: int -> a
        property b: float -> b
        property c: MyClass -> c
        property d: MyClass[] -> d
        "#,
    );

    block_on(engine.register(&module));
    block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let class = engine.get_type("MyClass").unwrap();

    let info = class.info();
    assert_eq!(
        info.properties
            .get(info.id(), &Lowercase::new("a"))
            .unwrap()
            .ty,
        engine.int_type()
    );
    assert_eq!(
        info.properties
            .get(info.id(), &Lowercase::new("b"))
            .unwrap()
            .ty,
        engine.float_type()
    );
    assert_eq!(
        info.properties
            .get(info.id(), &Lowercase::new("c"))
            .unwrap()
            .ty,
        engine.get_type("MyClass").unwrap().id()
    );
    assert_eq!(
        info.properties
            .get(info.id(), &Lowercase::new("d"))
            .unwrap()
            .ty,
        engine.get_type("MyClass[]").unwrap().id()
    );
}

#[test]
fn struct_reflection() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        struct MyStruct:
            field a: MyClass[]
        "#,
    );

    block_on(engine.register(&module));
    block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let struct_ = engine.get_type("MyClass#MyStruct").unwrap();

    let info = struct_.info();
    assert_eq!(
        info.properties
            .get(info.id(), &Lowercase::new("a"))
            .unwrap()
            .ty,
        engine.get_type("MyClass[]").unwrap().id()
    );
}
