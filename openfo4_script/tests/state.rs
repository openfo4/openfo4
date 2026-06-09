use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Context, ObjectAddr};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn state() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        state "":
        
        function MyFunction() -> int:
        return 3

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

    let mut stack = engine.create_stack();
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
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 3);
}

#[test]
fn state_override_order() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        state "" default:

        function MyFunction() -> int:
        return 1

        state "X":

        function MyFunction() -> int:
        return 2

        state "Y":
        function MyFunction() -> int:
        return 3

        class B:
        parent: A

        state "" default:

        state "X":

        function MyFunction() -> int:
        return 4

        state "Z":
        function MyFunction() -> int:
        return 5
        "#,
    );

    engine.register(&module);
    let instance = engine.compile("B", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();
    let type_id = engine.get_type("B").unwrap();

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();

    // Default state for all classes.
    // Calls A.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 1);

    let x = engine.object_pool().insert_string("X").addr();
    let y = engine.object_pool().insert_string("Y").addr();
    let z = engine.object_pool().insert_string("Z").addr();

    unsafe {
        let mut obj = engine.object_pool().get(addr);
        obj.set("B", "::state", x);
    }

    // C is the subclass of {A, B} with state "X".
    // Calls C.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 4);

    unsafe {
        let mut obj = engine.object_pool().get(addr);
        obj.set("B", "::state", y);
    }

    // C has no state "Y", so B is the subclass of A with state "Y".
    // Calls B.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 3);

    unsafe {
        let mut obj = engine.object_pool().get(addr);
        obj.set("B", "::state", z);
    }

    // C is the subclass of {A, B} with state "Z".
    // Calls C.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).unwrap(), 5);
}
