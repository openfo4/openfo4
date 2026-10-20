use crate::compiler::codegen::hir::lowering::stack::SuspensionPoint;
use crate::compiler::codegen::hir::lowering::{RESULT_TYPE, Translator};
use crate::compiler::codegen::hir::{Null, Value};
use crate::compiler::resources::TypeId;
use crate::runtime::RESULT_SUSPEND;

pub fn lock(
    translator: &mut Translator<'_>,
    object: &Value<Null>,
    ty: TypeId,
    point: &SuspensionPoint,
) {
    with_lock(translator, object, ty, |translator, offset| {
        // The entry block is the block that we resume to when resuming execution.
        // We still need to retry locking if we resumed.
        let entry_block = point.block;
        translator.builder.jump(entry_block, &[]);
        translator.builder.switch_to_block(entry_block);

        // We need to reload the SSA value here because the entry block may
        // be resumed, in which case only the object pointer is saved, but not the
        // lock pointer.
        let object = translator.load(object);
        let lock = translator.builder.ptroffset_imm(object, offset);

        let locked = translator.builder.call(
            translator.data.runtime_funcs.lock,
            &[translator.ctx_ptr, lock],
        )[0];

        let continue_block = translator.builder.create_block(&[]);
        let suspend_block = translator.builder.create_block(&[]);

        translator
            .builder
            .brif(locked, continue_block, &[], suspend_block, &[]);

        translator.builder.switch_to_block(suspend_block);

        let stack_ptr = translator.load_stack_ptr();
        let result = translator.builder.iconst(RESULT_TYPE, RESULT_SUSPEND);
        translator.suspend(point, stack_ptr, result);

        translator.builder.switch_to_block(continue_block);
        translator.current_block = continue_block;
    });
}

pub fn unlock(translator: &mut Translator<'_>, object: &Value<Null>, ty: TypeId) {
    with_lock(translator, object, ty, |translator, offset| {
        let object = translator.load(object);
        let lock = translator.builder.ptroffset_imm(object, offset);

        translator.builder.call(
            translator.data.runtime_funcs.unlock,
            &[translator.ctx_ptr, lock],
        );
    });
}

fn with_lock<F>(translator: &mut Translator<'_>, object: &Value<Null>, ty: TypeId, f: F)
where
    F: FnOnce(&mut Translator<'_>, usize),
{
    let ty = translator.data.resources.types.get(ty).unwrap();
    let Some(offset) = ty.info.lock else {
        return;
    };

    let object = translator.load(object);

    let lock_block = translator.builder.create_block(&[]);
    let merge_block = translator.builder.create_block(&[]);

    translator
        .builder
        .brif(object, lock_block, &[], merge_block, &[]);

    translator.builder.switch_to_block(lock_block);

    f(translator, offset);

    translator.builder.jump(merge_block, &[]);

    translator.builder.switch_to_block(merge_block);
    translator.current_block = merge_block;
}
