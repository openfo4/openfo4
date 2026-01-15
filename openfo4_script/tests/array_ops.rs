use openfo4_script::assembler::assemble;
use openfo4_script::compiler::{FieldType, ObjectField, TypeLayout};
use openfo4_script::engine::{Engine, Options};
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

    engine.register(&module);
    engine.register(&test);
    engine.register_native_function("test", "test_valid_array", test_valid_array);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::new(func.stack_size());
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).unwrap();
}

#[test]
fn array_find_int() {
    let mut engine = Engine::new();

    fn assert_valid_result(
        caller: Caller<'_, ()>,
        addr: ObjectAddr,
        index: i32,
    ) -> Result<(), Exception> {
        let array = unsafe { caller.array::<i32>(addr) };

        assert_eq!(index, 42);
        assert_eq!(array[index as usize], 42);

        Ok(())
    }

    let test = assemble(
        r#"
        class test:

        function assert_valid_result(arr: int[], index: int) static native:
        "#,
    );
    let module = assemble(
        r#"
        class MyClass:
    
        function MyFunction(arr: int[]) static:
        local res: int
        local nonevar: none
        res = array_find arr 42 10
        nonevar = call_static test assert_valid_result arr res
        "#,
    );

    engine.register(&module);
    engine.register(&test);
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let mut builder = engine.object_pool().insert_array(int_layout(), 100);
    for i in 0..100 {
        let value = if i < 10 { 42 } else { i as i32 };
        builder.set_at::<i32>(i, "", "", value);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::new(func.stack_size());
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

fn int_layout() -> TypeLayout {
    TypeLayout {
        header: None,
        fields: vec![ObjectField {
            class: "".to_owned(),
            name: "".to_owned(),
            offset: 0,
            ty: FieldType::I32,
            ty_name: "".to_owned(),
        }],
        properties: vec![],
    }
}
