use std::fmt::{Error, Write};

use crate::{Function, Instruction, Value};

pub(crate) fn for_function(f: &Function) -> String {
    let mut text = String::new();

    for instruction in &f.instructions {
        for_instruction(instruction, &mut text).unwrap();
        text.push_str("\n");
    }

    text
}

pub(crate) fn for_instruction<W>(instruction: &Instruction, mut text: W) -> Result<(), Error>
where
    W: Write,
{
    match instruction {
        Instruction::Nop => {
            write!(text, "nop")?;
        }
        Instruction::IAdd(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = iadd {lhs} {rhs}")?;
        }
        Instruction::ISub(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = isub {lhs} {rhs}")?;
        }
        Instruction::IMul(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = imul {lhs} {rhs}")?;
        }
        Instruction::IDiv(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = idiv {lhs} {rhs}")?;
        }
        Instruction::IMod(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = imod {lhs} {rhs}")?;
        }
        Instruction::INeg(op) => {
            let src = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = ineg {src}")?;
        }
        Instruction::FAdd(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = fadd {lhs} {rhs}")?;
        }
        Instruction::FSub(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = fsub {lhs} {rhs}")?;
        }
        Instruction::FMul(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = fmul {lhs} {rhs}")?;
        }
        Instruction::FDiv(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = fdiv {lhs} {rhs}")?;
        }
        Instruction::FNeg(op) => {
            let src = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = fneg {src}")?;
        }
        Instruction::CmpEq(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = cmpeq {lhs} {rhs}")?;
        }
        Instruction::CmpLt(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = cmplt {lhs} {rhs}")?;
        }
        Instruction::CmpLe(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = cmple {lhs} {rhs}")?;
        }
        Instruction::CmpGt(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = cmpgt {lhs} {rhs}")?;
        }
        Instruction::CmpGe(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = cmpge {lhs} {rhs}")?;
        }
        Instruction::Store(op) => {
            let src = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = {src}")?;
        }
        Instruction::Return(op) => {
            let value = for_value(op);
            write!(text, "return {value}")?;
        }
        Instruction::Branch(offset) => {
            let offset = for_value(offset);
            write!(text, "branch {offset}")?;
        }
        Instruction::BranchIfTrue(op) => {
            let condition = for_value(&op.condition);
            let offset = for_value(&op.offset);
            write!(text, "branch_if_true {condition} {offset}")?;
        }
        Instruction::BranchIfFalse(op) => {
            let condition = for_value(&op.condition);
            let offset = for_value(&op.offset);
            write!(text, "branch_if_false {condition} {offset}")?;
        }
        Instruction::CallMethod(op) => {
            let object = for_value(&op.object);
            let function_name = for_value(&op.function_name);
            let return_assignment = for_value(&op.return_assignment);
            let args = op
                .args
                .iter()
                .map(|v| for_value(v))
                .collect::<Vec<_>>()
                .join(" ");

            write!(
                text,
                "{return_assignment} = call {object}.{function_name} {args}"
            )?;
        }
        Instruction::CallParent(op) => {
            let func = for_value(&op.function);
            let dst = for_value(&op.return_assignment);
            let args = op
                .args
                .iter()
                .map(|v| for_value(v))
                .collect::<Vec<_>>()
                .join(" ");

            write!(text, "{dst} = call_parent {func} {args}")?;
        }
        Instruction::CallStatic(op) => {
            let object = for_value(&op.class);
            let function_name = for_value(&op.function);
            let return_assignment = for_value(&op.return_assignment);
            let args = op
                .args
                .iter()
                .map(|v| for_value(v))
                .collect::<Vec<_>>()
                .join(" ");

            write!(
                text,
                "{return_assignment} = call {object}.{function_name} {args}"
            )?;
        }
        Instruction::Cast(op) => {
            let src = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{} = cast {}", dst, src)?;
        }
        Instruction::Is(op) => {
            let object = for_value(&op.object);
            let class = for_value(&op.class);
            let dst = for_value(&op.return_assignment);
            write!(text, "{dst} = is {object} {class}")?;
        }
        Instruction::ArrayLength(op) => {
            let src = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = array_length {src}")?;
        }
        Instruction::ArrayFind(op) | Instruction::ArrayRFind(op) => {
            let array_value = for_value(&op.array_value);
            let return_index = for_value(&op.return_index);
            let start_index = for_value(&op.start_index);
            let value = for_value(&op.value);

            if matches!(instruction, Instruction::ArrayRFind(_)) {
                write!(
                    text,
                    "{return_index} = array_rfind {array_value}[{start_index}..] {value}"
                )?;
            } else {
                write!(
                    text,
                    "{return_index} = array_find {array_value}[{start_index}..] {value}"
                )?;
            }
        }
        Instruction::ArrayRemove(op) => {
            let array_value = for_value(&op.array_value);
            let index = for_value(&op.index);
            let count = for_value(&op.count);
            write!(text, "array_remove {array_value}[{index}..+{count}]")?;
        }
        Instruction::PropertyGet(op) => {
            let property = for_value(&op.property);
            let value = for_value(&op.value);
            let return_assignment = for_value(&op.return_assignment);
            write!(
                text,
                "{return_assignment} = property_get {property} {value}"
            )?;
        }
        Instruction::PropertySet(op) => {
            let property = for_value(&op.property);
            let object = for_value(&op.object);
            let value = for_value(&op.value);
            write!(text, "property_set {property} {object} {value}")?;
        }
        Instruction::StructGet(op) => {
            let struct_value = for_value(&op.struct_value);
            let member_name = for_value(&op.member_name);
            let dst = for_value(&op.return_assignment);
            write!(text, "{dst} = struct_get {struct_value} {member_name}")?;
        }
        Instruction::StructCreate(op) => {
            let dst = for_value(&op.return_assignment);
            write!(text, "{dst} = struct_create")?;
        }
        Instruction::StructSet(op) => {
            let struct_value = for_value(&op.struct_value);
            let member_name = for_value(&op.member_name);
            let value = for_value(&op.value);
            write!(text, "struct_set {struct_value} {member_name} {value}")?;
        }
        Instruction::ArrayGet(op) => {
            let array = for_value(&op.array);
            let index = for_value(&op.index);
            let dst = for_value(&op.return_assignment);
            write!(text, "{dst} = {array}[{index}]")?;
        }
        Instruction::ArraySet(op) => {
            let array = for_value(&op.array);
            let index = for_value(&op.index);
            let value = for_value(&op.value);
            write!(text, "array_set {array} {index} {value}")?;
        }
        Instruction::ArrayAdd(op) => {
            let array = for_value(&op.array);
            let value = for_value(&op.value);
            let count = for_value(&op.count);
            write!(text, "array_add {array} {value} {count}")?;
        }
        Instruction::ArrayInsert(op) => {
            let array = for_value(&op.array);
            let value = for_value(&op.value);
            let index = for_value(&op.index);
            write!(text, "array_insert {array} {value} {index}")?;
        }
        Instruction::ArrayPop(op) => {
            let array = for_value(&op.array);
            write!(text, "array_pop {array}")?;
        }
        Instruction::ArrayClear(op) => {
            let array = for_value(&op.array);
            write!(text, "array_clear {array}")?;
        }
        Instruction::Not(op) => {
            let src = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = not {src}")?;
        }
        Instruction::ArrayCreate(op) => {
            let len = for_value(&op.src);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = array_create {len}")?;
        }
        Instruction::ArrayFindStruct(op) | Instruction::ArrayRFindStruct(op) => {
            let array_value = for_value(&op.array_value);
            let return_index = for_value(&op.return_index);
            let start_index = for_value(&op.start_index);
            let value = for_value(&op.value);
            let member_name = for_value(&op.member_name);

            if matches!(instruction, Instruction::ArrayRFindStruct(_)) {
                write!(
                    text,
                    "{return_index} = array_rfind_struct {array_value} {member_name} {value} {start_index}"
                )?;
            } else {
                write!(
                    text,
                    "{return_index} = array_rfind_struct {array_value} {member_name} {value} {start_index}"
                )?;
            }
        }
        Instruction::StringConcat(op) => {
            let lhs = for_value(&op.lhs);
            let rhs = for_value(&op.rhs);
            let dst = for_value(&op.dst);
            write!(text, "{dst} = string_concat {lhs} {rhs}")?;
        }
    }

    Ok(())
}

fn for_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(val) => val.to_string(),
        Value::Int(val) => val.to_string(),
        Value::Float(val) => {
            format!("{val}f")
        }
        Value::String(val) => format!("\"{}\"", val.replace("\"", "\\\"")),
        Value::Ident(val) => val.to_owned(),
    }
}
