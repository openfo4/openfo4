use openfo4_jit::{Compiler, Signature, Type};

#[test]
fn call_return_value() {
    let mut compiler = Compiler::new();
    let guest_fn = compiler.declare_function(Signature {
        params: vec![],
        returns: vec![Type::I32],
    });

    let mut builder = compiler.define_function(guest_fn);

    let entry = builder.create_entry_block();
    builder.switch_to_block(entry);

    let result = builder.iconst(Type::I32, 1);
    builder.return_(&[result]);
    builder.finalize();

    let compiled_fn = compiler.compile_function(guest_fn);

    let fn_ptr =
        unsafe { core::mem::transmute::<*const u8, extern "C" fn() -> i32>(compiled_fn.as_ptr()) };

    assert_eq!(fn_ptr(), 1);
}

#[test]
fn call_native() {
    unsafe extern "C" fn my_native_func(x: *mut i32) {
        unsafe {
            *x += 1;
        }
    }

    let mut compiler = Compiler::new();
    let host_fn = compiler.declare_function(Signature {
        params: vec![Type::Pointer],
        returns: vec![],
    });
    let guest_fn = compiler.declare_function(Signature {
        params: vec![Type::Pointer],
        returns: vec![],
    });

    compiler.define_native_function(host_fn, my_native_func as *const u8);
    let mut builder = compiler.define_function(guest_fn);

    let entry = builder.create_entry_block();
    builder.switch_to_block(entry);
    let args = builder.block_params(entry);

    builder.call(host_fn, &[args[0]]);
    builder.return_(&[]);

    builder.finalize();

    let compiled_fn = compiler.compile_function(guest_fn);

    let fn_ptr =
        unsafe { core::mem::transmute::<*const u8, extern "C" fn(*mut i32)>(compiled_fn.as_ptr()) };

    let mut value = 0;
    fn_ptr(&mut value);

    assert_eq!(value, 1);
}
