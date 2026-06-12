use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Context, ObjectAddr};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn variable_shadowing() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class Super:
        global A: int = 0
        property A: int -> A

        function GetA() -> int:
        return A

        class Sub:
        parent: Super
        global A: int = 0
        property A: int -> A

        function GetA() -> int:
        return A

        class SubSub:
        parent: Sub
        global A: int = 0
        property A: int -> A

        function GetA() -> int:
        return A
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("SubSub", &Options::default());
    let func = instance.get_func("GetA").unwrap();

    let type_id = engine.get_type("SubSub").unwrap();

    let mut builder = engine.object_pool().insert(type_id);
    builder.set("SubSub", "A", 600);
    builder.set("Sub", "A", 400);
    builder.set("Super", "A", 200);

    let addr = builder.finalize().addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 600);
}

#[test]
fn load_property_from_parent_class() {
    let mut engine = Engine::new(COMPILER_OPTIONS);
    let module = assemble(
        r#"
        class A:
        global a: int = 0
        property A: int -> a

        class B:
        parent: A

        function F() -> int:
        local res: int
        res = property_get A self
        return res
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("B", &Options::default());
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("B").unwrap();

    let mut builder = engine.object_pool().insert(type_id);
    builder.set("A", "A", 1);

    let addr = builder.finalize().addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 1);
}
