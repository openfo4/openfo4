pub mod func;

mod backend;

use std::fmt::{self, Debug, Formatter};
use std::ptr;

use openfo4_common::collections::vec_map::{Index, VecMap};

use crate::backend::{Backend, BackendEnum, FunctionDefinitionDesc, TextWithSrcLocs};

pub use crate::backend::SourceLocMap;
use crate::func::IntoHostFunc;

/// Main driver for the JIT compiler.
pub struct Compiler {
    backend: BackendEnum,
    functions: VecMap<FuncId, FuncState>,
    isa: Isa,
}

impl Compiler {
    /// Creates a new `Compiler`.
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            backend: BackendEnum::new(config),
            functions: VecMap::new(),
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
    pub fn declare_function(&mut self, signature: Signature) -> FuncId {
        let id = self.backend.declare_function(&signature);
        self.functions.insert(id, FuncState { signature });
        id
    }

    /// Defines a native function.
    ///
    /// Code calling the given function will call the function `ptr`.
    ///
    /// # Codegen safety
    ///
    /// - The function must match the [`Signature`] that was passed in [`declare_function`].
    ///
    /// [`declare_function`]: Self::declare_function
    pub fn define_native_function<F, Params, Returns>(&mut self, id: FuncId, func: F)
    where
        F: IntoHostFunc<Params, Returns>,
    {
        let func = func.into_host_func();

        self.backend.define_host_function(id, func);
    }

    /// Defines a function.
    pub fn define_function(&mut self, id: FuncId) -> FunctionBuilder<'_> {
        assert!(
            self.functions.contains_key(id),
            "cannot define function before it was declared"
        );

        FunctionBuilder {
            id,
            compiler: self,
            instructions: Vec::new(),
            next_value: 0,
            next_variable: 0,
            current_block_full: false,
            current_block: None,
            blocks_filled: Vec::new(),
            current_srcloc: None,
            srclocs: SourceLocMap::default(),
            value_types: Vec::new(),
            variable_types: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Compiles a function and all its dependencies.
    pub fn compile_function(&mut self, id: FuncId) -> CompiledFunction {
        let func = self.functions.get(id).unwrap();

        CompiledFunction {
            inner: self.backend.compile_function(id),
            signature: func.signature.clone(),
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new(CompilerConfig::default())
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    pub params: Vec<Type>,
    pub returns: Vec<Type>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// An integer type with 8 bits. Equivalent to the [`i8`] type.
    I8,
    /// An integer type with 16 bits. Equivalent to the [`i16`] type.
    I16,
    /// An integer type with 32 bits. Equivalent to the [`i32`] type.
    I32,
    /// An integer type with 64 bits. Equivalent to the [`i64`] type.
    I64,
    /// An integer with the size of the current architecture.
    Isize,
    /// A floating point type with 32 bits. Equivalent to the [`f32`] type.
    F32,
    /// A floating point type with 64 bits. Equivalent to the [`f64`] type.
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
            Self::Isize => "isize",
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
            Self::Isize => size_of::<isize>(),
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
            Self::Isize => align_of::<isize>(),
            Self::F32 => align_of::<f32>(),
            Self::F64 => align_of::<f64>(),
            Self::Pointer => align_of::<*const ()>(),
        }
    }

    /// Returns `true` if the type is an integer type.
    pub const fn is_int(&self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::Isize
        )
    }

    /// Returns `true` if the type is a floating point type.
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    pub const fn is_ptr(&self) -> bool {
        matches!(self, Self::Pointer)
    }
}

/// An opaque identifier for a function.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FuncId(usize);

impl Index for FuncId {
    fn index(&self) -> usize {
        self.0
    }
}

/// An opaque identifier for an SSA value in a function definition.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Value(usize);

pub struct FunctionBuilder<'a> {
    id: FuncId,
    compiler: &'a mut Compiler,
    instructions: Vec<Instruction>,
    next_value: usize,
    next_variable: usize,
    current_block_full: bool,
    current_block: Option<Block>,
    current_srcloc: Option<(SourceLoc, usize)>,
    srclocs: SourceLocMap,
    value_types: Vec<Type>,
    variable_types: Vec<Type>,
    blocks: Vec<BlockProps>,
    blocks_filled: Vec<bool>,
}

impl<'a> FunctionBuilder<'a> {
    #[track_caller]
    fn produce_value(&mut self, ty: Type) -> Value {
        debug_assert!(!self.current_block_full);
        self.produce_value_maybe_full(ty)
    }

    fn produce_value_maybe_full(&mut self, ty: Type) -> Value {
        let id = self.next_value;
        self.next_value += 1;

        self.value_types.push(ty);
        debug_assert_eq!(self.value_types.len(), self.next_value);

        Value(id)
    }

    fn value_type(&self, value: Value) -> Type {
        self.value_types[value.0]
    }

