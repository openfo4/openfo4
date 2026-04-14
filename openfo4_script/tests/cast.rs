use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{ArrayObject, Context, Exception, ObjectAddr, Stack};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn cast_array() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyFunction(a: MyClass[]) -> MyClass[] static:
        local b: var[]
        local c: MyClass[]
        b = cast a
        c = cast b
        return c
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();
    let layout = engine.get_type("MyClass").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addrs: Vec<_> = (0..3)
        .map(|_| engine.object_pool().insert(layout).finalize().addr())
        .collect();

    let mut input = engine
        .object_pool()
        .insert_array(engine.ptr_type(), addrs.len());
    for (index, addr) in addrs.iter().enumerate() {
        input.set_at(index, "", "", *addr);
    }

    let input = input.finalize().addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).unwrap();

    assert_ne!(input, output);

    let output_array = unsafe { output.to_raw().cast::<ArrayObject>().read() };
    assert_eq!(output_array.len, 3);

    for (index, addr) in addrs.iter().enumerate() {
        let elem = unsafe { output_array.ptr.cast::<ObjectAddr>().add(index).read() };
        assert_eq!(elem, *addr);
    }
}

#[test]
fn class_upcast() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        class B:
        parent: A

        class C:
        parent: B

        function F(c: C) -> A static:
        local a: A
        a = cast c
        return a
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("C", &Options::default());
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("C").unwrap();

    let mut stack = Stack::new(0);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).unwrap(), addr);
}

#[test]
fn class_downcast() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        class B:
        parent: A

        class C:
        parent: B

        function F(a: A) -> C static:
        local c: C
        c = cast a
        return c
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("C", &Options::default());
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("C").unwrap();

    let mut stack = Stack::new(0);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).unwrap(), addr);
}

#[test]
fn class_downcast_fail() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        class B:
        parent: A

        class C:
        parent: B

        function F(a: A) -> C static:
        local c: C
        c = cast a
        return c
        "#,
    );

    engine.register(&module);

    engine.compile("A", &Options::default());
    let instance = engine.compile("C", &Options::default());
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("A").unwrap();

    let mut stack = Stack::new(0);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id).finalize().addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(func.call(&mut ctx, addr), Err(Exception::Error));
}
