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
fn state() {
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();
    let class = engine.get_type("MyClass").unwrap();

    let addr = engine.object_pool().insert(class.id()).addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 1);

    let object = engine.object_pool().get(addr).unwrap();
    let b = engine.object_pool().insert_string("b");
    object.set_self("::state", b.addr()).unwrap();

    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 2);

    let a = engine.object_pool().insert_string("a");
    object.set_self("::state", a.addr()).unwrap();

    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 1);

    let a = engine.object_pool().insert_string("c");
    object.set_self("::state", a.addr()).unwrap();

    // Fallthrough, default value
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 3);
}

#[test]
fn state_override_order() {
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));
    let instance = block_on(engine.compile("B", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();
    let b = engine.get_type("B").unwrap();

    let addr = engine.object_pool().insert(b.id()).addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, i32>().unwrap();

    // Default state for all classes.
    // Calls A.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 1);

    let x = engine.object_pool().insert_string("X").addr();
    let y = engine.object_pool().insert_string("Y").addr();
    let z = engine.object_pool().insert_string("Z").addr();

    let obj = engine.object_pool().get(addr).unwrap();
    obj.set_self("::state", x).unwrap();

    // C is the subclass of {A, B} with state "X".
    // Calls C.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 4);

    let obj = engine.object_pool().get(addr).unwrap();
    obj.set_self("::state", y).unwrap();

    // C has no state "Y", so B is the subclass of A with state "Y".
    // Calls B.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 3);

    let obj = engine.object_pool().get(addr).unwrap();
    obj.set_self("::state", z).unwrap();

    // C is the subclass of {A, B} with state "Z".
    // Calls C.MyFunction().
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), 5);
}

#[test]
fn access_undeclared_state_builtin() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        function F() -> string:
        return __state
        "#,
    );
    block_on(engine.register(&module));
    block_on(engine.compile("A", &Options::default())).unwrap();
}
