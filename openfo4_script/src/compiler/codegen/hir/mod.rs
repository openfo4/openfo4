//! High-level intermediate representation

mod lowering;
mod text;

use game_common::string::Lowercase;
use game_common::utils::vec_ext::VecExt;

use crate::compiler::codegen::pex::registry::{ClassId, StructId};
use crate::compiler::codegen::trace::Span;
use crate::compiler::codegen::{CodegenData, FunctionId};
use crate::compiler::debug::HirDebugInfo;
use crate::compiler::resources::TypeId;
use crate::runtime::object::ObjectTag;

#[derive(Clone, Debug)]
pub struct Function {
    params: Vec<usize>,
    variables: Vec<TypeId>,
    returns: Option<TypeId>,
    blocks: Vec<Vec<Instruction>>,
}

impl Function {
    pub fn variables(&self) -> impl Iterator<Item = (Variable, TypeId)> {
        self.variables
            .iter()
            .enumerate()
            .map(|(index, ty)| (Variable(index), *ty))
    }

    pub fn debug_text(&self, data: &CodegenData) -> HirDebugInfo {
        let mut buf = String::new();
        let mut regions = Vec::new();
        text::function_text(self, &mut buf, data, &mut regions).unwrap();
        HirDebugInfo { text: buf, regions }
    }

    pub fn lower(&self, data: &CodegenData, builder: openfo4_jit::FunctionBuilder<'_>, span: Span) {
        lowering::lower_function(self, data, builder, span);
    }
}

