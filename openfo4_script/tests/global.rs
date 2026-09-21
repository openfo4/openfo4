use futures_lite::future::block_on;
use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::func::TypedFunc;
use openfo4_script::runtime::object::ObjectBuilder;
use openfo4_script::runtime::{Context, ObjectAddr};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

fn run_test<F>(f: F)
where
    F: FnOnce(
        &Engine,
        ObjectBuilder,
        TypedFunc<'_, (ObjectAddr, i32), ()>,
        TypedFunc<'_, ObjectAddr, i32>,
        &mut Context<'_, ()>,
    ),
{
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:
        global a: int = 0
        property a: int -> a

        function GetA() -> int:
        return a

        function SetA(v: int):
        a = store v
        "#,
    );

    engine.register(&module);
    let instance = block_on(engine.compile("MyClass", &Options::default())).unwrap();
    let id = engine.get_type("MyClass").unwrap();

    let object = engine.object_pool().insert(id);

    let set_a = instance.get_func("SetA").unwrap();
    let set_a = set_a.typed::<(ObjectAddr, i32), ()>().unwrap();

    let get_a = instance.get_func("GetA").unwrap();
    let get_a = get_a.typed::<ObjectAddr, i32>().unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    f(&engine, object, set_a, get_a, &mut ctx);
}

#[test]
fn load_global() {
    run_test(|_engine, mut object, _set, get, ctx| {
        object.set("MyClass", "a", 123);
        let object = object.finalize().addr();

        let res = get.call(ctx, object).now_or_never().unwrap();
        assert_eq!(res, 123);
    });
}

#[test]
fn store_global() {
    run_test(|engine, object, set, _get, ctx| {
        let object = object.finalize().addr();

        set.call(ctx, (object, 456)).now_or_never().unwrap();

        let object = engine.object_pool().get(object).unwrap();
        assert_eq!(
            unsafe { object.get_typed::<i32>("MyClass", "a").unwrap() },
            456
        );
    });
}

#[test]
fn store_and_load_global() {
    run_test(|_engine, object, set, get, ctx| {
        let object = object.finalize().addr();

        set.call(ctx, (object, 789)).now_or_never().unwrap();

        let res = get.call(ctx, object).now_or_never().unwrap();
        assert_eq!(res, 789);
    });
}
