use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn default_class_values() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        global a: int = -100
        global b: float = 32.0f
        global c: bool = true
        global d: int = 0
        property a: int -> a
        property b: float -> b
        property c: bool -> c
        property d: int -> d
        "#,
    );

    engine.register(&module);
    engine.compile("MyClass", &Options::default());
    let layout = engine.get_type("MyClass").unwrap();

    let mut builder = engine.object_pool().insert(layout.clone());
    builder.set("MyClass", "d", 50);

    let object = builder.finalize();
    let object = engine.object_pool().get(object).unwrap();

    unsafe {
        assert_eq!(
            object.get("MyClass", "a").unwrap(),
            bytemuck::bytes_of(&-100_i32)
        );
        assert_eq!(
            object.get("MyClass", "b").unwrap(),
            bytemuck::bytes_of(&32.0_f32)
        );
        assert_eq!(object.get("MyClass", "c").unwrap(), [1]);
        assert_eq!(
            object.get("MyClass", "d").unwrap(),
            bytemuck::bytes_of(&50_i32)
        );
    }
}
