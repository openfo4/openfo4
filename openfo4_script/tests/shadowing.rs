use futures_lite::future::block_on;
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

    let instance = block_on(engine.compile("SubSub", &Options::default())).unwrap();
    let func = instance.get_func("GetA").unwrap();

    let subsub = engine.get_type("SubSub").unwrap();

    let builder = engine.object_pool().insert(subsub.id());
    builder.set("SubSub", "A", 600).unwrap();
    builder.set("Sub", "A", 400).unwrap();
    builder.set("Super", "A", 200).unwrap();

    let addr = builder.addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 600);
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

    let instance = block_on(engine.compile("B", &Options::default())).unwrap();
    let func = instance.get_func("F").unwrap();
    let b = engine.get_type("B").unwrap();

    let builder = engine.object_pool().insert(b.id());
    builder.set("A", "A", 1).unwrap();

    let addr = builder.addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 1);
}
