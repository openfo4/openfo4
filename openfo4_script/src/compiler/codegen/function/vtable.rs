use openfo4_jit::Type as JitType;
use openfo4_jit::Value as JitValue;

use crate::compiler::codegen::FunctionDef;
use crate::compiler::codegen::function::FunctionTranslator;

pub fn load_function_addr(
    translator: &mut FunctionTranslator<'_>,
    callee: &FunctionDef,
    object_ptr: JitValue,
) -> JitValue {
    let class = translator.data.classes.get(&callee.class).unwrap();
    let func_index = class.functions.get_index_of(&callee.name).unwrap();

    let mut class_index = 0;
    let mut next_parent = class.parent.as_ref();
    while let Some(parent) = next_parent {
        class_index += 1;
        let class = translator.data.classes.get(parent).unwrap();
        next_parent = class.parent.as_ref();
    }

    let header_ptr = translator.builder.load(JitType::Pointer, object_ptr, 0);

    // 1 usize for Class Count
    // 2 usize for each skipped Class (TypeId, *VTable)
    // 1 usize for the TypeId of our class
    let byte_class_offset =
        size_of::<usize>() + size_of::<usize>() * 2 * class_index + size_of::<usize>();

    let vtable_ptr =
        translator
            .builder
            .load(JitType::Pointer, header_ptr, byte_class_offset as u32);

    let byte_func_offset = size_of::<*const u8>() * func_index;
    let fn_ptr = translator
        .builder
        .load(JitType::Pointer, vtable_ptr, byte_func_offset as u32);

    fn_ptr
}
