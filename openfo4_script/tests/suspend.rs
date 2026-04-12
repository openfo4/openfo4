use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Caller, Context, Exception, Stack};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn suspend() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn yield_once(mut caller: Caller<'_, bool>) -> Result<(), Exception> {
        if *caller.data() {
            Ok(())
        } else {
            *caller.data_mut() = true;
            Err(Exception::Suspend)
        }
    }

    let module = assemble(
        r#"
        class test:

        function YieldOnce() static native:

        class MyClass:

        function MyFunc() static:
        local nonevar: none
        nonevar = call_static test YieldOnce
        return nonevar
        "#,
    );

    engine.register(&module);
    engine.register_native_function("test", "YieldOnce", yield_once);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunc").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, false);

    let func = func.typed::<(), ()>().unwrap();
    assert_eq!(func.call(&mut ctx, ()), Err(Exception::Suspend));
    assert_eq!(func.call(&mut ctx, ()), Ok(()));
}
