use futures_lite::future::block_on;
use game_common::string::Lowercase;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::compiler::resources::PropertyType;
use openfo4_script::engine::{Engine, Options};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn class_reflection() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

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

    engine.register(&module);
    block_on(engine.compile("MyClass", &Options::default()));
    let id = engine.get_type("MyClass").unwrap();

    let info = engine.type_info(id);
    dbg!(&info);
    assert_eq!(
        info.properties
            .get(&(Lowercase::new("MyClass"), Lowercase::new("a")))
            .unwrap()
            .ty,
        PropertyType::Inline(engine.int_type())
    );
    assert_eq!(
        info.properties
            .get(&(Lowercase::new("MyClass"), Lowercase::new("b")))
            .unwrap()
            .ty,
        PropertyType::Inline(engine.float_type())
    );
    assert_eq!(
        info.properties
            .get(&(Lowercase::new("MyClass"), Lowercase::new("c")))
            .unwrap()
            .ty,
        PropertyType::Pointer(id)
    );
    assert_eq!(
        info.properties
            .get(&(Lowercase::new("MyClass"), Lowercase::new("d")))
            .unwrap()
            .ty,
        PropertyType::ArrayPointer(id)
    );
}

#[test]
fn struct_reflection() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        struct MyStruct:
            field a: MyClass[]
        "#,
    );

    engine.register(&module);
    block_on(engine.compile("MyClass", &Options::default()));
    let class = engine.get_type("MyClass").unwrap();
    let id = engine.get_type("MyClass#MyStruct").unwrap();

    let info = engine.type_info(id);
    assert_eq!(
        info.properties
            .get(&(Lowercase::new("MyClass#MyStruct"), Lowercase::new("a")))
            .unwrap()
            .ty,
        PropertyType::ArrayPointer(class)
    );
}
