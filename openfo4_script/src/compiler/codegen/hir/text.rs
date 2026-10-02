use core::range::Range;
use std::fmt::{Display, Formatter, Result, Write};

use openfo4_jit::SourceLoc;

use crate::compiler::codegen::CodegenData;
use crate::compiler::codegen::hir::{
    AddrCompareOp, Function, Instruction, Literal, Null, ScalarCompareOp, StringCompareOp, Value,
};
use crate::compiler::debug::HirRegion;
use crate::compiler::resources::{TypeId, TypeRepr};

pub fn function_text(
    func: &Function,
    mut writer: &mut String,
    data: &CodegenData,
    regions: &mut Vec<HirRegion>,
) -> Result {
    let ty_to_string = |ty: TypeId| {
        let ty = data.resources.types.get(ty).unwrap();
        match ty.repr {
            TypeRepr::Unit => "unit".to_string(),
            TypeRepr::Int { bits } => format!("i{}", bits),
            TypeRepr::Float { bits } => format!("f{}", bits),
            TypeRepr::Composite(_) => "composite".to_owned(),
            TypeRepr::Pointer(_) => "ptr".to_owned(),
        }
    };

    write!(writer, "func(")?;

    for (index, param) in func.params.iter().enumerate() {
        let ty = func.variables[*param];
        let ty = ty_to_string(ty);

        write!(writer, "%{}: {}", param, ty)?;
        if index + 1 < func.params.len() {
            write!(writer, ", ")?;
        }
    }

    write!(writer, "):")?;

    if let Some(ty) = func.returns {
        let ty = ty_to_string(ty);
        write!(writer, " {}", ty)?;
    }

    writeln!(writer, "")?;

    for (var, ty) in func.variables.iter().enumerate() {
        if func.params.contains(&var) {
            continue;
        }

        let ty = ty_to_string(*ty);

        writeln!(writer, "  %{}: {}", var, ty)?;
    }

    let mut index = 0;
    for (block_index, block) in func.blocks.iter().enumerate() {
        writeln!(writer, "b{}:", block_index)?;

        for instruction in block {
            let start = writer.len();

            write!(writer, "  ")?;
            write_instruction(func, instruction, &mut writer, data)?;

            let end = writer.len();
            regions.push(HirRegion {
                region: Range { start, end },
                loc: Some(SourceLoc::new(index as u32)),
            });
            index += 1;
        }
    }

    Ok(())
}