#[derive(Clone, Debug)]
enum Instruction {
    IAdd(Binop<i32>),
    ISub(Binop<i32>),
    IMul(Binop<i32>),
    IDiv(Binop<i32>),
    IMod(Binop<i32>),
    INeg(Unop<i32>),
    FAdd(Binop<f32>),
    FSub(Binop<f32>),
    FMul(Binop<f32>),
    FDiv(Binop<f32>),
    FNeg(Unop<f32>),
    BNot(Unop<bool>),
    Store {
        src: Value<Literal>,
        dst: Value<Literal>,
    },
    Call(Call),
    CallIndirect(Call),
    Return(Value<Literal>),
    CompareBool(Compare<bool, ScalarCompareOp>),
    CompareFloat(Compare<f32, ScalarCompareOp>),
    CompareInt(Compare<i32, ScalarCompareOp>),
    CompareString(Compare<String, StringCompareOp>),
    CompareAddr(Compare<Null, AddrCompareOp>),
    StringConcat(Binop<String>),
    Jump {
        block: Block,
    },
    Brif {
        condition: Value<bool>,
        invert_condition: bool,
        then_block: Block,
        else_block: Block,
    },
    StructCreate {
        dst: Value<Null>,
        ty: TypeId,
    },
    StructLoad {
        st: Value<Null>,
        dst: Value<Literal>,
        struct_id: StructId,
        field_name: Lowercase<String>,
    },
    StructStore {
        st: Value<Null>,
        value: Value<Literal>,
        struct_id: StructId,
        field_name: Lowercase<String>,
    },
    ClassLoad {
        object: Value<Null>,
        dst: Value<Literal>,
        class_id: ClassId,
        field_name: Lowercase<String>,
    },
    ClassStore {
        object: Value<Null>,
        value: Value<Literal>,
        class_id: ClassId,
        field_name: Lowercase<String>,
    },
    ClassUpcast {
        src: Value<Null>,
        dst: Value<Null>,
        src_class: ClassId,
        dst_class: ClassId,
    },
    ClassDowncast {
        src: Value<Null>,
        dst: Value<Null>,
        src_class: ClassId,
        dst_class: ClassId,
    },
    ClassIsInstanceOf {
        src: Value<Null>,
        dst: Value<bool>,
        class: ClassId,
    },
    ArrayLength {
        array: Value<Null>,
        dst: Value<i32>,
    },
    ArrayGet {
        array: Value<Null>,
        index: Value<i32>,
        dst: Value<Literal>,
    },
    ArraySet {
        array: Value<Null>,
        value: Value<Literal>,
        index: Value<i32>,
    },
    ArrayFind(ArrayFind),
    ArrayFindStruct(ArrayFindStruct),
    ArrayCreate {
        array: Value<Null>,
        len: Value<i32>,
        elem_ty: TypeId,
    },
    ArrayAdd(ArrayAdd),
    ArrayInsert(ArrayInsert),
    ArrayRemove(ArrayRemove),
    ArrayClear {
        array: Value<Null>,
    },
    ConvertIntToFloat {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
    ConvertFloatToInt {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
    ConvertBoolToInt {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
    ConvertBoolToFloat {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
    ConvertIntToBool {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
    ConvertFloatToBool {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
    CreateVar {
        src: Value<Literal>,
        dst: Value<Null>,
        tag: ObjectTag,
        cast_array: bool,
    },
    ExtractVar {
        src: Value<Null>,
        dst: Value<Literal>,
        tag: ObjectTag,
        class: Option<ClassId>,
        cast_array: bool,
    },
    BoolToString {
        src: Value<bool>,
        dst: Value<String>,
    },
    IntToString {
        src: Value<i32>,
        dst: Value<String>,
    },
    FloatToString {
        src: Value<f32>,
        dst: Value<String>,
    },
    ObjectToString {
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Call {
    pub callee: FunctionId,
    pub args: Vec<Value<Literal>>,
    pub object: Option<Value<Null>>,
    pub dst: Value<Literal>,
}

#[derive(Clone, Debug)]
pub struct Compare<T, O> {
    pub lhs: Value<T>,
    pub rhs: Value<T>,
    pub dst: Value<bool>,
    pub op: O,
}

#[derive(Clone, Debug)]
pub struct Unop<T> {
    pub src: Value<T>,
    pub dst: Value<T>,
}

#[derive(Clone, Debug)]
pub struct Binop<T> {
    pub lhs: Value<T>,
    pub rhs: Value<T>,
    pub dst: Value<T>,
}

#[derive(Clone, Debug)]
struct ArrayAdd {
    array: Value<Null>,
    value: Value<Literal>,
    count: Value<i32>,
    elem_ty: TypeId,
}

#[derive(Clone, Debug)]
struct ArrayInsert {
    array: Value<Null>,
    index: Value<i32>,
    value: Value<Literal>,
    elem_ty: TypeId,
}

#[derive(Clone, Debug)]
struct ArrayRemove {
    array: Value<Null>,
    index: Value<i32>,
    count: Value<i32>,
    elem_ty: TypeId,
}

#[derive(Clone, Debug)]
struct ArrayFind {
    array: Value<Null>,
    start_index: Value<i32>,
    searched_value: Value<Literal>,
    scan_backwards: bool,
    write_index: Value<i32>,
    elem_ty: TypeId,
}

#[derive(Clone, Debug)]
struct ArrayFindStruct {
    array: Value<Null>,
    start_index: Value<i32>,
    searched_value: Value<Literal>,
    scan_backwards: bool,
    write_index: Value<i32>,
    struct_ty: StructId,
    struct_member: Lowercase<String>,
}

#[derive(Clone, Debug)]
pub enum Value<T> {
    Const(T),
    Variable(Variable),
    ClassVar {
        object: Variable,
        class: ClassId,
        field: Lowercase<String>,
    },
}

impl<T> Value<T> {
    pub fn map<U, F>(self, f: F) -> Value<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Const(v) => Value::Const(f(v)),
            Self::Variable(v) => Value::Variable(v),
            Self::ClassVar {
                object,
                class,
                field,
            } => Value::ClassVar {
                object,
                class,
                field,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum Literal {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Null,
}

impl Literal {
    pub const fn to_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub const fn to_int(&self) -> Option<i32> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub const fn to_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self {
            Self::String(v) => Some(v.clone()),
            _ => None,
        }
    }

    pub const fn to_null(&self) -> Option<Null> {
        match self {
            Self::Null => Some(Null),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Null;

#[derive(Clone, Debug)]
pub struct FunctionBuilder {
    params: Vec<usize>,
    variables: Vec<TypeId>,
    instructions: Vec<Instruction>,
    returns: Option<TypeId>,
    blocks: Vec<Vec<Instruction>>,
    current_block: Block,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block(usize);

impl FunctionBuilder {
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            variables: Vec::new(),
            instructions: Vec::new(),
            returns: None,
            // Start with entry block.
            blocks: vec![Vec::new()],
            current_block: Block(0),
        }
    }

    pub fn add_param(&mut self, ty: TypeId) -> Variable {
        self.params.push(self.variables.len());
        Variable(self.variables.push_and_index(ty))
    }

    pub fn set_returns(&mut self, ty: TypeId) {
        assert!(self.returns.is_none());
        self.returns = Some(ty);
    }

    pub fn declare_variable(&mut self, ty: TypeId) -> Variable {
        Variable(self.variables.push_and_index(ty))
    }

    pub fn create_block(&mut self) -> Block {
        let block = Block(self.blocks.len());
        self.blocks.push(Vec::new());
        block
    }

    pub fn switch_to_block(&mut self, block: Block) {
        self.blocks[self.current_block.0] = core::mem::take(&mut self.instructions);
        self.current_block = block;
    }

    pub fn jump(&mut self, block: Block) {
        self.instructions.push(Instruction::Jump { block });
    }

    pub fn brif_true(&mut self, condition: Value<bool>, then_block: Block, else_block: Block) {
        self.instructions.push(Instruction::Brif {
            condition,
            invert_condition: false,
            then_block,
            else_block,
        });
    }

    pub fn brif_false(&mut self, condition: Value<bool>, then_block: Block, else_block: Block) {
        self.instructions.push(Instruction::Brif {
            condition,
            invert_condition: true,
            then_block,
            else_block,
        });
    }

    pub fn iadd(&mut self, lhs: Value<i32>, rhs: Value<i32>, dst: Value<i32>) {
        self.instructions
            .push(Instruction::IAdd(Binop { lhs, rhs, dst }));
    }

    pub fn isub(&mut self, lhs: Value<i32>, rhs: Value<i32>, dst: Value<i32>) {
        self.instructions
            .push(Instruction::ISub(Binop { lhs, rhs, dst }));
    }

    pub fn imul(&mut self, lhs: Value<i32>, rhs: Value<i32>, dst: Value<i32>) {
        self.instructions
            .push(Instruction::IMul(Binop { lhs, rhs, dst }));
    }

    pub fn idiv(&mut self, lhs: Value<i32>, rhs: Value<i32>, dst: Value<i32>) {
        self.instructions
            .push(Instruction::IDiv(Binop { lhs, rhs, dst }));
    }

    pub fn imod(&mut self, lhs: Value<i32>, rhs: Value<i32>, dst: Value<i32>) {
        self.instructions
            .push(Instruction::IMod(Binop { lhs, rhs, dst }));
    }

    pub fn ineg(&mut self, src: Value<i32>, dst: Value<i32>) {
        self.instructions.push(Instruction::INeg(Unop { src, dst }));
    }

    pub fn fadd(&mut self, lhs: Value<f32>, rhs: Value<f32>, dst: Value<f32>) {
        self.instructions
            .push(Instruction::FAdd(Binop { lhs, rhs, dst }));
    }

    pub fn fsub(&mut self, lhs: Value<f32>, rhs: Value<f32>, dst: Value<f32>) {
        self.instructions
            .push(Instruction::FSub(Binop { lhs, rhs, dst }));
    }

    pub fn fmul(&mut self, lhs: Value<f32>, rhs: Value<f32>, dst: Value<f32>) {
        self.instructions
            .push(Instruction::FMul(Binop { lhs, rhs, dst }));
    }

    pub fn fdiv(&mut self, lhs: Value<f32>, rhs: Value<f32>, dst: Value<f32>) {
        self.instructions
            .push(Instruction::FDiv(Binop { lhs, rhs, dst }));
    }

    pub fn fneg(&mut self, src: Value<f32>, dst: Value<f32>) {
        self.instructions.push(Instruction::FNeg(Unop { src, dst }));
    }

    pub fn bnot(&mut self, src: Value<bool>, dst: Value<bool>) {
        self.instructions.push(Instruction::BNot(Unop { src, dst }));
    }

    pub fn store(&mut self, src: Value<Literal>, dst: Value<Literal>) {
        self.instructions.push(Instruction::Store { src, dst });
    }

    pub fn call(
        &mut self,
        callee: FunctionId,
        object: Option<Value<Null>>,
        args: &[Value<Literal>],
        dst: Value<Literal>,
    ) {
        self.instructions.push(Instruction::Call(Call {
            callee,
            args: args.to_vec(),
            dst,
            object,
        }));
    }

    /// Calls the given function indirectly.
    ///
    /// - The function pointer is loaded through the `object` value.
    /// - The `object` value is *NOT* automatically inserted into `args`.
    /// - This function will check `object` for `null`.
    pub fn call_indirect(
        &mut self,
        callee: FunctionId,
        object: Option<Value<Null>>,
        args: &[Value<Literal>],
        dst: Value<Literal>,
    ) {
        self.instructions.push(Instruction::CallIndirect(Call {
            callee,
            args: args.to_vec(),
            dst,
            object,
        }));
    }

    pub fn return_(&mut self, value: Value<Literal>) {
        self.instructions.push(Instruction::Return(value));
    }

    pub fn compare_bool(
        &mut self,
        lhs: Value<bool>,
        rhs: Value<bool>,
        dst: Value<bool>,
        op: ScalarCompareOp,
    ) {
        self.instructions
            .push(Instruction::CompareBool(Compare { lhs, rhs, dst, op }));
    }

    pub fn compare_int(
        &mut self,
        lhs: Value<i32>,
        rhs: Value<i32>,
        dst: Value<bool>,
        op: ScalarCompareOp,
    ) {
        self.instructions
            .push(Instruction::CompareInt(Compare { lhs, rhs, dst, op }));
    }

    pub fn compare_float(
        &mut self,
        lhs: Value<f32>,
        rhs: Value<f32>,
        dst: Value<bool>,
        op: ScalarCompareOp,
    ) {
        self.instructions
            .push(Instruction::CompareFloat(Compare { lhs, rhs, dst, op }));
    }

    pub fn compare_string(
        &mut self,
        lhs: Value<String>,
        rhs: Value<String>,
        dst: Value<bool>,
        op: StringCompareOp,
    ) {
        self.instructions
            .push(Instruction::CompareString(Compare { lhs, rhs, dst, op }));
    }

    pub fn compare_addr(
        &mut self,
        lhs: Value<Null>,
        rhs: Value<Null>,
        dst: Value<bool>,
        op: AddrCompareOp,
    ) {
        self.instructions
            .push(Instruction::CompareAddr(Compare { lhs, rhs, dst, op }));
    }

    pub fn struct_create(&mut self, dst: Value<Null>, ty: TypeId) {
        self.instructions
            .push(Instruction::StructCreate { dst, ty });
    }

    pub fn struct_load(
        &mut self,
        st: Value<Null>,
        dst: Value<Literal>,
        struct_id: StructId,
        field_name: Lowercase<String>,
    ) {
        self.instructions.push(Instruction::StructLoad {
            st,
            dst,
            struct_id,
            field_name,
        });
    }

    pub fn struct_store(
        &mut self,
        st: Value<Null>,
        value: Value<Literal>,
        struct_id: StructId,
        field_name: Lowercase<String>,
    ) {
        self.instructions.push(Instruction::StructStore {
            st,
            value,
            struct_id,
            field_name,
        });
    }

    pub fn convert_int_to_float(
        &mut self,
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    ) {
        self.instructions.push(Instruction::ConvertIntToFloat {
            src,
            dst,
            conv_array,
        });
    }

    pub fn convert_float_to_int(
        &mut self,
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    ) {
        self.instructions.push(Instruction::ConvertFloatToInt {
            src,
            dst,
            conv_array,
        });
    }

    pub fn convert_int_to_bool(
        &mut self,
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    ) {
        self.instructions.push(Instruction::ConvertIntToBool {
            src,
            dst,
            conv_array,
        });
    }

    pub fn convert_float_to_bool(
        &mut self,
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    ) {
        self.instructions.push(Instruction::ConvertFloatToBool {
            src,
            dst,
            conv_array,
        });
    }

    pub fn convert_bool_to_int(
        &mut self,
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    ) {
        self.instructions.push(Instruction::ConvertBoolToInt {
            src,
            dst,
            conv_array,
        });
    }

    pub fn convert_bool_to_float(
        &mut self,
        src: Value<Literal>,
        dst: Value<Literal>,
        conv_array: bool,
    ) {
        self.instructions.push(Instruction::ConvertBoolToFloat {
            src,
            dst,
            conv_array,
        });
    }

    pub fn class_load(
        &mut self,
        object: Value<Null>,
        dst: Value<Literal>,
        class: ClassId,
        field: &Lowercase<str>,
    ) {
        self.instructions.push(Instruction::ClassLoad {
            object,
            dst,
            class_id: class,
            field_name: field.to_owned(),
        });
    }

    pub fn class_store(
        &mut self,
        object: Value<Null>,
        value: Value<Literal>,
        class: ClassId,
        field: &Lowercase<str>,
    ) {
        self.instructions.push(Instruction::ClassStore {
            object,
            value,
            class_id: class,
            field_name: field.to_owned(),
        });
    }

    /// Performs an upcast of a class.
    ///
    /// Both `src` and `dst` must be classes and `dst` must be a supertype of `src`.
    /// As such this operation cannot fail.
    pub fn class_upcast(
        &mut self,
        src: Value<Null>,
        dst: Value<Null>,
        src_class: ClassId,
        dst_class: ClassId,
    ) {
        self.instructions.push(Instruction::ClassUpcast {
            src,
            dst,
            src_class,
            dst_class,
        });
    }

    /// Performs a downcast of a class.
    pub fn class_downcast(
        &mut self,
        src: Value<Null>,
        dst: Value<Null>,
        src_class: ClassId,
        dst_class: ClassId,
    ) {
        self.instructions.push(Instruction::ClassDowncast {
            src,
            dst,
            src_class,
            dst_class,
        });
    }

    pub fn class_is_instance_of(&mut self, src: Value<Null>, dst: Value<bool>, class: ClassId) {
        self.instructions
            .push(Instruction::ClassIsInstanceOf { src, dst, class });
    }

    pub fn array_length(&mut self, array: Value<Null>, dst: Value<i32>) {
        self.instructions
            .push(Instruction::ArrayLength { array, dst })
    }

    pub fn array_get(&mut self, array: Value<Null>, index: Value<i32>, dst: Value<Literal>) {
        self.instructions
            .push(Instruction::ArrayGet { array, index, dst });
    }

    pub fn array_set(&mut self, array: Value<Null>, index: Value<i32>, value: Value<Literal>) {
        self.instructions.push(Instruction::ArraySet {
            array,
            value,
            index,
        });
    }

    pub fn array_create(&mut self, len: Value<i32>, dst: Value<Null>, elem_ty: TypeId) {
        self.instructions.push(Instruction::ArrayCreate {
            array: dst,
            len,
            elem_ty,
        })
    }

    pub fn array_add(
        &mut self,
        array: Value<Null>,
        value: Value<Literal>,
        count: Value<i32>,
        elem_ty: TypeId,
    ) {
        self.instructions.push(Instruction::ArrayAdd(ArrayAdd {
            array,
            value,
            count,
            elem_ty,
        }));
    }

    pub fn array_insert(
        &mut self,
        array: Value<Null>,
        value: Value<Literal>,
        index: Value<i32>,
        elem_ty: TypeId,
    ) {
        self.instructions
            .push(Instruction::ArrayInsert(ArrayInsert {
                array,
                value,
                index,
                elem_ty,
            }));
    }

    pub fn array_remove(
        &mut self,
        array: Value<Null>,
        index: Value<i32>,
        count: Value<i32>,
        elem_ty: TypeId,
    ) {
        self.instructions
            .push(Instruction::ArrayRemove(ArrayRemove {
                array,
                index,
                count,
                elem_ty,
            }));
    }

    pub fn array_clear(&mut self, array: Value<Null>) {
        self.instructions.push(Instruction::ArrayClear { array });
    }

    pub fn array_find(
        &mut self,
        array: Value<Null>,
        start_index: Value<i32>,
        searched_value: Value<Literal>,
        scan_backwards: bool,
        write_index: Value<i32>,
        elem_ty: TypeId,
    ) {
        self.instructions.push(Instruction::ArrayFind(ArrayFind {
            array,
            start_index,
            searched_value,
            scan_backwards,
            write_index,
            elem_ty,
        }));
    }

    pub fn array_find_struct(
        &mut self,
        array: Value<Null>,
        start_index: Value<i32>,
        searched_value: Value<Literal>,
        write_index: Value<i32>,
        scan_backwards: bool,
        struct_ty: StructId,
        struct_member: Lowercase<String>,
    ) {
        self.instructions
            .push(Instruction::ArrayFindStruct(ArrayFindStruct {
                array,
                start_index,
                searched_value,
                scan_backwards,
                write_index,
                struct_ty,
                struct_member,
            }));
    }

    pub fn string_concat(&mut self, lhs: Value<String>, rhs: Value<String>, dst: Value<String>) {
        self.instructions
            .push(Instruction::StringConcat(Binop { lhs, rhs, dst }));
    }

    pub fn create_var(
        &mut self,
        src: Value<Literal>,
        dst: Value<Null>,
        tag: ObjectTag,
        cast_array: bool,
    ) {
        self.instructions.push(Instruction::CreateVar {
            src,
            dst,
            tag,
            cast_array,
        });
    }

    pub fn extract_var(
        &mut self,
        src: Value<Null>,
        dst: Value<Literal>,
        tag: ObjectTag,
        class: Option<ClassId>,
        cast_array: bool,
    ) {
        self.instructions.push(Instruction::ExtractVar {
            src,
            dst,
            tag,
            class,
            cast_array,
        });
    }

    pub fn bool_to_string(&mut self, src: Value<bool>, dst: Value<String>) {
        self.instructions
            .push(Instruction::BoolToString { src, dst });
    }

    pub fn int_to_string(&mut self, src: Value<i32>, dst: Value<String>) {
        self.instructions
            .push(Instruction::IntToString { src, dst });
    }

    pub fn float_to_string(&mut self, src: Value<f32>, dst: Value<String>) {
        self.instructions
            .push(Instruction::FloatToString { src, dst });
    }

    pub fn object_to_string(&mut self, src: Value<Literal>, dst: Value<Literal>, conv_array: bool) {
        self.instructions.push(Instruction::ObjectToString {
            src,
            dst,
            conv_array,
        });
    }

    pub fn finalize(mut self) -> Function {
        self.blocks[self.current_block.0] = self.instructions;

        Function {
            params: self.params,
            variables: self.variables,
            returns: self.returns,
            blocks: self.blocks,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarCompareOp {
    Equal,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StringCompareOp {
    Equal,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AddrCompareOp {
    Equal,
    NotEqual,
}