    #[track_caller]
    fn unop<F>(&mut self, src: Value, op: F) -> Value
    where
        F: FnOnce(Unop) -> Instruction,
    {
        debug_assert!(src.0 < self.next_value);
        debug_assert!(!self.current_block_full);
        self.assert_block_not_filled();

        let dst_ty = self.value_type(src);

        let dst = self.produce_value(dst_ty);
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

        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));
        let dst_ty = self.value_type(lhs);

        let dst = self.produce_value(dst_ty);
        self.instructions.push(op(Binop { lhs, rhs, dst }));
        dst
    }

    fn binop_imm<F>(&mut self, lhs: Value, rhs: Imm, op: F) -> Value
    where
        F: FnOnce(Binop) -> Instruction,
    {
        debug_assert!(lhs.0 < self.next_value);
        debug_assert!(!self.current_block_full);

        let ty = self.value_type(lhs);

        let rhs = self.iconst(ty, rhs);

        let dst = self.produce_value(ty);
        self.instructions.push(op(Binop { lhs, rhs, dst }));
        dst
    }

    pub fn create_entry_block(&mut self) -> Block {
        let state = &self.compiler.functions.get(self.id).unwrap();

        let params = state.signature.params.clone();
        let values = state
            .signature
            .params
            // FIXME: Don't clone.
            .clone()
            .iter()
            .map(|ty| self.produce_value_maybe_full(*ty))
            .collect::<Vec<_>>();

        let id = Block(self.blocks.len());
        self.blocks.push(BlockProps {
            id,
            types: params,
            values,
            is_entry: true,
        });

        self.blocks_filled.push(false);

        id
    }

    pub fn create_block(&mut self, params: &[Type]) -> Block {
        let values = params
            .iter()
            .map(|ty| self.produce_value_maybe_full(*ty))
            .collect::<Vec<_>>();

        let id = Block(self.blocks.len());
        self.blocks.push(BlockProps {
            id,
            types: params.to_vec(),
            values,
            is_entry: false,
        });

        self.blocks_filled.push(false);

        id
    }

    pub fn block_params(&self, block: Block) -> &[Value] {
        &self.blocks[block.0].values
    }

    pub fn is_block_full(&self, block: Block) -> bool {
        self.blocks_filled[block.0]
    }

    #[track_caller]
    pub fn switch_to_block(&mut self, block: Block) {
        debug_assert!(block.0 < self.blocks.len());
        debug_assert!(!self.blocks_filled[block.0]);

        if self.current_block == Some(block) {
            return;
        }

        if self.current_block.is_some() {
            debug_assert!(self.current_block_full);
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
    pub fn iconst<T>(&mut self, ty: Type, value: T) -> Value
    where
        T: Into<Imm>,
    {
        self.assert_block_not_filled();
        debug_assert!(ty.is_int());

        let id = self.produce_value(ty);
        let value = value.into().to_word();

        self.instructions.push(Instruction::IConst(ty, value, id));
        id
    }

    /// Creates a new pointer from the given host pointer.
    ///
    /// Returns a value of type [`Pointer`].
    ///
    /// [`Pointer`]: Type::Pointer
    #[track_caller]
    pub fn ptrconst<T>(&mut self, ptr: T) -> Value
    where
        T: IntoPointer,
    {
        self.assert_block_not_filled();

        let id = self.produce_value(Type::Pointer);
        let value = Word(ptr.into_raw());

        self.instructions.push(Instruction::PtrConst(value, id));
        id
    }

    /// Creates a new pointer from the given base `pointer` and the given `offset`.
    ///
    /// The returned pointer will have the address `pointer+offset` and the provenance of `pointer`.
    ///
    /// Returns a value of type [`Pointer`].
    ///
    /// [`Pointer`]: Type::Pointer
    #[track_caller]
    pub fn ptroffset(&mut self, pointer: Value, offset: Value) -> Value {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(pointer).is_ptr());
        debug_assert!(self.value_type(offset).is_int());

        let dst = self.produce_value(Type::Pointer);
        self.instructions.push(Instruction::PtrOffset {
            ptr: pointer,
            offset,
            dst,
        });
        dst
    }

    /// Creates a uninitialized value.
    #[track_caller]
    pub fn uninit(&mut self, ty: Type) -> Value {
        self.assert_block_not_filled();

        match ty {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::Isize => self.iconst(ty, 0),
            Type::F32 | Type::F64 => self.fconst(ty, 0.0),
            Type::Pointer => self.ptrconst(ptr::null::<()>()),
        }
    }

    #[track_caller]
    pub fn fconst<T>(&mut self, ty: Type, value: T) -> Value
    where
        T: Into<ImmFloat>,
    {
        self.assert_block_not_filled();
        debug_assert!(ty.is_float());

        let id = self.produce_value(ty);
        let value = value.into().to_word();

        self.instructions.push(Instruction::FConst(ty, value, id));
        id
    }

    /// Integer addition
    #[track_caller]
    pub fn iadd(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::IAdd)
    }

    /// Integer addition with immediate value.
    ///
    /// Computes and returns `lhs + rhs`.
    #[track_caller]
    pub fn iadd_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::IAdd)
    }

    /// Integer subtraction
    #[track_caller]
    pub fn isub(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::ISub)
    }

    /// Integer subtraction with immediate value.
    ///
    /// Computes and returns `lhs - rhs`.
    #[track_caller]
    pub fn isub_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::ISub)
    }

    /// Integer multiplication
    #[track_caller]
    pub fn imul(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::IMul)
    }

    /// Integer multiplication with immediate value.
    ///
    /// Computes and returns `lhs * rhs`.
    #[track_caller]
    pub fn imul_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::IMul)
    }

    /// Unsigned integer division.
    #[track_caller]
    pub fn udiv(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::UDiv)
    }

    /// Signed integer division.
    #[track_caller]
    pub fn sdiv(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::SDiv)
    }

    #[track_caller]
    pub fn urem(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::URem)
    }

    #[track_caller]
    pub fn srem(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::SRem)
    }

    #[track_caller]
    pub fn ineg(&mut self, value: Value) -> Value {
        debug_assert!(self.value_type(value).is_int());

        self.unop(value, Instruction::INeg)
    }

    /// Signed integer minumum.
    ///
    /// Computes and returns `min(lhs, rhs)`.
    #[track_caller]
    pub fn smin(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::SMin)
    }

    /// Signed integer minumum with immediate.
    ///
    /// Computes and returns `min(lhs, rhs)`.
    #[track_caller]
    pub fn smin_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::SMin)
    }

    /// Signed integer maximum.
    ///
    /// Computes and returns `max(lhs, rhs)`.
    #[track_caller]
    pub fn smax(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::SMax)
    }

    /// Signed integer maximum with immediate.
    ///
    /// Computes and returns `max(lhs, rhs)`.
    #[track_caller]
    pub fn smax_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::SMax)
    }

    /// Unsigned integer minumum.
    ///
    /// Computes and returns `min(lhs, rhs)`.
    #[track_caller]
    pub fn umin(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::UMin)
    }

    /// Unsigned integer minimum with immediate.
    ///
    /// Computes and returns `min(lhs, rhs)`.
    #[track_caller]
    pub fn umin_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::UMin)
    }

    /// Unsigned integer maximum.
    ///
    /// Computes and returns `max(lhs, rhs)`.
    #[track_caller]
    pub fn umax(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_int());
        debug_assert!(self.value_type(rhs).is_int());

        self.binop(lhs, rhs, Instruction::UMax)
    }

    /// Unsigned integer maximum with immediate.
    ///
    /// Computes and returns `max(lhs, rhs)`.
    #[track_caller]
    pub fn umax_imm<T>(&mut self, lhs: Value, rhs: T) -> Value
    where
        T: Into<Imm>,
    {
        debug_assert!(self.value_type(lhs).is_int());

        self.binop_imm(lhs, rhs.into(), Instruction::UMax)
    }

    /// Floating point addition.
    ///
    /// Performs the floating point addition of `lhs + rhs`. The result is a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be a floating point value.
    /// - `rhs` must be a floating point value.
    #[track_caller]
    pub fn fadd(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_float());
        debug_assert!(self.value_type(rhs).is_float());
        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));

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
    #[track_caller]
    pub fn fsub(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_float());
        debug_assert!(self.value_type(rhs).is_float());
        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));

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
    #[track_caller]
    pub fn fmul(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_float());
        debug_assert!(self.value_type(rhs).is_float());
        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));

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
    #[track_caller]
    pub fn fdiv(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_float());
        debug_assert!(self.value_type(rhs).is_float());
        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));

        self.binop(lhs, rhs, Instruction::FDiv)
    }

    #[track_caller]
    pub fn fmin(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_float());
        debug_assert!(self.value_type(rhs).is_float());
        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));

        self.binop(lhs, rhs, Instruction::FMin)
    }

    #[track_caller]
    pub fn fmax(&mut self, lhs: Value, rhs: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_float());
        debug_assert!(self.value_type(rhs).is_float());
        debug_assert_eq!(self.value_type(lhs), self.value_type(rhs));

        self.binop(lhs, rhs, Instruction::FMax)
    }

    #[track_caller]
    pub fn fneg(&mut self, value: Value) -> Value {
        debug_assert!(self.value_type(value).is_float());

        self.unop(value, Instruction::FNeg)
    }

    /// Absolute floating point value
    #[track_caller]
    pub fn fabs(&mut self, value: Value) -> Value {
        debug_assert!(self.value_type(value).is_float());

        self.unop(value, Instruction::FAbs)
    }

    #[track_caller]
    pub fn floor(&mut self, value: Value) -> Value {
        debug_assert!(self.value_type(value).is_float());

        self.unop(value, Instruction::Floor)
    }

    #[track_caller]
    pub fn ceil(&mut self, value: Value) -> Value {
        debug_assert!(self.value_type(value).is_float());

        self.unop(value, Instruction::Ceil)
    }

    #[track_caller]
    pub fn call(&mut self, func: FuncId, args: &[Value]) -> Vec<Value> {
        self.assert_block_not_filled();

        let Some(state) = self.compiler.functions.get(func) else {
            panic!("cannot call: function {:?} not declared", func);
        };

        debug_assert_eq!(state.signature.params.len(), args.len());

        let returns: Vec<_> = state
            .signature
            .returns
            // FIXME: Don't clone.
            .clone()
            .iter()
            .map(|ty| self.produce_value(*ty))
            .collect();

        self.instructions
            .push(Instruction::Call(func, args.to_vec(), returns.clone()));
        returns
    }

    /// Indirect function call.
    #[track_caller]
    pub fn call_indirect(
        &mut self,
        signature: Signature,
        callee: Value,
        args: &[Value],
    ) -> Vec<Value> {
        self.assert_block_not_filled();
        debug_assert_eq!(signature.params.len(), args.len());

        let returns: Vec<_> = signature
            .returns
            .iter()
            .map(|ty| self.produce_value(*ty))
            .collect();

        self.instructions.push(Instruction::CallIndirect(
            signature,
            callee,
            args.to_vec(),
            returns.clone(),
        ));
        returns
    }

    #[track_caller]
    pub fn return_(&mut self, rvals: &[Value]) {
        self.assert_block_not_filled();

        self.instructions.push(Instruction::Return(rvals.to_vec()));
        self.current_block_full = true;
        self.blocks_filled[self.current_block.unwrap().0] = true;
    }

    /// Unconditional jump.
    ///
    /// Unconditionally jumps to the specified [`Block`] while passing the specified [`Value`]s.
    #[track_caller]
    pub fn jump(&mut self, block: Block, block_args: &[Value]) {
        debug_assert!(block.0 < self.blocks.len());
        self.assert_block_not_filled();

        self.instructions
            .push(Instruction::Jump(block, block_args.to_vec()));
        self.current_block_full = true;
        self.blocks_filled[self.current_block.unwrap().0] = true;
    }

    /// Conditional branch.
    ///
    /// Jumps to `true_block` when `condition != 0` and to `false_block` when `condition == 0`.
    #[track_caller]
    pub fn brif(
        &mut self,
        condition: Value,
        true_block: Block,
        true_block_args: &[Value],
        false_block: Block,
        false_block_args: &[Value],
    ) {
        debug_assert!(true_block.0 < self.blocks.len());
        debug_assert!(false_block.0 < self.blocks.len());
        self.assert_block_not_filled();

        assert_eq!(self.blocks[true_block.0].types.len(), true_block_args.len());
        assert_eq!(
            self.blocks[false_block.0].types.len(),
            false_block_args.len()
        );

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

    /// Integer comparision.
    ///
    /// Performs the interger comparison of `lhs $op rhs`. If the comparison is evaluates to `true`
    /// a non-zero integer value is returned, otherwise a zero integer value is returned.
    ///
    /// Returns a value of type [`I8`].
    ///
    /// [`I8`]: Type::I8
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be an integer value.
    /// - `rhs` must be an integer value.
    #[track_caller]
    pub fn icmp(&mut self, lhs: Value, rhs: Value, op: IntCompareOp) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(Type::I8);

        self.instructions
            .push(Instruction::ICmp { lhs, rhs, dst, op });
        dst
    }

    /// Integer comparision with immediate operand.
    ///
    /// Returns a value of type [`I8`].
    ///
    /// [`I8`]: Type::I8
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be an integer value.
    #[track_caller]
    pub fn icmp_imm<T>(&mut self, lhs: Value, rhs: T, op: IntCompareOp) -> Value
    where
        T: Into<Imm>,
    {
        self.assert_block_not_filled();

        let ty = self.value_type(lhs);
        let rhs = self.iconst(ty, rhs);
        let dst = self.produce_value(Type::I8);

        self.instructions
            .push(Instruction::ICmp { lhs, rhs, dst, op });
        dst
    }

    /// Floating point comparison.
    ///
    /// Performs the floating point comparison of `lhs $op rhs`. If the comparison evaluates to
    /// `true` a non-zero integer value is returned, otherwise a zero integer value is returned.
    ///
    /// Returns a value of type [`I8`].
    ///
    /// [`I8`]: Type::I8
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be a floating point value.
    /// - `rhs` must be a floating point value.
    #[track_caller]
    pub fn fcmp(&mut self, lhs: Value, rhs: Value, op: FloatCompareOp) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(Type::I8);

        self.instructions
            .push(Instruction::FCmp { lhs, rhs, dst, op });
        dst
    }

    /// Loads a value from memory address `addr + offset`.
    ///
    /// # Codegen Safety
    ///
    /// - The section `addr + offset..addr + offset + ty.size()` must be mapped as readable memory.
    /// - `addr + offset` must be aligned to `ty`.
    #[track_caller]
    pub fn load<T>(&mut self, ty: Type, addr: Value, offset: T) -> Value
    where
        T: Into<Imm>,
    {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(addr).is_ptr());

        let dst = self.produce_value(ty);
        self.instructions.push(Instruction::Load {
            ty,
            addr,
            dst,
            offset: offset.into().to_word(),
        });
        dst
    }

    /// Stores a value to memory address `addr + offset`.
    ///
    /// # Codegen Safety
    ///
    /// - Section `addr + offset..addr + offset + ty.size()` must be mapped as writable memory.
    /// - `addr + offset` must be aligned to `ty`.
    #[track_caller]
    pub fn store<T>(&mut self, value: Value, addr: Value, offset: T)
    where
        T: Into<Imm>,
    {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(addr).is_ptr());

        self.instructions.push(Instruction::Store {
            value,
            addr,
            offset: offset.into().to_word(),
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
    #[track_caller]
    pub fn convert_float_to_uint(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(ty);

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
    #[track_caller]
    pub fn convert_float_to_sint(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(ty);

        self.instructions.push(Instruction::FToS { value, ty, dst });
        dst
    }

    /// Converts an unsigned integer value to a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be a floating point type.
    #[track_caller]
    pub fn convert_uint_to_float(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(ty);

        self.instructions.push(Instruction::UToF { value, ty, dst });
        dst
    }

    /// Converts a signed integer value to a floating point value.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be a floating point type.
    #[track_caller]
    pub fn convert_sint_to_float(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(ty);

        self.instructions.push(Instruction::SToF { value, ty, dst });
        dst
    }

    /// Bitwise and.
    ///
    /// Computes and returns `lhs & rhs`.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be an integer value.
    /// - `rhs` must be an integer value.
    /// - `lhs` and `rhs` must be of the same type.
    #[track_caller]
    pub fn band(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::BAnd)
    }

    /// Bitwise or.
    ///
    /// Computes and returns `lhs | rhs`.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be an integer value.
    /// - `rhs` must be an integer value.
    /// - `lhs` and `rhs` must be of the same type.
    #[track_caller]
    pub fn bor(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::BOr)
    }

    /// Bitwise xor.
    ///
    /// Computes and returns `lhs ^ rhs`.
    ///
    /// # Codegen Safety
    ///
    /// - `lhs` must be an integer value.
    /// - `rhs` must be an integer value.
    /// - `lhs` and `rhs` must be of the same type.
    #[track_caller]
    pub fn bxor(&mut self, lhs: Value, rhs: Value) -> Value {
        self.binop(lhs, rhs, Instruction::BXor)
    }

    /// Bitwise not.
    ///
    /// Computes and returns `!value`.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    #[track_caller]
    pub fn bnot(&mut self, value: Value) -> Value {
        self.unop(value, Instruction::BNot)
    }

    /// Unsigned integer extension.
    ///
    /// Extends the given [`Value`] to the given [`Type`] by zero extending.
    /// The new [`Type`] must be at least as wide as the current type of the [`Value`].
    /// The new high bits are zeroed.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be at least as wide as the current type of `value`.
    #[track_caller]
    pub fn uextend(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(value).is_int());
        debug_assert!(ty.is_int());

        let dst = self.produce_value(ty);
        self.instructions
            .push(Instruction::UExtend { value, dst, ty });
        dst
    }

    /// Signed integer extension.
    ///
    /// Extends the given [`Value`] to the given [`Type`] by sign extending.
    /// The new [`Type`] must be at least as wide as the current type of the [`Value`].
    /// The new high bits contain the sign bit.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be at least as wide as the current type of `value`.
    #[track_caller]
    pub fn sextend(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(value).is_int());
        debug_assert!(ty.is_int());

        let dst = self.produce_value(ty);
        self.instructions
            .push(Instruction::SExtend { value, dst, ty });
        dst
    }

    /// Reduce the integer witdth of a value by discarding the most significant bits.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must be an integer value.
    /// - `ty` must be at most as wide as the current type of `value`.
    #[track_caller]
    pub fn ireduce(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(value).is_int());
        debug_assert!(ty.is_int());

        let dst = self.produce_value(ty);
        self.instructions
            .push(Instruction::IReduce { value, dst, ty });
        dst
    }

    /// Reinterpret the bits of a value as a different type.
    ///
    /// # Codegen Safety
    ///
    /// - Both types must have the same size.
    /// - The new type must have an aligned less than or equal to the current type.
    #[track_caller]
    pub fn bitcast(&mut self, value: Value, ty: Type) -> Value {
        self.assert_block_not_filled();

        let dst = self.produce_value(ty);
        self.instructions
            .push(Instruction::Bitcast { value, dst, ty });
        dst
    }

    /// Returns the number of ones in the integer value.
    ///
    /// Returns a value with the same type as the input `value`.
    ///
    /// # Codegen Safety
    ///
    /// - `value` must an integer value.
    #[track_caller]
    pub fn popcnt(&mut self, value: Value) -> Value {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(value).is_int());

        self.unop(value, Instruction::Popcnt)
    }

    pub fn def_var(&mut self, ty: Type, value: Value) -> Variable {
        let var = Variable(self.next_variable);
        self.next_variable += 1;

        self.variable_types.push(ty);
        debug_assert_eq!(self.variable_types.len(), self.next_variable);

        self.instructions
            .push(Instruction::DefVar { ty, var, value });

        var
    }

    #[track_caller]
    pub fn set_var(&mut self, var: Variable, value: Value) {
        self.instructions.push(Instruction::SetVar { var, value });
    }

    #[track_caller]
    pub fn get_var(&mut self, var: Variable) -> Value {
        let ty = self.variable_types[var.0];
        let dst = self.produce_value(ty);

        self.instructions.push(Instruction::GetVar { var, dst });
        dst
    }

    /// Copies `size` bytes from `src..src+size` to `dst..dst+size`.
    ///
    /// # Codegen Safety
    ///
    /// - `src` must be of type [`Pointer`].
    /// - `dst` must be of type [`Pointer`].
    ///
    /// [`Pointer`]: Type::Pointer
    pub fn memcpy(&mut self, src: Value, dst: Value, size: Value) {
        self.assert_block_not_filled();
        debug_assert!(self.value_type(src).is_ptr());
        debug_assert!(self.value_type(dst).is_ptr());

        self.instructions
            .push(Instruction::Memcpy { src, dst, size });
    }

    /// Compares `size` bytes from `lhs..lhs+size` to `rhs..rhs+size`.
    ///
    /// Returns a value of type [`I8`].
    ///
    /// # Codegen Safety
    ///
    /// - `src` must be of type [`Pointer`].
    /// - `dst` must be of type [`Pointer`].
    ///
    /// [`I8`]: Type::I8
    /// [`Pointer`]: Type::Pointer
    pub fn memcmp(&mut self, lhs: Value, rhs: Value, size: Value) -> Value {
        debug_assert!(self.value_type(lhs).is_ptr());
        debug_assert!(self.value_type(rhs).is_ptr());

        let dst = self.produce_value(Type::I8);

        self.instructions.push(Instruction::Memcmp {
            lhs,
            rhs,
            size,
            dst,
        });
        dst
    }

    /// Finalize the function definition.
    pub fn finalize(self) {
        self.compiler.backend.define_function(
            self.id,
            FunctionDefinitionDesc {
                instructions: self.instructions,
                num_values: self.next_value,
                num_variables: self.next_variable,
                srcloc: self.srclocs,
                blocks: self.blocks,
            },
        );
    }

    #[track_caller]
    #[inline]
    fn assert_block_not_filled(&self) {
        debug_assert!(!self.current_block_full);
    }

    /// Begins a source location region.
    ///
    /// All following instructions will be tagged with the given [`SourceLoc`].
    /// If another region is still active, it is closed before this region opened.
    pub fn begin_srcloc(&mut self, loc: SourceLoc) {
        self.end_srcloc();
        let start = self.instructions.len();
        self.current_srcloc = Some((loc, start));
    }

    /// Ends a source location region.
    ///
    /// Does nothing if no [`SourceLoc`] is currently open.
    pub fn end_srcloc(&mut self) {
        if let Some((loc, start)) = self.current_srcloc.take() {
            let end = self.instructions.len();
            self.srclocs.insert(start, end, loc);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block(usize);

impl Block {
    /// A placeholder value that does not refer to a valid block.
    pub const DANGLING: Self = Self(usize::MAX);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(usize);

#[derive(Clone, Debug)]
enum Instruction {
    SwitchBlock(Block),
    IConst(Type, Word, Value),
    FConst(Type, Word, Value),
    PtrConst(Word, Value),
    PtrOffset {
        ptr: Value,
        offset: Value,
        dst: Value,
    },
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
    Call(FuncId, Vec<Value>, Vec<Value>),
    CallIndirect(Signature, Value, Vec<Value>, Vec<Value>),
    Return(Vec<Value>),
    Jump(Block, Vec<Value>),
    Brif {
        condition: Value,
        true_block: Block,
        true_block_args: Vec<Value>,
        false_block: Block,
        false_block_args: Vec<Value>,
    },
    ICmp {
        lhs: Value,
        rhs: Value,
        dst: Value,
        op: IntCompareOp,
    },
    FCmp {
        lhs: Value,
        rhs: Value,
        dst: Value,
        op: FloatCompareOp,
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
        offset: Word,
    },
    Store {
        value: Value,
        addr: Value,
        offset: Word,
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
    IReduce {
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
    Bitcast {
        value: Value,
        dst: Value,
        ty: Type,
    },
    Popcnt(Unop),
    Memcpy {
        src: Value,
        dst: Value,
        size: Value,
    },
    Memcmp {
        lhs: Value,
        rhs: Value,
        dst: Value,
        size: Value,
    },
    SMin(Binop),
    SMax(Binop),
    UMin(Binop),
    UMax(Binop),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FloatCompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// A compiled function ready to be called.
#[derive(Clone)]
pub struct CompiledFunction {
    inner: backend::CompiledFunc,
    signature: Signature,
}

impl CompiledFunction {
    /// Calls the guest function.
    ///
    /// The `params` pointer contains the input elements read from the function body.
    /// The `returns` pointer contains the output elements written by the function body.
    ///
    /// ## ABI
    ///
    /// The `params` and `returns` pointers must match the [`Signature`] used while building this
    /// function. `params` and `returns` share the same ABI, except that `params` must be readable
    /// and `returns` must be writable.
    ///
    /// We consider a pointer valid for a [`Type`] when it is non-null and well-aligned.
    /// If the pointer is readable the bytes of a value must also be initialized and contain a
    /// valid bit pattern for the [`Type`].
    ///
    /// The first value is placed at offset 0, all following values are placed at offset
    /// `align_up(offset(i - 1) + size(i - 1), align(i))`, where:
    /// - `offset(i)` is the offset of the value at index `i`,
    /// - `size(i)` is the size in bytes of the value at index `i`,
    /// - `align(i)` is the alignment of the value at index `i` and
    /// - `align_up` returns the next multiple of the given alignment.
    ///
    /// In addition both pointers must be aligned to the maximum of any [`Type`] that the pointer
    /// points to. In other words the pointer must be aligned to `max(align(elem))` over all
    /// elements.
    ///
    /// `returns` may point to uninitialized memory as long as it is still writable for all return
    /// values. Whether a value is initialized after the function returns depends on the code of
    /// the function.
    ///
    /// # Safety
    ///
    /// - The *Codegen Safety* requirements must hold for every instruction inserted into this
    ///   function and all functions called by this function recursively. This includes indirect
    ///   function calls.
    /// - `params` must be non-null and valid for reads according to the ABI described above.
    /// - `returns` must be non-null and valie for writes according to the ABI described above.
    ///
    pub unsafe fn call(&self, params: *const u8, returns: *mut u8) {
        debug_assert!(!params.is_null());
        debug_assert!(!returns.is_null());

        if cfg!(debug_assertions) {
            for ty in &self.signature.params {
                debug_assert_eq!(params.align_offset(ty.align()), 0);
            }

            for ty in &self.signature.returns {
                debug_assert_eq!(returns.align_offset(ty.align()), 0);
            }
        }

        // SAFETY: The caller guarantees that the function was soundly constructed
        // and the arguments are valid.
        unsafe {
            self.inner.call(params, returns);
        }
    }

    /// Returns a pointer to the compiled function.
    pub fn as_ptr(&self) -> *const u8 {
        self.inner.as_ptr()
    }

    /// Returns a textual representation of the compiled code.
    ///
    /// This is only available when debug was enabled. The text format is unspecified and only
    /// meant for debugging purposes.
    pub fn text(&self) -> Option<&TextWithSrcLocs<String>> {
        self.inner.clif()
    }

    /// Returns the disassembly of the generated machine code.
    ///
    /// Only available on the [`Cranelift`] backend and when `debug` is enabled.
    ///
    /// [`Cranelift`]: BackendKind::Cranelift
    pub fn disasm(&self) -> Option<&TextWithSrcLocs<String>> {
        self.inner.disasm()
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

#[derive(Clone, Debug, Default)]
pub struct CompilerConfig {
    pub backend: BackendKind,
    pub validation: bool,
    pub debug: bool,
}

/// A source location attached to code regions for debugging output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceLoc(u32);

impl SourceLoc {
    pub fn new(loc: u32) -> Self {
        Self(loc + 1)
    }
}

/// An immediate integer value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Imm {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Isize(isize),
}

impl Imm {
    fn to_word(&self) -> Word {
        let val: i64 = match *self {
            Self::I8(v) => v.into(),
            Self::I16(v) => v.into(),
            Self::I32(v) => v.into(),
            Self::I64(v) => v.into(),
            Self::Isize(v) => {
                // Ensure our isize always fits into Word,
                // meaning the following cast will be guaranteed
                // to be lossless.
                const {
                    assert!(size_of::<isize>() <= size_of::<Word>());
                }

                v as _
            }
        };

        let val: isize = val.try_into().unwrap();
        let val = val.cast_unsigned();

        Word(ptr::without_provenance_mut(val))
    }
}

macro_rules! impl_int_imm_from {
    ($signed:ty, $unsigned:ty, $variant:ident) => {
        impl From<$signed> for Imm {
            #[inline]
            fn from(value: $signed) -> Self {
                Self::$variant(value)
            }
        }

        impl From<$unsigned> for Imm {
            #[inline]
            fn from(value: $unsigned) -> Self {
                Self::$variant(value.cast_signed())
            }
        }
    };
}

impl_int_imm_from!(i8, u8, I8);
impl_int_imm_from!(i16, u16, I16);
impl_int_imm_from!(i32, u32, I32);
impl_int_imm_from!(i64, u64, I64);
impl_int_imm_from!(isize, usize, Isize);

/// An immediate floating point value.
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum ImmFloat {
    F32(f32),
    F64(f64),
}

impl ImmFloat {
    fn to_word(&self) -> Word {
        match self {
            Self::F32(v) => v.to_bits().cast_signed().into(),
            Self::F64(v) => v.to_bits().cast_signed().into(),
        }
    }
}

impl From<f32> for ImmFloat {
    fn from(value: f32) -> Self {
        Self::F32(value)
    }
}

impl From<f64> for ImmFloat {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}

/// A immediate value.
// A word needs to be represented as pointer internally to preserve
// provenance. All integer values always have an invalid provenance.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
struct Word(*mut ());

// From our point of view a word is only a bunch of bits.
// How they are used is not our problem and the consumer
// must ensure Send and Sync requirements are satisfied as needed.
unsafe impl Send for Word {}
unsafe impl Sync for Word {}

impl From<Word> for i64 {
    fn from(value: Word) -> Self {
        isize::from(value).try_into().unwrap()
    }
}

impl From<Word> for isize {
    fn from(value: Word) -> Self {
        value.0.addr().cast_signed()
    }
}

impl From<Word> for *mut () {
    fn from(value: Word) -> Self {
        value.0
    }
}

impl TryFrom<Word> for i8 {
    type Error = ();

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        i64::from(value).try_into().map_err(|_| ())
    }
}

impl TryFrom<Word> for i16 {
    type Error = ();

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        i64::from(value).try_into().map_err(|_| ())
    }
}

impl TryFrom<Word> for i32 {
    type Error = ();

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        i64::from(value).try_into().map_err(|_| ())
    }
}

impl TryFrom<Word> for f32 {
    type Error = <i32 as TryFrom<Word>>::Error;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        i32::try_from(value)
            .map(i32::cast_unsigned)
            .map(f32::from_bits)
    }
}

impl TryFrom<Word> for f64 {
    type Error = <i64 as TryFrom<Word>>::Error;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        i64::try_from(value)
            .map(i64::cast_unsigned)
            .map(f64::from_bits)
    }
}

impl From<i8> for Word {
    fn from(value: i8) -> Self {
        Imm::I8(value).to_word()
    }
}

impl From<i16> for Word {
    fn from(value: i16) -> Self {
        Imm::I16(value).to_word()
    }
}

impl From<i32> for Word {
    fn from(value: i32) -> Self {
        Imm::I32(value).to_word()
    }
}

impl From<i64> for Word {
    fn from(value: i64) -> Self {
        Imm::I64(value).to_word()
    }
}

impl From<isize> for Word {
    fn from(value: isize) -> Self {
        Imm::Isize(value).to_word()
    }
}

impl From<f32> for Word {
    fn from(value: f32) -> Self {
        Imm::I32(value.to_bits().cast_signed()).to_word()
    }
}

impl From<f64> for Word {
    fn from(value: f64) -> Self {
        Imm::I64(value.to_bits().cast_signed()).to_word()
    }
}

#[derive(Clone, Debug)]
struct BlockProps {
    id: Block,
    types: Vec<Type>,
    values: Vec<Value>,
    is_entry: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Interpreter,
    #[cfg(feature = "cranelift")]
    Cranelift,
}

impl Default for BackendKind {
    fn default() -> Self {
        cfg_select! {
            // Miri does not work with JIT backends.
            // It does not allow mapping memory as EXEC.
            miri => Self::Interpreter,
            feature = "cranelift" => Self::Cranelift,
            _ => Self::Interpreter,
        }
    }
}

/// A pointer-like type.
pub trait IntoPointer {
    /// Converts this pointer into a untyped raw pointer.
    fn into_raw(self) -> *mut ();
}

impl<T> IntoPointer for *const T {
    fn into_raw(self) -> *mut () {
        self.cast_mut().cast()
    }
}

impl<T> IntoPointer for *mut T {
    fn into_raw(self) -> *mut () {
        self.cast()
    }
}
