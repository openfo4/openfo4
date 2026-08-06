use crate::compiler::codegen::hir::lowering::{ISIZE_TYPE, Translator};
use crate::compiler::resources::TypeId;

use openfo4_jit::Value as JitValue;

pub fn call_malloc(
    translator: &mut Translator<'_>,
    type_id: TypeId,
    array_len: JitValue,
) -> JitValue {
    let type_id = translator.builder.iconst(ISIZE_TYPE, type_id.as_bits());

    translator.builder.call(
        translator.data.runtime_funcs.alloc_zeroed,
        &[translator.ctx_ptr, type_id, array_len],
    )[0]
}
