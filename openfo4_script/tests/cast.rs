use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::func::TypedFunc;
use openfo4_script::runtime::{ArrayObject, Context, NativeTuple, ObjectAddr};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn cast_array() {
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunction").unwrap();
    let class_ty = engine.get_type("MyClass").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addrs: Vec<_> = (0..3)
        .map(|_| engine.object_pool().insert(class_ty.id()).addr())
        .collect();

    let mut input = engine
        .object_pool()
        .insert_array(class_ty.id(), addrs.len());
    for (index, addr) in addrs.iter().enumerate() {
        input.set_at(index, *addr);
    }

    let input = input.finalize().addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    let output = func.call(&mut ctx, input).now_or_never().unwrap();

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
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("C", &Options::default())).unwrap();
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("C").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id.id()).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), addr);
}

#[test]
fn class_downcast() {
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("C", &Options::default())).unwrap();
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("C").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id.id()).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(func.call(&mut ctx, addr).now_or_never().unwrap(), addr);
}

#[test]
fn class_downcast_fail() {
    let engine = Engine::new(COMPILER_OPTIONS);

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

    block_on(engine.register(&module));

    block_on(engine.compile("A", &Options::default())).unwrap();
    let instance = block_on(engine.compile("C", &Options::default())).unwrap();
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("A").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id.id()).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(
        func.call(&mut ctx, addr).now_or_never().unwrap(),
        ObjectAddr::NULL
    );
}

#[test]
fn class_downcast_on_null() {
    let engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class A:

        class B:
        parent: A

        function F(a: A) -> B static:
        local b: B
        b = cast a
        return b
        "#,
    );

    block_on(engine.register(&module));

    block_on(engine.compile("A", &Options::default())).unwrap();
    let instance = block_on(engine.compile("B", &Options::default())).unwrap();
    let func = instance.get_func("F").unwrap();
    let type_id = engine.get_type("A").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let addr = engine.object_pool().insert(type_id.id()).addr();

    let func = func.typed::<ObjectAddr, ObjectAddr>().unwrap();
    assert_eq!(
        func.call(&mut ctx, addr).now_or_never().unwrap(),
        ObjectAddr::NULL
    );
}

#[test]
fn cast_bool_to_int() {
    cast_test_impl("bool", "int", |_, func, ctx| {
        let res: i32 = func.call(ctx, false).now_or_never().unwrap();
        assert_eq!(res, 0);

        let res: i32 = func.call(ctx, true).now_or_never().unwrap();
        assert_eq!(res, 1);
    });
}

#[test]
fn cast_bool_to_float() {
    cast_test_impl("bool", "float", |_, func, ctx| {
        let res: f32 = func.call(ctx, false).now_or_never().unwrap();
        assert_eq!(res, 0.0);

        let res: f32 = func.call(ctx, true).now_or_never().unwrap();
        assert_eq!(res, 1.0);
    });
}

#[test]
fn cast_int_to_bool() {
    cast_test_impl("int", "bool", |_, func, ctx| {
        let res: bool = func.call(ctx, 0).now_or_never().unwrap();
        assert!(!res);

        let res: bool = func.call(ctx, 123).now_or_never().unwrap();
        assert!(res);
    });
}

#[test]
fn cast_int_to_float() {
    cast_test_impl("int", "float", |_, func, ctx| {
        let res: f32 = func.call(ctx, 42).now_or_never().unwrap();
        assert_eq!(res, 42.0);
    });
}

#[test]
fn cast_float_to_bool() {
    cast_test_impl("float", "bool", |_, func, ctx| {
        let res: bool = func.call(ctx, 0.0).now_or_never().unwrap();
        assert!(!res);

        let res: bool = func.call(ctx, 1.5).now_or_never().unwrap();
        assert!(res);
    });
}

#[test]
fn cast_float_to_int() {
    cast_test_impl("float", "int", |_, func, ctx| {
        let res: i32 = func.call(ctx, -42.0).now_or_never().unwrap();
        assert_eq!(res, -42);
    });
}

#[test]
fn cast_bool_to_string() {
    cast_test_impl("bool", "string", |engine, func, ctx| {
        let string = func.call(ctx, true).now_or_never().unwrap();
        let string = unsafe { engine.object_pool().string(string) };
        assert_eq!(string, "true");
    });
}

#[test]
fn cast_int_to_string() {
    cast_test_impl("int", "string", |engine, func, ctx| {
        let string = func.call(ctx, 32).now_or_never().unwrap();
        let string = unsafe { engine.object_pool().string(string) };
        assert_eq!(string, "32");
    });
}

#[test]
fn cast_float_to_string() {
    cast_test_impl("float", "string", |engine, func, ctx| {
        let string = func.call(ctx, -42.0).now_or_never().unwrap();
        let string = unsafe { engine.object_pool().string(string) };
        assert_eq!(string, "-42");
    });
}

#[test]
fn cast_null_to_class() {
    cast_test_impl("none", "MyClass", |_engine, func, ctx| {
        let res: ObjectAddr = func.call(ctx, ObjectAddr::NULL).now_or_never().unwrap();
        assert_eq!(res, ObjectAddr::NULL);
    });
}

fn cast_test_impl<F, Params, Returns>(in_ty: &str, out_ty: &str, f: F)
where
    F: FnOnce(&mut Engine, &TypedFunc<'_, Params, Returns>, &mut Context<'_, ()>),
    Params: NativeTuple,
    Returns: NativeTuple,
{
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(&format!(
        r#"
        class MyClass:

        function MyFunc(in: {in_ty}) -> {out_ty} static:
        local out: {out_ty}
        out = cast in
        return out
        "#,
    ));

    block_on(engine.register(&module));

    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let func = instance.get_func("MyFunc").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed().unwrap();
    f(&mut engine, &func, &mut ctx);
}
