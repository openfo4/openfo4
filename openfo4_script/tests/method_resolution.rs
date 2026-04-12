use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Context, ObjectAddr, Stack};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn method_resolution_external() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        function F() -> int:
        return 1

        function G() -> int:
        return 2

        class B:
        parent: A

        function F() -> int:
        return 3

        function H() -> int:
        return 4
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("B", &Options::default());
    let type_id = engine.get_type("B").unwrap();

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let mut stack = Stack::new(0);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let f = instance.get_func("F").unwrap();
    let f = f.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(f.call(&mut ctx, addr).unwrap(), 3);

    let g = instance.get_func("G").unwrap();
    let g = g.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(g.call(&mut ctx, addr).unwrap(), 2);

    let h = instance.get_func("H").unwrap();
    let h = h.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(h.call(&mut ctx, addr).unwrap(), 4);
}

#[test]
fn method_resolution_internal() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        function F() -> int:
        return 1

        function G() -> int:
        return 2

        class B:
        parent: A

        function F() -> int:
        return 3

        function H() -> int:
        return 4

        function CallF() -> int:
        local res: int
        res = call_method F self
        return res

        function CallG() -> int:
        local res: int
        res = call_method G self
        return res

        function CallH() -> int:
        local res: int
        res = call_method H self
        return res
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("B", &Options::default());
    let type_id = engine.get_type("B").unwrap();

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let mut stack = Stack::new(0);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let f = instance.get_func("CallF").unwrap();
    let f = f.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(f.call(&mut ctx, addr).unwrap(), 3);

    let g = instance.get_func("CallG").unwrap();
    let g = g.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(g.call(&mut ctx, addr).unwrap(), 2);

    let h = instance.get_func("CallH").unwrap();
    let h = h.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(h.call(&mut ctx, addr).unwrap(), 4);
}
