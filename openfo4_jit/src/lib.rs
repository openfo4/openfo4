use std::fmt::{self, Debug, Formatter};

use slab::Slab;

use crate::backend::cranelift::{self, CraneliftModule};
use crate::backend::{FunctionDefinitionDesc, StackSlotDesc};

mod backend;

/// Main driver for the JIT compiler.
pub struct Compiler {
    inner: CraneliftModule,
    functions: Slab<FuncState>,
    isa: Isa,
}

impl Compiler {
    /// Creates a new `Compiler`.
    pub fn new() -> Self {
        Self {
            inner: CraneliftModule::new(),
            functions: Slab::new(),
            isa: Isa::new(),
        }
    }

    /// Returns information about the instruction set architecture of the current machine.
    pub fn isa(&self) -> &Isa {
        &self.isa
    }

    /// Declare a new function with the given [`Signature`].
    ///
    /// The function will not have any code associated with it yet. For that call
    /// [`define_function`] or [`define_native_function`].
    ///
    /// [`define_function`]: Self::define_function
    /// [`define_native_function`]: Self::define_native_function
    pub fn declare_function(&mut self, signature: Signature) -> FunctionId {
        let id = self.inner.declare_function(&signature).unwrap();

        FunctionId(self.functions.insert(FuncState { signature, id }))
    }

    /// Defines a native function.
    ///
    /// Code calling the given function will call the function `ptr`.
    ///
    /// # Codegen safety
    ///
    /// - `ptr` must reference a mapped, executable code section that contains the start of a
    ///   function.
    /// - The function must have been defined as `extern "C"`.
    /// - The function must only use `extern "C"`-safe ABI types in parameters and return positions.
    /// - The function must match the [`Signature`] that was passed in [`declare_function`].
    ///
    /// [`declare_function`]: Self::declare_function
    pub fn define_native_function(&mut self, id: FunctionId, ptr: *const u8) {
        let id = self.functions[id.0].id;
        self.inner.define_native_function(id, ptr);
    }

    /// Defines a function.
    pub fn define_function(&mut self, id: FunctionId) -> FunctionBuilder<'_> {
        assert!(
            self.functions.contains(id.0),
            "cannot define function before it was declared"
        );

        FunctionBuilder {
            id,
            compiler: self,
            instructions: Vec::new(),
            next_value: 0,
            next_block: 0,
            next_variable: 0,
            block_params: Vec::new(),
            stack_slots: Slab::new(),
            current_block_full: false,
            current_block: None,
            blocks_filled: Vec::new(),
        }
    }

    /// Compiles a function and all its dependencies.
    pub fn compile_function(&mut self, id: FunctionId) -> CompiledFunction {
        let func = &self.functions[id.0];

        CompiledFunction {
            inner: self.inner.compile_function(func.id, true),
            signature: func.signature.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Isa {
    ptr_size: usize,
    ptr_align: usize,
}

impl Isa {
    const fn new() -> Self {
        Self {
            ptr_size: size_of::<*const ()>(),
            ptr_align: align_of::<*const ()>(),
        }
    }

    pub fn pointer_size(&self) -> usize {
        self.ptr_size
    }

    pub fn pointer_align(&self) -> usize {
        self.ptr_align
    }
}

#[derive(Clone, Debug)]
struct FuncState {
    signature: Signature,
    id: cranelift::FunctionId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    pub params: Vec<Type>,
    pub returns: Vec<Type>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    /// The pointer size of the current architecture.
    Pointer,
}

impl Type {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Pointer => "ptr",
        }
    }

    pub const fn size(&self) -> usize {
        match self {
            Self::I8 => size_of::<i8>(),
            Self::I16 => size_of::<i16>(),
            Self::I32 => size_of::<i32>(),
            Self::I64 => size_of::<i64>(),
            Self::F32 => size_of::<f32>(),
            Self::F64 => size_of::<f64>(),
            Self::Pointer => size_of::<*const ()>(),
        }
    }

    pub const fn align(&self) -> usize {
        match self {
            Self::I8 => align_of::<i8>(),
            Self::I16 => align_of::<i16>(),
            Self::I32 => align_of::<i32>(),
            Self::I64 => align_of::<i64>(),
            Self::F32 => align_of::<f32>(),
            Self::F64 => align_of::<f64>(),
            Self::Pointer => align_of::<*const ()>(),
        }
    }
}

/// An opaque identifier for a function.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(usize);

/// An opaque identifier for an SSA value in a function definition.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Value(usize);

