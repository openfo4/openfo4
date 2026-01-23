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

#[test]
fn array_get() {
    let mut engine = Engine::new();

    fn assert_valid_result(_caller: Caller<'_, ()>, value: i32) -> Result<(), Exception> {
        assert_eq!(value, 9);
        Ok(())
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
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let mut builder = engine.object_pool().insert_array(int_layout(), 10);
    for i in 0..10 {
        builder.set_at::<i32>(i, "", "", (i * i) as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::new(func.stack_size());
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_add() {
    let mut engine = Engine::new();

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) -> Result<(), Exception> {
        let arr = unsafe { caller.array::<i32>(addr) };
        assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 5, 5, 5]);
        Ok(())
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
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let mut builder = engine.object_pool().insert_array(int_layout(), 10);
    for i in 0..10 {
        builder.set_at::<i32>(i, "", "", i as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_insert() {
    let mut engine = Engine::new();

    fn assert_valid_result(caller: Caller<'_, ()>, addr: ObjectAddr) -> Result<(), Exception> {
        let arr = unsafe { caller.array::<i32>(addr) };
        assert_eq!(arr, [0, 1, 2, 42, 3, 4, 5, 6, 7, 8, 9]);
        Ok(())
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
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let mut builder = engine.object_pool().insert_array(int_layout(), 10);
    for i in 0..10 {
        builder.set_at::<i32>(i, "", "", i as i32);
    }

    let array_ptr = builder.finalize();

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let mut stack = Stack::for_func(func);
    let mut ctx = Context::new(&engine, &mut stack, ());

    let func = func.typed::<ObjectAddr, ()>().unwrap();
    func.call(&mut ctx, array_ptr.addr()).unwrap();
}

#[test]
fn array_find_struct() {
    let mut engine = Engine::new();

    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    struct MyStruct {
        x: i32,
        y: f32,
    }

    fn assert_valid_result(
        caller: Caller<'_, ()>,
        addr: ObjectAddr,
        index: i32,
    ) -> Result<(), Exception> {
        let array = unsafe { caller.copy_array_of_structs::<MyStruct>(addr) };

        assert_eq!(index, 42);
        assert_eq!(array[index as usize].x, 42);
        assert_eq!(array[index as usize].y, 42.0);

        Ok(())
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
    engine.register_native_function("test", "assert_valid_result", assert_valid_result);

    let instance = engine.compile("MyClass", &Options::default());
    let func = instance.get_func("MyFunction").unwrap();

    let struct_layout = engine.get_class_layout("MyClass#MyStruct").unwrap();

    let mut array = engine.object_pool().insert_array(ptr_layout(), 100);
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

fn ptr_layout() -> TypeLayout {
    TypeLayout {
        header: None,
        fields: vec![ObjectField {
            class: "".to_owned(),
            name: "".to_owned(),
            offset: 0,
            ty: FieldType::Pointer,
            ty_name: "".to_owned(),
        }],
        properties: vec![],
    }
}
