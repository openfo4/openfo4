use openfo4_script::assembler::assemble;
use openfo4_script::engine::Engine;
use openfo4_script::runtime::{Caller, Context, Exception, ObjectAddr, Stack};

#[test]
fn array_create() {
    let mut engine = Engine::new();

    fn test_valid_array(caller: Caller<'_, ()>, addr: ObjectAddr) -> Result<(), Exception> {
        let array = unsafe { caller.array::<i32>(addr) };

        for elem in array {
            assert_eq!(*elem, 0);
        }

        Ok(())
    }

    let test = assemble(
        r#"
        class test:

        function test_valid_array(arr: int[]) static native:
        "#,
    );
    let module = assemble(
        r#"
        class MyClass:

        function MyFunction() static:
        local my_array: int[]
        local nonevar: none
        my_array = array_create 5
        nonevar = call_static test test_valid_array my_array
        "#,
    );

    dbg!(&module);

    engine.register(&module);
    engine.register(&test);
    engine.register_native_function("test", "test_valid_array", test_valid_array);

    let instance = engine.compile("MyClass");
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::new(func.stack_size());
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).unwrap();
}