pub struct FunctionBuilder<'a> {
    id: FunctionId,
    compiler: &'a mut Compiler,
    instructions: Vec<Instruction>,
    next_value: usize,
    next_block: usize,
    next_variable: usize,
    stack_slots: Slab<StackSlotDesc>,
    block_params: Vec<Vec<Value>>,
    blocks_filled: Vec<bool>,
    current_block_full: bool,
    current_block: Option<Block>,
}

impl<'a> FunctionBuilder<'a> {
    #[track_caller]
    fn produce_value(&mut self) -> Value {
        debug_assert!(!self.current_block_full);

        let id = self.next_value;
        self.next_value += 1;
        Value(id)
    }

    fn unop<F>(&mut self, src: Value, op: F) -> Value
    where
        F: FnOnce(Unop) -> Instruction,
    {
        debug_assert!(src.0 < self.next_value);
        debug_assert!(!self.current_block_full);

        let dst = self.produce_value();
        self.instructions.push(op(Unop { src, dst }));
        dst
    }

    #[track_caller]
    fn binop<F>(&mut self, lhs: Value, rhs: Value, op: F) -> Value
    where
        F: FnOnce(Binop) -> Instruction,
    {
        debug_assert!(lhs.0 < self.next_value);
        debug_assert!(rhs.0 < self.next_value);
        debug_assert!(!self.current_block_full);

        let dst = self.produce_value();
        self.instructions.push(op(Binop { lhs, rhs, dst }));
        dst
    }

    pub fn create_entry_block(&mut self) -> Block {
        let state = &self.compiler.functions[self.id.0];

        let block = self.next_block;
        self.next_block += 1;

        let params = state.signature.params.clone();
        let values = (0..state.signature.params.len())
            .map(|_| self.produce_value())
            .collect::<Vec<_>>();

        self.instructions.push(Instruction::CreateBlock {
            id: Block(block),
            params,
            values: values.clone(),
            is_entry: true,
        });

        debug_assert_eq!(block, self.block_params.len());
        self.block_params.push(values);
        self.blocks_filled.push(false);

        Block(block)
    }

    pub fn create_block(&mut self, params: &[Type]) -> Block {
        let block = self.next_block;
        self.next_block += 1;

        let values = (0..params.len())
            .map(|_| self.produce_value())
            .collect::<Vec<_>>();

        self.instructions.push(Instruction::CreateBlock {
            id: Block(block),
            params: params.to_vec(),
            values: values.clone(),
            is_entry: false,
        });

        debug_assert_eq!(block, self.block_params.len());
        self.block_params.push(values);
        self.blocks_filled.push(false);

        Block(block)
    }

    pub fn block_params(&self, block: Block) -> &[Value] {
        &self.block_params[block.0]
    }

    pub fn is_block_full(&self, block: Block) -> bool {
        self.blocks_filled[block.0]
    }

    #[track_caller]
    pub fn switch_to_block(&mut self, block: Block) {
        debug_assert!(block.0 < self.next_block);
        debug_assert!(!self.blocks_filled[block.0]);

        if self.current_block == Some(block) {
            return;
        }

        self.instructions.push(Instruction::SwitchBlock(block));
        self.current_block_full = false;
        self.current_block = Some(block);
    }

    pub fn is_current_block_full(&mut self) -> bool {
        self.current_block_full
    }

    /// Create an integer constant.
    #[track_caller]
    pub fn iconst<T: Into<i64>>(&mut self, ty: Type, value: T) -> Value {
        let id = self.produce_value();
        self.instructions
            .push(Instruction::IConst(ty, value.into(), id));
        id
    }

    /// Creates a uninitialized value.
    pub fn uninit(&mut self, ty: Type) -> Value {
        match ty {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::Pointer => self.iconst(ty, 0),
            Type::F32 | Type::F64 => self.fconst(ty, 0.0),
        }
    }

    pub fn fconst<T: Into<f64>>(&mut self, ty: Type, value: T) -> Value {
        let id = self.produce_value();
        self.instructions
            .push(Instruction::FConst(ty, value.into(), id));
        id
    }

