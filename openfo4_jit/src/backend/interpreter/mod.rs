mod ops;

use std::cmp::Ordering;
use std::mem::MaybeUninit;
use std::sync::Arc;

use game_common::collections::aligned_box::AlignedBox;
use game_common::collections::vec_map::VecMap;
use parking_lot::Mutex;
use slab::Slab;

use crate::backend::{Backend, FunctionDefinitionDesc, StackSlotDesc};
use crate::func::{HostFunc, MAX_ALIGN};
use crate::{CompilerConfig, FuncId, Instruction, Signature, Type, Value, Word};

pub struct Interpreter {
    functions: Slab<FuncState>,
    rt: Arc<Runtime>,
}

impl Backend for Interpreter {
    type CompiledFunc = CompiledFunc;

    fn new(_config: CompilerConfig) -> Self {
        Self {
            rt: Arc::default(),
            functions: Slab::new(),
        }
    }

    fn declare_function(&mut self, signature: &Signature) -> FuncId {
        let id = self.functions.insert(FuncState {
            signature: signature.clone(),
            defined: false,
        });
        FuncId(id)
    }

    fn define_function(&mut self, id: FuncId, definition: FunctionDefinitionDesc) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(!state.defined);
        state.defined = true;

        let mut blocks = Vec::new();
        let mut entry_block = 0;
        for (index, block) in definition.blocks.into_iter().enumerate() {
            blocks.push(BlockData {
                offset: 0,
                types: block.types,
                values: block.values,
            });

            if block.is_entry {
                entry_block = index;
            }
        }

        for (index, instruction) in definition.instructions.iter().enumerate() {
            match instruction {
                Instruction::SwitchBlock(block) => {
                    blocks[block.0].offset = index;
                }
                _ => {}
            }
        }

        let mut stack_slots = VecMap::new();
        for (slot, desc) in &definition.stack_slots {
            stack_slots.insert(slot, *desc);
        }

        self.rt.functions.lock().insert(
            id.0,
            Arc::new(FuncData {
                instructions: definition.instructions,
                blocks,
                entry_block,
                host: None,
                signature: state.signature.clone(),
                num_values: definition.num_values,
                stack_slots,
            }),
        );
    }

    fn define_host_function(&mut self, id: FuncId, func: HostFunc) {
        let state = self.functions.get_mut(id.0).unwrap();
        assert!(!state.defined);
        state.defined = true;

        self.rt.functions.lock().insert(
            id.0,
            Arc::new(FuncData {
                instructions: Vec::new(),
                blocks: Vec::new(),
                entry_block: 0,
                host: Some(func),
                signature: state.signature.clone(),
                num_values: 0,
                stack_slots: VecMap::new(),
            }),
        );
    }

    fn compile_function(&mut self, id: FuncId) -> Self::CompiledFunc {
        let state = self.functions.get(id.0).unwrap();
        assert!(state.defined);

        let func = Arc::clone(&*self.rt.functions.lock().get(id.0).unwrap());

        CompiledFunc {
            rt: Arc::clone(&self.rt),
            func,
            index: id.0,
        }
    }
}

#[derive(Debug, Default)]
struct Runtime {
    functions: Mutex<VecMap<usize, Arc<FuncData>>>,
}

struct FuncState {
    signature: Signature,
    defined: bool,
}

#[derive(Clone, Debug)]
pub struct CompiledFunc {
    rt: Arc<Runtime>,
    func: Arc<FuncData>,
    index: usize,
}

impl CompiledFunc {
    pub unsafe fn call(&self, params: *const u8, returns: *mut u8) {
        let mut stack_slots = VecMap::new();
        for (index, slot) in &self.func.stack_slots {
            let stack = AlignedBox::new_uninit_slice(slot.size, slot.align);
            stack_slots.insert(index, stack);
        }

        let mut ctx = Context {
            frames: vec![Frame {
                data: self.func.clone(),
                pc: 0,
                values: ValuePool::new(self.func.num_values),
                returns: Returns::Host(returns),
                variables: VecMap::new(),
                stack_slots,
            }],
        };

        unsafe {
            self.load_params(&mut ctx, params);
        }

        while !ctx.frames.is_empty() {
            self.step(&mut ctx);
        }
    }