fn write_instruction<W>(
    func: &Function,
    instruction: &Instruction,
    mut writer: W,
    data: &CodegenData,
) -> Result
where
    W: Write,
{
    let ty_to_string = |ty: TypeId| {
        let ty = data.resources.types.get(ty).unwrap();
        match ty.repr {
            TypeRepr::Unit => "unit".to_string(),
            TypeRepr::Int { bits } => format!("i{}", bits),
            TypeRepr::Float { bits } => format!("f{}", bits),
            TypeRepr::Composite(_) => "composite".to_owned(),
            TypeRepr::Pointer(_) => "ptr".to_owned(),
        }
    };

    match instruction {
        Instruction::IAdd(op) => {
            writeln!(writer, "{} = iadd {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::ISub(op) => {
            writeln!(writer, "{} = isub {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::IMul(op) => {
            writeln!(writer, "{} = imul {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::IDiv(op) => {
            writeln!(writer, "{} = idiv {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::IMod(op) => {
            writeln!(writer, "{} = imod {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::INeg(op) => {
            writeln!(writer, "{} = ineg {}", op.dst, op.src)?;
        }
        Instruction::FAdd(op) => {
            writeln!(writer, "{} = fadd {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::FSub(op) => {
            writeln!(writer, "{} = fsub {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::FMul(op) => {
            writeln!(writer, "{} = fmul {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::FDiv(op) => {
            writeln!(writer, "{} = fdiv {} {}", op.dst, op.lhs, op.rhs)?;
        }
        Instruction::FNeg(op) => {
            writeln!(writer, "{} = fneg {}", op.dst, op.src)?;
        }
        Instruction::BNot(op) => {
            writeln!(writer, "{} = bnot {}", op.dst, op.src)?;
        }
        Instruction::Store { src, dst } => {
            writeln!(writer, "{} = {}", dst, src)?;
        }
        Instruction::CompareBool(op) => {
            writeln!(writer, "{} = bcmp {} {} {}", op.dst, op.op, op.lhs, op.rhs)?;
        }
        Instruction::CompareInt(op) => {
            writeln!(writer, "{} = icmp {} {} {}", op.dst, op.op, op.lhs, op.rhs)?;
        }
        Instruction::CompareFloat(op) => {
            writeln!(writer, "{} = fcmp {} {} {}", op.dst, op.op, op.lhs, op.rhs)?;
        }
        Instruction::CompareString(op) => {
            writeln!(writer, "{} = scmp {} {} {}", op.dst, op.op, op.lhs, op.rhs)?;
        }
        Instruction::CompareAddr(op) => {
            writeln!(writer, "{} = acmp {} {} {}", op.dst, op.op, op.lhs, op.rhs)?;
        }
        Instruction::StringConcat(op) => {
            writeln!(writer, "{} = string_concat {} {}", op.dst, op.lhs, op.rhs)?;
        }

        Instruction::Jump { block } => {
            writeln!(writer, "jmp b{}", block.0)?;
        }
        Instruction::Brif {
            condition,
            then_block,
            else_block,
        } => {
            writeln!(
                writer,
                "brif {} b{} b{}",
                condition, then_block.0, else_block.0,
            )?;
        }
        Instruction::Return(value) => {
            if func.returns.is_some() {
                writeln!(writer, "ret {}", value)?;
            } else {
                writeln!(writer, "ret")?;
            }
        }
        Instruction::Call(op) => {
            write!(writer, "{} = call {:?} ", op.dst, op.callee)?;

            for (index, arg) in op.args.iter().enumerate() {
                write!(writer, "{}", arg)?;

                if index + 1 < op.args.len() {
                    write!(writer, " ")?;
                }
            }

            writeln!(writer, "")?;
        }
        Instruction::CallIndirect(op) => {
            write!(
                writer,
                "{} = call_indirect {} {:?} ",
                op.dst,
                op.object.as_ref().unwrap(),
                op.callee
            )?;

            for (index, arg) in op.args.iter().enumerate() {
                write!(writer, "{}", arg)?;

                if index + 1 < op.args.len() {
                    write!(writer, " ")?;
                }
            }

            writeln!(writer, "")?;
        }
        Instruction::StructCreate { dst, ty } => {
            let ty = ty_to_string(*ty);
            writeln!(writer, "{} = struct_create {}", dst, ty)?;
        }
        Instruction::StructLoad {
            st,
            dst,
            struct_id,
            field_name,
        } => {
            let str = data.registry.struct_(*struct_id).unwrap();
            writeln!(
                writer,
                "{} = struct_load {} {} {}",
                dst, st, str.debug_name, field_name
            )?;
        }
        Instruction::StructStore {
            st,
            value,
            struct_id,
            field_name,
        } => {
            let str = data.registry.struct_(*struct_id).unwrap();
            writeln!(
                writer,
                "struct_store {} {} {} {}",
                st, str.debug_name, field_name, value
            )?;
        }
        Instruction::ClassLoad {
            object,
            dst,
            class_id,
            field_name,
        } => {
            let class = data.registry.class(*class_id).unwrap();
            writeln!(
                writer,
                "{} = class_load {} {} {}",
                dst, object, class.debug_name, field_name
            )?;
        }
        Instruction::ClassStore {
            object,
            value,
            class_id,
            field_name,
        } => {
            let class = data.registry.class(*class_id).unwrap();
            writeln!(
                writer,
                "class_store {} {} {} {}",
                object, class.debug_name, field_name, value
            )?;
        }
        Instruction::ClassUpcast {
            src,
            dst,
            src_class,
            dst_class,
        } => {
            let src_class = data.registry.class(*src_class).unwrap();
            let dst_class = data.registry.class(*dst_class).unwrap();
            writeln!(
                writer,
                "{} = class_upcast {} {} {}",
                dst, src, src_class.debug_name, dst_class.debug_name
            )?;
        }
        Instruction::ClassDowncast {
            src,
            dst,
            src_class,
            dst_class,
        } => {
            let src_class = data.registry.class(*src_class).unwrap();
            let dst_class = data.registry.class(*dst_class).unwrap();
            writeln!(
                writer,
                "{} = class_downcast {} {} {}",
                dst, src, src_class.debug_name, dst_class.debug_name
            )?;
        }
        Instruction::ClassIsInstanceOf { src, dst, class } => {
            let class = data.registry.class(*class).unwrap();
            writeln!(
                writer,
                "{} = class_is_instance_of {} {}",
                dst, src, class.debug_name
            )?;
        }
        Instruction::ArrayCreate {
            array,
            len,
            elem_ty,
        } => {
            writeln!(writer, "{} = array_create {:?} {}", array, elem_ty, len)?;
        }
        Instruction::ArrayLength { array, dst } => {
            writeln!(writer, "{} = array_length {}", dst, array)?;
        }
        Instruction::ArrayGet { array, index, dst } => {
            writeln!(writer, "{} = array_get {} {}", dst, array, index)?;
        }
        Instruction::ArraySet {
            array,
            value,
            index,
        } => {
            writeln!(writer, "array_set {} {} {}", array, index, value)?;
        }
        Instruction::ArrayClear { array } => {
            writeln!(writer, "array_clear {}", array)?;
        }
        Instruction::ArrayAdd(op) => {
            writeln!(writer, "array_add {} {} {}", op.array, op.value, op.count)?;
        }
        Instruction::ArrayInsert(op) => {
            writeln!(
                writer,
                "array_insert {} {} {}",
                op.array, op.index, op.value
            )?;
        }
        Instruction::ArrayRemove(op) => {
            writeln!(
                writer,
                "array_remove {} {} {}",
                op.array, op.index, op.count
            )?;
        }
        Instruction::CreateVar {
            src,
            dst,
            tag,
            cast_array,
        } => {
            writeln!(
                writer,
                "{} = create_var {} {:?} {}",
                dst, src, tag, cast_array
            )?;
        }
        Instruction::ExtractVar {
            src,
            dst,
            tag,
            class,
            cast_array,
        } => {
            writeln!(
                writer,
                "{} = extract_var {} {:?} {:?} {}",
                dst, src, tag, class, cast_array
            )?;
        }
        Instruction::ConvertBoolToInt {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = cvt_bool_to_int {}", dst, src)?;
        }
        Instruction::ConvertBoolToFloat {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = cvt_bool_to_float {}", dst, src)?;
        }
        Instruction::ConvertIntToBool {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = cvt_int_to_bool {}", dst, src)?;
        }
        Instruction::ConvertIntToFloat {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = cvt_int_to_float {}", dst, src)?;
        }
        Instruction::ConvertFloatToBool {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = cvt_float_to_bool {}", dst, src)?;
        }
        Instruction::ConvertFloatToInt {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = cvt_float_to_int {}", dst, src)?;
        }
        Instruction::BoolToString { src, dst } => {
            writeln!(writer, "{} = bool_to_string {}", dst, src)?;
        }
        Instruction::IntToString { src, dst } => {
            writeln!(writer, "{} = int_to_string {}", dst, src)?;
        }
        Instruction::FloatToString { src, dst } => {
            writeln!(writer, "{} = float_to_string {}", dst, src)?;
        }
        Instruction::ObjectToString {
            src,
            dst,
            conv_array: _,
        } => {
            writeln!(writer, "{} = object_to_string {}", dst, src)?;
        }
    }

    Ok(())
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(v) => write!(f, "{}", v),
            Self::Int(v) => write!(f, "{}", v),
            Self::Float(v) => write!(f, "{}f", v),
            Self::String(v) => write!(f, "\"{}\"", v.replace("\"", "\\\"")),
        }
    }
}

impl<T> Display for Value<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Const(v) => write!(f, "{}", v),
            Self::Variable(v) => write!(f, "%{}", v.0),
            Self::ClassVar {
                object,
                class,
                field,
            } => {
                write!(f, "%{}->{:?}->{}", object.0, class, field)
            }
        }
    }
}

impl Display for ScalarCompareOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Equal => f.write_str("eq"),
            Self::LessThan => f.write_str("lt"),
            Self::LessThanOrEqual => f.write_str("le"),
            Self::GreaterThan => f.write_str("gt"),
            Self::GreaterThanOrEqual => f.write_str("ge"),
        }
    }
}

impl Display for StringCompareOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Equal => f.write_str("eq"),
        }
    }
}

impl Display for AddrCompareOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Equal => f.write_str("eq"),
            Self::NotEqual => f.write_str("ne"),
        }
    }
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("null")
    }
}
