use openfo4_script::assembler::assemble;
use openfo4_script::compiler::CompilerOptions;
use openfo4_script::engine::{Engine, Options};
use openfo4_script::runtime::{Caller, Context, Exception, NativeTuple, ObjectAddr};

const COMPILER_OPTIONS: CompilerOptions = CompilerOptions {
    validation: true,
    debug: true,
};

#[test]
fn array_create() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn test_valid_array(caller: Caller<'_, ()>, addr: ObjectAddr) {
        let array = unsafe { caller.array::<i32>(addr) };

        for elem in array {
            assert_eq!(*elem, 0);
        }
    }

    let module = assemble(
        r#"
        class test:

        function test_valid_array(arr: int[]) static native:

        class MyClass:

        function MyFunction() static:
        local my_array: int[]
        local nonevar: none
        my_array = array_create 5
        nonevar = call_static test test_valid_array my_array
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "test_valid_array", test_valid_array)
        .unwrap();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), ()>().unwrap();
    func.call(&mut ctx, ()).unwrap();
}

#[test]
fn array_get() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(_caller: Caller<'_, ()>, value: i32) {
        assert_eq!(value, 9);
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(index: int) static native:

        class MyClass:

        function MyFunction(arr: int[]) static:
        local res: int
        local nonevar: none
        res = array_get arr 3
        nonevar = call_static test assert_valid_result res
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let mut builder = engine.object_pool().insert_array(engine.int_type(), 10);
    for i in 0..10 {
        builder.set_at::<i32>(i, "", "", (i * i) as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_add() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) {
        let arr = unsafe { caller.array::<i32>(addr) };
        assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 5, 5, 5]);
    }

    let module = assemble(
        r#"
        class test:
        function assert_valid_result(arr: int[]) static native:

        class MyClass:

        function MyFunction(arr: int[]) static:
        local nonevar: none
        array_add arr 5 3
        nonevar = call_static test assert_valid_result arr
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let mut builder = engine.object_pool().insert_array(engine.int_type(), 10);
    for i in 0..10 {
        builder.set_at::<i32>(i, "", "", i as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_insert() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) {
        let arr = unsafe { caller.array::<i32>(addr) };
        assert_eq!(arr, [0, 1, 2, 42, 3, 4, 5, 6, 7, 8, 9]);
    }

    let module = assemble(
        r#"
        class test:
        function assert_valid_result(arr: int[]) static native:

        class MyClass:

        function MyFunction(arr: int[]) static:
        local nonevar: none
        array_insert arr 42 3
        nonevar = call_static test assert_valid_result arr
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let mut builder = engine.object_pool().insert_array(engine.int_type(), 10);
    for i in 0..10 {
        builder.set_at::<i32>(i, "", "", i as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_clear() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) {
        let array = unsafe { caller.array::<i32>(addr) };
        assert_eq!(array.len(), 0);
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(arr: int[]) static native:

        class MyClass:

        function MyFunction(arr: int[]) static:
        local nonevar: none
        array_clear arr
        nonevar = call_static test assert_valid_result arr
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let mut builder = engine.object_pool().insert_array(engine.int_type(), 5);
    for i in 0..5 {
        builder.set_at::<i32>(i, "", "", i as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_find_int() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr, index: i32) {
        let array = unsafe { caller.array::<i32>(addr) };

        assert_eq!(index, 42);
        assert_eq!(array[index as usize], 42);
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(arr: int[], index: int) static native:

        class MyClass:

        function MyFunction(arr: int[]) static:
        local res: int
        local nonevar: none
        res = array_find arr 42 10
        nonevar = call_static test assert_valid_result arr res
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let mut builder = engine.object_pool().insert_array(engine.int_type(), 100);
    for i in 0..100 {
        let value = if i < 10 { 42 } else { i as i32 };
        builder.set_at::<i32>(i, "", "", value);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_find_struct() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr, index: i32) {
        let array = unsafe { caller.array::<ObjectAddr>(addr) };
        let object = caller.object_pool().get(array[index as usize]).unwrap();

        assert_eq!(index, 42);
        unsafe {
            assert_eq!(object.get_typed::<i32>("", "x").unwrap(), 42);
            assert_eq!(object.get_typed::<f32>("", "y").unwrap(), 42.0);
        }
    }

    let module = assemble(
        r#"
        class test:

        function assert_valid_result(arr: MyClass#MyStruct[], index: int) static native:

        class MyClass:

        struct MyStruct:
            field x: int
            field y: float

        function MyFunction(arr: MyClass#MyStruct[]) static:
        local res: int
        local nonevar: none
        res = array_find_struct arr "y" 42.0f 10
        nonevar = call_static test assert_valid_result arr res
        "#,
    );

    engine.register(&module);
    engine
        .register_native_function("test", "assert_valid_result", assert_valid_result)
        .unwrap();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let struct_layout = engine.get_type("MyClass#MyStruct").unwrap();

    let mut array = engine.object_pool().insert_array(engine.ptr_type(), 100);
    for i in 0..100 {
        let mut object = engine.object_pool().insert(struct_layout.clone());

        if i < 10 {
            object.set::<i32>("", "x", 42);
            object.set::<f32>("", "y", 42.0);
        } else {
            object.set::<i32>("", "x", i as i32);
            object.set::<f32>("", "y", i as f32);
        }

        let object = object.finalize();

        array.set_at::<ObjectAddr>(i, "", "", object.addr());
    }

    let array_ptr = array.finalize();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_len_on_null() {
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(
        r#"
        class MyClass:

        function MyFunction() -> int static:
        local arr: int[]
        local res: int
        res = array_length arr
        return res
        "#,
    );

    engine.register(&module);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<(), i32>().unwrap();
    assert_eq!(func.call(&mut ctx, ()).unwrap(), 0);
}

#[test]
fn array_add_on_null() {
    let res = run_test::<_, (), ()>(
        r#"
        class MyClass:

        function MyFunc() static:
        local array: int[]
        local value: int
        local count: int
        local nonevar: none
        array_add array value count
        return nonevar
        "#,
        "MyClass",
        "MyFunc",
        |_| {},
        (),
    );
    assert_eq!(res, Err(Exception::Error));
}

#[test]
fn array_insert_on_null() {
    let res = run_test::<_, (), ()>(
        r#"
        class MyClass:

        function MyFunc() static:
        local array: int[]
        local value: int
        local index: int
        local nonevar: none
        array_insert array value index
        return nonevar
        "#,
        "MyClass",
        "MyFunc",
        |_| {},
        (),
    );
    assert_eq!(res, Err(Exception::Error));
}

#[test]
fn array_remove_on_null() {
    let res = run_test::<_, (), ()>(
        r#"
        class MyClass:

        function MyFunc() static:
        local array: int[]
        local index: int
        local count: int
        local nonevar: none
        array_remove array index count
        return nonevar
        "#,
        "MyClass",
        "MyFunc",
        |_| {},
        (),
    );
    assert_eq!(res, Err(Exception::Error));
}

#[test]
fn array_clear_on_null() {
    let res = run_test::<_, (), ()>(
        r#"
        class MyClass:

        function MyFunc() static:
        local array: int[]
        local nonevar: none
        array_clear array
        return nonevar
        "#,
        "MyClass",
        "MyFunc",
        |_| {},
        (),
    );
    assert_eq!(res, Err(Exception::Error));
}

#[test]
fn array_find_on_null() {
    let res = run_test::<_, (), ()>(
        r#"
        class MyClass:

        function MyFunc() static:
        local array: int[]
        local value: int
        local start_index: int
        local dst: int
        local nonevar: none
        dst = array_find array value start_index
        return nonevar
        "#,
        "MyClass",
        "MyFunc",
        |_| {},
        (),
    );
    assert_eq!(res, Err(Exception::Error));
}

#[test]
fn array_find_struct_on_null() {
    let res = run_test::<_, (), ()>(
        r#"
        class MyClass:

        struct MyStruct:
            field x: int

        function MyFunc() static:
        local array: MyClass#MyStruct[]
        local value: int
        local start_index: int
        local dst: int
        local nonevar: none
        dst = array_find_struct array "x" value start_index
        return nonevar
        "#,
        "MyClass",
        "MyFunc",
        |_| {},
        (),
    );
    assert_eq!(res, Err(Exception::Error));
}

fn run_test<F, I, O>(
    module: &str,
    class: &str,
    func: &str,
    setup: F,
    input: I,
) -> Result<O, Exception>
where
    F: FnOnce(&mut Engine),
    I: NativeTuple,
    O: NativeTuple,
{
    let mut engine = Engine::new(COMPILER_OPTIONS);

    let module = assemble(module);
    engine.register(&module);

    let instance = engine.compile(class, &Options::default());
    let func = instance.get_func(func).unwrap();

    setup(&mut engine);

    let mut stack = engine.create_stack();
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<I, O>().unwrap();
    func.call(&mut ctx, input)
}