    unsafe fn load_params(&self, ctx: &mut Context, params: *const u8) {
        let block = &self.func.blocks[self.func.entry_block];

        let mut offset = 0;
        for (ty, value) in block.types.iter().zip(&block.values) {
            offset = (offset + ty.align() - 1) & !(ty.align() - 1);

            let ptr = unsafe { params.add(offset) };

            let word = match ty {
                Type::I8 => Word::from(unsafe { ptr.cast::<i8>().read() }),
                Type::I16 => Word::from(unsafe { ptr.cast::<i16>().read() }),
                Type::I32 => Word::from(unsafe { ptr.cast::<i32>().read() }),
                Type::I64 => Word::from(unsafe { ptr.cast::<i64>().read() }),
                Type::Pointer => Word::from(unsafe { ptr.cast::<isize>().read() }),
                Type::F32 => Word::from(unsafe { ptr.cast::<f32>().read() }),
                Type::F64 => Word::from(unsafe { ptr.cast::<f64>().read() }),
            };

            offset += ty.size();

            ctx.frame().values.insert(*ty, word, *value);
        }
    }

    fn step(&self, ctx: &mut Context) {
        let frame = ctx.frame();

        macro_rules! impl_binop {
            ($op:expr, $func:ident) => {{
                let ty = frame.values.ty($op.lhs);
                let lhs = frame.values.get($op.lhs);
                let rhs = frame.values.get($op.rhs);
                let res = ops::$func(ty, lhs, rhs);
                frame.values.insert(ty, res, $op.dst);
            }};
        }

        macro_rules! impl_unop {
            ($op:expr, $func:ident) => {{
                let ty = frame.values.ty($op.src);
                let src = frame.values.get($op.src);
                let res = ops::$func(ty, src);
                frame.values.insert(ty, res, $op.dst);
            }};
        }

        match &frame.data.instructions[frame.pc] {
            Instruction::SwitchBlock(_) => {}
            Instruction::IConst(ty, word, value) => {
                frame.values.insert(*ty, *word, *value);
            }
            Instruction::FConst(ty, word, value) => {
                frame.values.insert(*ty, *word, *value);
            }
            // Arithmetic
            Instruction::IAdd(op) => impl_binop!(op, iadd),
            Instruction::ISub(op) => impl_binop!(op, isub),
            Instruction::IMul(op) => impl_binop!(op, imul),
            Instruction::SDiv(op) => impl_binop!(op, sdiv),
            Instruction::UDiv(op) => impl_binop!(op, udiv),
            Instruction::SRem(op) => impl_binop!(op, srem),
            Instruction::URem(op) => impl_binop!(op, urem),
            Instruction::SMin(op) => impl_binop!(op, smin),
            Instruction::UMin(op) => impl_binop!(op, umin),
            Instruction::SMax(op) => impl_binop!(op, smax),
            Instruction::UMax(op) => impl_binop!(op, umax),
            Instruction::FAdd(op) => impl_binop!(op, fadd),
            Instruction::FSub(op) => impl_binop!(op, fsub),
            Instruction::FMul(op) => impl_binop!(op, fmul),
            Instruction::FDiv(op) => impl_binop!(op, fdiv),
            Instruction::FMin(op) => impl_binop!(op, fmin),
            Instruction::FMax(op) => impl_binop!(op, fmax),
            Instruction::INeg(op) => impl_unop!(op, ineg),
            Instruction::FNeg(op) => impl_unop!(op, fneg),
            Instruction::FAbs(op) => impl_unop!(op, fabs),
            Instruction::Floor(op) => impl_unop!(op, floor),
            Instruction::Ceil(op) => impl_unop!(op, ceil),
            Instruction::BAnd(op) => impl_binop!(op, band),
            Instruction::BOr(op) => impl_binop!(op, bor),
            Instruction::BXor(op) => impl_binop!(op, bxor),
            Instruction::BNot(op) => impl_unop!(op, bnot),
            Instruction::Popcnt(op) => impl_unop!(op, popcnt),
            Instruction::ICmp { lhs, rhs, dst, op } => {
                let ty = frame.values.ty(*lhs);
                let lhs = frame.values.get(*lhs);
                let rhs = frame.values.get(*rhs);
                let res = ops::icmp(ty, lhs, rhs, *op);
                frame.values.insert(Type::I8, res, *dst);
            }
            Instruction::FCmp { lhs, rhs, dst, op } => {
                let ty = frame.values.ty(*lhs);
                let lhs = frame.values.get(*lhs);
                let rhs = frame.values.get(*rhs);
                let res = ops::fcmp(ty, lhs, rhs, *op);
                frame.values.insert(Type::I8, res, *dst);
            }
            // Conversion
            Instruction::SExtend { value, dst, ty } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::sextend(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            Instruction::UExtend { value, dst, ty } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::uextend(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            Instruction::IReduce { value, dst, ty } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::ireduce(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            Instruction::Bitcast { value, dst, ty } => {
                let value: Word = frame.values.get(*value);
                frame.values.insert(*ty, value, *dst);
            }
            Instruction::FToS { value, ty, dst } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::ftos(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            Instruction::FToU { value, ty, dst } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::ftou(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            Instruction::SToF { value, ty, dst } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::stof(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            Instruction::UToF { value, ty, dst } => {
                let src_ty = frame.values.ty(*value);
                let src = frame.values.get(*value);
                let res = ops::utof(src_ty, src, *ty);
                frame.values.insert(*ty, res, *dst);
            }
            // Memory
            Instruction::Load {
                ty,
                addr,
                dst,
                offset,
            } => {
                let addr: isize = frame.values.get(*addr);
                let offset: isize = (*offset).into();

                let value = unsafe {
                    let ptr = addr as *const u8;
                    let ptr = ptr.add(offset.cast_unsigned());
                    ops::read(*ty, ptr)
                };

                frame.values.insert(*ty, value, *dst);
            }
            Instruction::Store {
                value,
                addr,
                offset,
            } => {
                let addr: isize = frame.values.get(*addr);
                let offset: isize = (*offset).into();

                let ty = frame.values.ty(*value);
                let value = frame.values.get(*value);

                unsafe {
                    let ptr = addr as *mut u8;
                    let ptr = ptr.add(offset.cast_unsigned());
                    ops::write(ty, value, ptr);
                }
            }
            Instruction::StackLoad {
                ty,
                slot,
                dst,
                offset,
            } => {
                let stack = frame.stack_slots.get(slot.0).unwrap();
                let offset = isize::from(*offset).cast_unsigned();

                let ptr = unsafe { stack.as_ptr().add(offset).cast::<u8>() };
                let value = unsafe { ops::read(*ty, ptr) };

                frame.values.insert(*ty, value, *dst);
            }
            Instruction::StackStore {
                slot,
                value,
                offset,
            } => {
                let stack = frame.stack_slots.get_mut(slot.0).unwrap();
                let offset = isize::from(*offset).cast_unsigned();

                let ty = frame.values.ty(*value);
                let value = frame.values.get(*value);

                let ptr = unsafe { stack.as_mut_ptr().add(offset).cast::<u8>() };
                unsafe {
                    ops::write(ty, value, ptr);
                }
            }
            Instruction::StackAddr {
                slot,
                ty: _,
                dst,
                offset,
            } => {
                let stack = frame.stack_slots.get_mut(slot.0).unwrap();
                let offset = isize::from(*offset).cast_unsigned();

                let addr = unsafe { stack.as_mut_ptr().add(offset).expose_provenance() };

                frame.values.insert(Type::Pointer, addr.cast_signed(), *dst);
            }
            // Variables
            Instruction::DefVar { ty: _, var, value } => {
                frame.variables.insert(var.0, *value);
            }
            Instruction::GetVar { var, dst } => {
                let value = *frame.variables.get(var.0).unwrap();
                frame.values.copy(value, *dst);
            }
            Instruction::SetVar { var, value } => {
                frame.variables.insert(var.0, *value);
            }
            // Control Flow
            Instruction::Return(values) => {
                let values = values.to_vec();

                let frame = ctx.frames.pop().unwrap();

                match frame.returns {
                    Returns::Host(base) => {
                        let mut offset = 0;
                        for value in values {
                            let ty = frame.values.ty(value);
                            let value = frame.values.get(value);

                            offset = (offset + ty.align() - 1) & !(ty.align() - 1);

                            unsafe {
                                let ptr = base.add(offset);
                                ops::write(ty, value, ptr);
                            }

                            offset += ty.size();
                        }
                    }
                    Returns::Guest(returns) => {
                        let caller = ctx.frame();

                        for (src, dst) in values.iter().zip(returns) {
                            let ty = frame.values.ty(*src);
                            let word: Word = frame.values.get(*src);
                            caller.values.insert(ty, word, dst);
                        }
                    }
                };

                return;
            }
            Instruction::Jump(block, args) => {
                let block = &self.func.blocks[block.0];
                for (src, dst) in args.iter().zip(&block.values) {
                    frame.values.copy(*src, *dst);
                }

                frame.pc = block.offset;
                return;
            }
            Instruction::Brif {
                condition,
                true_block,
                true_block_args,
                false_block,
                false_block_args,
            } => {
                let condition: Word = frame.values.get(*condition);
                let (block, args) = if condition.0 != 0 {
                    (true_block, true_block_args)
                } else {
                    (false_block, false_block_args)
                };

                let block = &self.func.blocks[block.0];
                for (src, dst) in args.iter().zip(&block.values) {
                    frame.values.copy(*src, *dst);
                }

                frame.pc = block.offset;
                return;
            }
            Instruction::Call(callee, args, returns) => {
                let args = args.to_vec();
                let returns = returns.to_vec();
                let index = callee.0;
                self.call_and_enter(ctx, index, &args, &returns);
                return;
            }
            Instruction::CallIndirect(_, callee, args, returns) => {
                let args = args.to_vec();
                let returns = returns.to_vec();
                let index: isize = frame.values.get(*callee);
                self.call_and_enter(ctx, index.cast_unsigned(), &args, &returns);
                return;
            }
            // Libcall
            Instruction::Memcpy { src, dst, size } => {
                let src: isize = frame.values.get(*src);
                let dst: isize = frame.values.get(*dst);
                let count: isize = frame.values.get(*size);

                let src = src as *const u8;
                let dst = dst as *mut u8;
                let count = count.cast_unsigned();

                unsafe {
                    core::ptr::copy_nonoverlapping(src, dst, count);
                }
            }
            Instruction::Memcmp {
                lhs,
                rhs,
                dst,
                size,
            } => {
                let lhs: isize = frame.values.get(*lhs);
                let rhs: isize = frame.values.get(*rhs);
                let count: isize = frame.values.get(*size);

                let lhs = lhs as *const u8;
                let rhs = rhs as *const u8;
                let count = count.cast_unsigned();

                let res = unsafe {
                    let lhs = core::slice::from_raw_parts(lhs.cast::<u8>(), count);
                    let rhs = core::slice::from_raw_parts(rhs.cast::<u8>(), count);
                    match lhs.cmp(rhs) {
                        Ordering::Equal => 0,
                        Ordering::Greater => 1,
                        Ordering::Less => -1,
                    }
                };

                frame.values.insert(Type::I8, res, *dst);
            }
        }

        frame.pc += 1;
    }

    fn call_and_enter(&self, ctx: &mut Context, index: usize, args: &[Value], returns: &[Value]) {
        let data = Arc::clone(&*self.rt.functions.lock().get(index).unwrap());

        let frame = ctx.frame();
        frame.pc += 1;

        match &data.host {
            Some(func) => {
                let mut stack_params = AlignedBox::<[u8]>::new_uninit_slice(
                    call_abi_size(&data.signature.params),
                    MAX_ALIGN,
                );

                let mut offset = 0;
                for (ty, value) in data.signature.params.iter().zip(args) {
                    offset = (offset + ty.align() - 1) & !(ty.align() - 1);

                    let value = frame.values.get(*value);
                    unsafe {
                        let ptr = stack_params.as_mut_ptr().add(offset).cast::<u8>();
                        ops::write(*ty, value, ptr);
                    }

                    offset += ty.size();
                }

                let mut stack_returns = AlignedBox::<[u8]>::new_uninit_slice(
                    call_abi_size(&data.signature.returns),
                    MAX_ALIGN,
                );

                unsafe {
                    func.call(
                        stack_params.as_ptr().cast::<u8>(),
                        stack_returns.as_mut_ptr().cast::<u8>(),
                    );
                }

                let mut offset = 0;
                for (ty, dst) in data.signature.returns.iter().zip(returns) {
                    offset = (offset + ty.align() - 1) & !(ty.align() - 1);

                    let ptr = unsafe { stack_returns.as_ptr().add(offset).cast::<u8>() };
                    let value = unsafe { ops::read(*ty, ptr) };
                    frame.values.insert(*ty, value, *dst);

                    offset += ty.size();
                }
            }
            None => {
                let mut values = ValuePool::new(data.num_values);

                let block = &data.blocks[data.entry_block];
                for ((src, dst), ty) in args.iter().zip(&block.values).zip(&block.types) {
                    let src: Word = frame.values.get(*src);
                    values.insert(*ty, src, *dst);
                }

                let mut stack_slots = VecMap::new();
                for (index, slot) in &data.stack_slots {
                    let stack = AlignedBox::new_uninit_slice(slot.size, slot.align);
                    stack_slots.insert(index, stack);
                }

                ctx.frames.push(Frame {
                    data,
                    values,
                    pc: 0,
                    returns: Returns::Guest(returns.to_vec()),
                    variables: VecMap::new(),
                    stack_slots,
                });
            }
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Clone, Debug)]
struct ValuePool {
    values: Vec<Option<(Type, Word)>>,
}

impl ValuePool {
    fn new(size: usize) -> Self {
        Self {
            values: vec![const { None }; size],
        }
    }

    fn insert<W>(&mut self, ty: Type, word: W, value: Value)
    where
        W: Into<Word>,
    {
        self.values[value.0] = Some((ty, word.into()));
    }

    fn copy(&mut self, src: Value, dst: Value) {
        self.values[dst.0] = self.values[src.0];
    }

    fn ty(&self, value: Value) -> Type {
        self.values[value.0].unwrap().0
    }

    fn get<T, E>(&self, value: Value) -> T
    where
        T: TryFrom<Word, Error = E>,
        E: std::fmt::Debug,
    {
        let word = self.values[value.0].unwrap().1;
        word.try_into().unwrap()
    }
}

#[derive(Clone, Debug)]
struct FuncData {
    instructions: Vec<Instruction>,
    blocks: Vec<BlockData>,
    entry_block: usize,
    host: Option<HostFunc>,
    signature: Signature,
    num_values: usize,
    stack_slots: VecMap<usize, StackSlotDesc>,
}

#[derive(Clone, Debug)]
struct BlockData {
    /// Offset into the instruction stream.
    offset: usize,
    types: Vec<Type>,
    values: Vec<Value>,
}

#[derive(Clone, Debug)]
struct Frame {
    data: Arc<FuncData>,
    values: ValuePool,
    pc: usize,
    returns: Returns,
    variables: VecMap<usize, Value>,
    stack_slots: VecMap<usize, AlignedBox<[MaybeUninit<u8>]>>,
}

#[derive(Clone, Debug)]
enum Returns {
    Host(*mut u8),
    Guest(Vec<Value>),
}

#[derive(Debug)]
struct Context {
    frames: Vec<Frame>,
}

impl Context {
    fn frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }
}

fn call_abi_size(types: &[Type]) -> usize {
    let mut offset = 0;
    for ty in types {
        offset = (offset + ty.align() - 1) & !(ty.align() - 1);
        offset += ty.size();
    }

    offset
}
