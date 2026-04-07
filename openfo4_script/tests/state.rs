use openfo4_script::assembler::assemble;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Context, ObjectAddr, Stack};

#[test]
fn state() {
    let mut engine = Engine::new();

    let module = assemble(
        r#"
        class MyClass:

        state "a" default:

        function MyFunction() -> int:
        return 1

        state "b":

        function MyFunction() -> int:
        return 2
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();
    let type_id = engine.get_type("MyClass").unwrap();

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 1);

    let mut object = engine.object_pool().get(addr);
    unsafe {
        let b = engine.object_pool().insert_string("b");
        object.set("MyClass", "::state", b.addr());
    }

    assert_eq!(func.call(&mut ctx, addr).unwrap(), 2);

    unsafe {
        let a = engine.object_pool().insert_string("a");
        object.set("MyClass", "::state", a.addr());
    }

    assert_eq!(func.call(&mut ctx, addr).unwrap(), 1);

    unsafe {
        let a = engine.object_pool().insert_string("c");
        object.set("MyClass", "::state", a.addr());
    }

    // Fallthrough, default value
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 0);
}
