use openfo4_jit::{Compiler, CompilerConfig, Signature, Type};

#[test]
fn call_return_value() {
    let mut compiler = Compiler::new(CompilerConfig::default());
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

    let mut returns: i32 = 0;
    unsafe {
        compiled_fn.call(core::ptr::dangling(), (&raw mut returns).cast::<u8>());
    }

    assert_eq!(returns, 1);
}

#[test]
fn call_native() {
    fn my_native_func(x: *mut i32) {
        unsafe {
            *x += 1;
        }
    }

    let mut compiler = Compiler::new(CompilerConfig::default());
    let host_fn = compiler.declare_function(Signature {
        params: vec![Type::Pointer],
        returns: vec![],
    });
    let guest_fn = compiler.declare_function(Signature {
        params: vec![Type::Pointer],
        returns: vec![],
    });

    compiler.define_native_function(host_fn, my_native_func);
    let mut builder = compiler.define_function(guest_fn);

    let entry = builder.create_entry_block();
    builder.switch_to_block(entry);
    let args = builder.block_params(entry);

    builder.call(host_fn, &[args[0]]);
    builder.return_(&[]);

    builder.finalize();

    let compiled_fn = compiler.compile_function(guest_fn);

    let mut value: i32 = 0;
    let value_ptr = &raw mut value;

    unsafe {
        compiled_fn.call(
            (&raw const value_ptr).cast::<u8>(),
            core::ptr::dangling_mut(),
        );
    }

    assert_eq!(value, 1);
}

#[test]
fn call_native_from_host() {
    fn my_native_func(a: u8, b: u64) {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }

    #[repr(C)]
    union Param {
        a: u8,
        b: u64,
    }

    let mut compiler = Compiler::new(CompilerConfig::default());
    let host_fn = compiler.declare_function(Signature {
        params: vec![Type::I8, Type::I64],
        returns: vec![],
    });

    compiler.define_native_function(host_fn, my_native_func);

    let compiled_fn = compiler.compile_function(host_fn);

    let params = [Param { a: 1 }, Param { b: 2 }];

    unsafe {
        compiled_fn.call((&raw const params).cast::<u8>(), core::ptr::dangling_mut());
    }
}
