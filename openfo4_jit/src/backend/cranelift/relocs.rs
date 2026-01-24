use cranelift_codegen::FinalizedRelocTarget;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::ExternalName;
use slab::Slab;

use crate::FuncId;
use crate::backend::cranelift::{FuncState, libcall};

pub fn perform_relocations(id: FuncId, funcs: &mut Slab<FuncState>) {
    let func = &funcs[id.0].compiled.as_ref().unwrap();

    unsafe {
        let src = func.code.buffer.data().as_ptr();
        let dst = func.allocation.as_writable_ptr();
        let count = func.code.buffer.data().len();
        core::ptr::copy_nonoverlapping(src, dst, count);
    }

    for reloc in func.code.buffer.relocs() {
        let base = match reloc.target {
            FinalizedRelocTarget::ExternalName(ExternalName::User(name)) => {
                let id = func.callees.get(&name).unwrap();
                funcs[id.0]
                    .compiled
                    .as_ref()
                    .unwrap()
                    .allocation
                    .as_exec_ptr()
            }
            FinalizedRelocTarget::ExternalName(ExternalName::LibCall(name)) => {
                libcall::get_address(name)
            }
            FinalizedRelocTarget::Func(offset) => unsafe {
                func.allocation
                    .as_exec_ptr()
                    .add(usize::try_from(offset).unwrap())
            },
            _ => unreachable!(),
        };

        let dst = unsafe {
            func.allocation
                .as_writable_ptr()
                .add(reloc.offset.try_into().unwrap())
        };

        let offset = isize::try_from(reloc.addend).unwrap();
        let addr = unsafe { base.offset(offset).addr() };

        match reloc.kind {
            Reloc::Abs4 => {
                let addr = u32::try_from(addr).unwrap();

                unsafe {
                    dst.cast::<u32>().write_unaligned(addr);
                }
            }
            Reloc::Abs8 => {
                let addr = u64::try_from(addr).unwrap();

                unsafe {
                    dst.cast::<u64>().write_unaligned(addr);
                }
            }
            Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => {
                let cur_addr = unsafe {
                    func.allocation
                        .as_exec_ptr()
                        .add(reloc.offset.try_into().unwrap())
                        .addr()
                };

                let pcrel = i32::try_from((addr as isize) - (cur_addr as isize)).unwrap();

                unsafe {
                    dst.cast::<i32>().write_unaligned(pcrel);
                }
            }
            x => todo!("unimplemeted relocation: {:?}", x),
        }
    }

    unsafe {
        let ptr = func.allocation.as_exec_ptr();
        let len = func.code.buffer.data().len();
        wasmtime_internal_jit_icache_coherence::clear_cache(ptr.cast(), len).unwrap();
    }
}