    /// Integer addition
    pub fn iadd(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::IAdd)
    }

    /// Integer subtraction
    pub fn isub(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::ISub)
    }

    /// Integer multiplication
    pub fn imul(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::IMul)
    }

    /// Unsigned integer division.
    pub fn udiv(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::UDiv)
    }

    /// Signed integer division.
    pub fn sdiv(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::SDiv)
    }

    pub fn urem(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::URem)
    }

    pub fn srem(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::SRem)
    }

    pub fn ineg(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::INeg)
    }

    /// Floating point addition.
    ///
    /// Performs the floating point addition of `lhs + rhs`. The result is a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be a floating point value.
    /// - `rhs` must be a floating point value.
    pub fn fadd(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::FAdd)
    }

    /// Floating point subtraction.
    ///
    /// Performs the floating point subtraction of `lhs - rhs`. The result is a floating point
    /// value.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be a floating point value.
    /// - `rhs` must be a floating point value.
    pub fn fsub(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::FSub)
    }

    /// Floating point multiplication.
    ///
    /// Performs the floating point multiplication of `lhs * rhs`. The result is a floating point
    /// value.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be a floating point value.
    /// - `rhs` must be a floating point value.
    pub fn fmul(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::FMul)
    }

    /// Floating point division.
    ///
    /// Performs the floating point division of `lhs / rhs`. The result is a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be a floating point value.
    /// - `rhs` must be a floating point value.
    pub fn fdiv(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::FDiv)
    }

    pub fn fmin(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::FMin)
    }

    pub fn fmax(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::FMax)
    }

    pub fn fneg(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::FNeg)
    }

    /// Absolute floating point value
    pub fn fabs(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::FAbs)
    }

    pub fn floor(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::Floor)
    }

    pub fn ceil(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::Ceil)
    }

    #[track_caller]
    pub fn call(&mut self, func: FunctionId, args: &[Value]) -> Vec<Value> {
        let Some(state) = self.compiler.functions.get(func.0) else {
            panic!("cannot call: function {:?} not declared", func);
        };

        let returns: Vec<_> = (0..state.signature.returns.len())
            .map(|_| self.produce_value())
            .collect();

        self.instructions
            .push(Instruction::Call(func, args.to_vec(), returns.clone()));
        returns
    }

    pub fn return_(&mut self, rvals: &[Value]) {
        self.instructions.push(Instruction::Return(rvals.to_vec()));
        self.current_block_full = true;
        self.blocks_filled[self.current_block.unwrap().0] = true;
    }

    /// Unconditional jump.
    ///
    /// Unconditionally jumps to the specified [`Block`] while passing the specified [`Value`]s.
    pub fn jump(&mut self, block: Block, block_args: &[Value]) {
        debug_assert!(block.0 < self.next_block);

        self.instructions
            .push(Instruction::Jump(block, block_args.to_vec()));
        self.current_block_full = true;
        self.blocks_filled[self.current_block.unwrap().0] = true;
    }

    /// Conditional branch.
    ///
    /// Jumps to `true_block` when `condition != 0` and to `false_block` when `condition == 0`.
    pub fn brif(
        &mut self,
        condition: Value,
        true_block: Block,
        true_block_args: &[Value],
        false_block: Block,
        false_block_args: &[Value],
    ) {
        debug_assert!(true_block.0 < self.next_block);
        debug_assert!(false_block.0 < self.next_block);

        self.instructions.push(Instruction::Brif {
            condition,
            true_block,
            true_block_args: true_block_args.to_vec(),
            false_block,
            false_block_args: false_block_args.to_vec(),
        });
        self.current_block_full = true;
        self.blocks_filled[self.current_block.unwrap().0] = true;
    }

    pub fn create_stack_slot(&mut self, size: usize, align: usize) -> StackSlot {
        debug_assert!(align.is_power_of_two());
        StackSlot(self.stack_slots.insert(StackSlotDesc { size, align }))
    }

    pub fn create_typed_stack_slot(&mut self, ty: Type) -> StackSlot {
        let (size, align) = match ty {
            Type::I8 => (size_of::<i8>(), align_of::<i8>()),
            Type::I16 => (size_of::<i16>(), align_of::<i16>()),
            Type::I32 => (size_of::<i32>(), align_of::<i32>()),
            Type::I64 => (size_of::<i64>(), align_of::<i64>()),
            Type::F32 => (size_of::<f32>(), align_of::<f32>()),
            Type::F64 => (size_of::<f64>(), align_of::<f64>()),
            Type::Pointer => (
                self.compiler.isa().pointer_size(),
                self.compiler.isa().pointer_align(),
            ),
        };

        self.create_stack_slot(size, align)
    }

    pub fn stack_load(&mut self, slot: StackSlot, ty: Type, offset: u32) -> Value {
        let dst = self.produce_value();
        self.instructions.push(Instruction::StackLoad {
            ty,
            slot,
            dst,
            offset,
        });
        dst
    }

    pub fn stack_store(&mut self, slot: StackSlot, value: Value, offset: u32) {
        debug_assert!(value.0 < self.next_value);

        self.instructions.push(Instruction::StackStore {
            slot,
            value,
            offset,
        });
    }

    pub fn stack_addr(&mut self, slot: StackSlot, ty: Type, offset: u32) -> Value {
        debug_assert!(self.stack_slots.contains(slot.0));

        let dst = self.produce_value();
        self.instructions.push(Instruction::StackAddr {
            slot,
            ty,
            offset,
            dst,
        });
        dst
    }

    /// Integer comparision.
    ///
    /// Performs the interger comparison of `lhs $op rhs`. If the comparison is evaluates to `true`
    /// a non-zero integer value is returned, otherwise a zero integer value is returned.
    ///
    /// Returns a value of type [`I8`].
    ///
    /// [`I8`]: Type::I8
    pub fn icmp(&mut self, lhs: Value, rhs: Value, op: IntCompareOp) -> Value {
        let dst = self.produce_value();

        self.instructions
            .push(Instruction::ICmp { lhs, rhs, dst, op });
        dst
    }

    pub fn load(&mut self, ty: Type, addr: Value, offset: u32) -> Value {
        let dst = self.produce_value();
        self.instructions.push(Instruction::Load {
            ty,
            addr,
            dst,
            offset,
        });
        dst
    }

    pub fn store(&mut self, value: Value, addr: Value, offset: u32) {
        self.instructions.push(Instruction::Store {
            value,
            addr,
            offset,
        });
    }

    /// Converts a floating point value to an unsigned integer value.
    ///
    /// `NaN` values are mapped to `0`.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be a floating point value.
    /// - `ty` must be an integer type.
    pub fn convert_float_to_uint(&mut self, value: Value, ty: Type) -> Value {
        let dst = self.produce_value();

        self.instructions.push(Instruction::FToU { value, ty, dst });
        dst
    }

    /// Converts a floating point value to a signed integer value.
    ///
    /// `NaN` values are mapped to `0`.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be a floating point value.
    /// - `ty` must be an integer type.
    pub fn convert_float_to_sint(&mut self, value: Value, ty: Type) -> Value {
        let dst = self.produce_value();

        self.instructions.push(Instruction::FToS { value, ty, dst });
        dst
    }

    /// Converts an unsigned integer value to a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be a floating point type.
    pub fn convert_uint_to_float(&mut self, value: Value, ty: Type) -> Value {
        let dst = self.produce_value();

        self.instructions.push(Instruction::UToF { value, ty, dst });
        dst
    }

    /// Converts a signed integer value to a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be a floating point type.
    pub fn convert_sint_to_float(&mut self, value: Value, ty: Type) -> Value {
        let dst = self.produce_value();

        self.instructions.push(Instruction::SToF { value, ty, dst });
        dst
    }

    /// Bitwise and.
    pub fn band(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::BAnd)
    }

    pub fn bor(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::BOr)
    }

    pub fn bxor(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::BXor)
    }

    pub fn bnot(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::BNot)
    }

    pub fn uextend(&mut self, value: Value, ty: Type) -> Value {
        let dst = self.produce_value();
        self.instructions
            .push(Instruction::UExtend { value, dst, ty });
        dst
    }

    pub fn sextend(&mut self, value: Value, ty: Type) -> Value {
        let dst = self.produce_value();
        self.instructions
            .push(Instruction::SExtend { value, dst, ty });
        dst
    }

    pub fn def_var(&mut self, ty: Type, value: Value) -> Variable {
        let var = Variable(self.next_variable);
        self.next_variable += 1;

        self.instructions
            .push(Instruction::DefVar { ty, var, value });

        var
    }

    pub fn set_var(&mut self, var: Variable, value: Value) {
        self.instructions.push(Instruction::SetVar { var, value });
    }

    pub fn get_var(&mut self, var: Variable) -> Value {
        let dst = self.produce_value();

        self.instructions.push(Instruction::GetVar { var, dst });
        dst
    }

    /// Finalize the function definition.
    pub fn finalize(self) {
        let func_id = self.compiler.functions[self.id.0].id;
        self.compiler.inner.define_function(
            func_id,
            FunctionDefinitionDesc {
                instructions: self.instructions,
                num_values: self.next_value,
                num_blocks: self.next_block,
                num_variables: self.next_variable,
                block_params: self.block_params,
                stack_slots: self.stack_slots,
            },
        );
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StackSlot(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(usize);

#[derive(Clone, Debug)]
enum Instruction {
    CreateBlock {
        id: Block,
        params: Vec<Type>,
        values: Vec<Value>,
        is_entry: bool,
    },
    SwitchBlock(Block),
    IConst(Type, i64, Value),
    FConst(Type, f64, Value),
    IAdd(Binop),
    ISub(Binop),
    IMul(Binop),
    SDiv(Binop),
    UDiv(Binop),
    URem(Binop),
    SRem(Binop),
    FAdd(Binop),
    FSub(Binop),
    FMul(Binop),
    FDiv(Binop),
    FMin(Binop),
    FMax(Binop),
    Call(FunctionId, Vec<Value>, Vec<Value>),
    Return(Vec<Value>),
    Jump(Block, Vec<Value>),
    Brif {
        condition: Value,
        true_block: Block,
        true_block_args: Vec<Value>,
        false_block: Block,
        false_block_args: Vec<Value>,
    },
    StackLoad {
        ty: Type,
        slot: StackSlot,
        dst: Value,
        offset: u32,
    },
    StackStore {
        slot: StackSlot,
        value: Value,
        offset: u32,
    },
    StackAddr {
        slot: StackSlot,
        ty: Type,
        dst: Value,
        offset: u32,
    },
    ICmp {
        lhs: Value,
        rhs: Value,
        dst: Value,
        op: IntCompareOp,
    },
    INeg(Unop),
    FNeg(Unop),
    FAbs(Unop),
    Floor(Unop),
    Ceil(Unop),
    Load {
        ty: Type,
        addr: Value,
        dst: Value,
        offset: u32,
    },
    Store {
        value: Value,
        addr: Value,
        offset: u32,
    },
    FToU {
        value: Value,
        ty: Type,
        dst: Value,
    },
    FToS {
        value: Value,
        ty: Type,
        dst: Value,
    },
    UToF {
        value: Value,
        ty: Type,
        dst: Value,
    },
    SToF {
        value: Value,
        ty: Type,
        dst: Value,
    },
    BAnd(Binop),
    BOr(Binop),
    BXor(Binop),
    BNot(Unop),
    UExtend {
        value: Value,
        dst: Value,
        ty: Type,
    },
    SExtend {
        value: Value,
        dst: Value,
        ty: Type,
    },
    DefVar {
        ty: Type,
        var: Variable,
        value: Value,
    },
    SetVar {
        var: Variable,
        value: Value,
    },
    GetVar {
        var: Variable,
        dst: Value,
    },
}

#[derive(Copy, Clone, Debug)]
struct Unop {
    src: Value,
    dst: Value,
}

#[derive(Copy, Clone, Debug)]
struct Binop {
    lhs: Value,
    rhs: Value,
    dst: Value,
}

#[derive(Copy, Clone, Debug)]
pub enum IntCompareOp {
    Equal,
    NotEqual,
    SignedLessThan,
    SignedLessThanOrEqual,
    SignedGreaterThan,
    SignedGreaterThanOrEqual,
    UnsignedLessThan,
    UnsignedLessThanOrEqual,
    UnsignedGreaterThan,
    UnsignedGreaterThanOrEqual,
}

/// A compiled function ready to be called.
#[derive(Clone)]
pub struct CompiledFunction {
    inner: cranelift::CompiledFunction,
    signature: Signature,
}

impl CompiledFunction {
    /// Returns a pointer to the compiled function.
    ///
    /// # Safety
    ///
    /// While this function is safe because it only returns a pointer, using the pointer requires
    /// special care:
    ///
    /// - You must ensure that all previous calls before `compile_function` respect the codegen
    ///   safety.
    /// - You must manually cast this pointer to a function pointer. The signature of the function
    ///   must match the signature that the compiled function was declared.
    /// - You must ensure that the function is safe to call on the current thread.
    /// - Dropping this `CompiledFunction` will deallocate the code that this pointer references.
    ///   When this happens no threads may currently have a callstack that includes this function.
    /// - You must ensure that all native functions referenced remain valid until no threads
    ///   have a callstack including this function.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use openfo4_jit::CompiledFunction;
    /// # use core::mem::transmute;
    /// #
    /// # let compiled_function: CompiledFunction = unreachable!();
    /// #
    /// // Cast the raw pointer to a function pointer.
    /// let fn_ptr = compiled_function.as_ptr();
    /// let fn_ptr = unsafe { transmute::<*const u8, extern "C" fn(i32) -> i32>(fn_ptr) };
    ///
    /// // Call the function.
    /// let result = fn_ptr(42);
    ///
    /// ```
    pub fn as_ptr(&self) -> *const u8 {
        self.inner.as_ptr()
    }

    /// Returns a textual representation of the compiled code.
    ///
    /// This is only available when debug was enabled. The text format is unspecified and only
    /// meant for debugging purposes.
    pub fn text(&self) -> Option<&str> {
        self.inner.text()
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

impl Debug for CompiledFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledFunction")
            .field("ptr", &self.as_ptr())
            .finish_non_exhaustive()
    }
}
