use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Context, ObjectAddr, Stack};

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

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 600);
}
