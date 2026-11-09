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
fn default_class_values() {
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));
    block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let class = engine.get_type("MyClass").unwrap();

    let object = engine.object_pool().insert(class.id());
    object.set_self("d", 50).unwrap();

    assert_eq!(object.get_self::<i32>("a").unwrap(), -100);
    assert_eq!(object.get_self::<f32>("b").unwrap(), 32.0);
    assert_eq!(object.get_self::<bool>("c").unwrap(), true);
    assert_eq!(object.get_self::<i32>("d").unwrap(), 50);
}

#[test]
fn field_without_property() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        global a: int = 0
        global b: float = 0.0f
        global c: A = null

        property a: int -> a
        property c: A -> c

        class B:
        parent: A

        global e: bool = false

        property e: bool -> e

        function GetA() -> int:
        return a

        function GetC() -> A:
        return c

        function GetE() -> bool:
        return e
        "#,
    );

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("B", &Options::default())).unwrap();
    let a = engine.get_type("A").unwrap();
    let b = engine.get_type("B").unwrap();

    let builder = engine.object_pool().insert(b.id());
    builder.set(a.id(), "a", 1).unwrap();
    builder.set(a.id(), "c", builder.addr()).unwrap();
    builder.set(b.id(), "e", true).unwrap();

    let addr = builder.addr();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let get_a = instance.get_func("GetA").unwrap();
    let get_a = get_a.typed::<ObjectAddr, i32>().unwrap();
    assert_eq!(get_a.call(&mut ctx, addr).now_or_never().unwrap(), 1);

    let get_b = instance.get_func("GetC").unwrap();
    let get_c = get_b.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(get_c.call(&mut ctx, addr).now_or_never().unwrap(), addr);

    let get_e = instance.get_func("GetE").unwrap();
    let get_e = get_e.typed::<ObjectAddr, bool>().unwrap();
    assert_eq!(get_e.call(&mut ctx, addr).now_or_never().unwrap(), true);
}
