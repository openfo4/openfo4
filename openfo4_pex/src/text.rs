use std::fmt::Write;

use crate::{Function, Instruction, Value};

pub(crate) fn for_function(f: &Function) -> String {
    let mut text = String::new();

    for instruction in &f.instructions {
        match instruction {
            Instruction::Nop => {
                writeln!(text, "nop").unwrap();
            }
            Instruction::IAdd(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = iadd {lhs} {rhs}").unwrap();
            }
            Instruction::ISub(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = isub {lhs} {rhs}").unwrap();
            }
            Instruction::IMul(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = imul {lhs} {rhs}").unwrap();
            }
            Instruction::IDiv(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = idiv {lhs} {rhs}").unwrap();
            }
            Instruction::IMod(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = imod {lhs} {rhs}").unwrap();
            }
            Instruction::INeg(op) => {
                let src = for_value(&op.src);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = ineg {src}").unwrap();
            }
            Instruction::FAdd(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = fadd {lhs} {rhs}").unwrap();
            }
            Instruction::FSub(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = fsub {lhs} {rhs}").unwrap();
            }
            Instruction::FMul(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = fmul {lhs} {rhs}").unwrap();
            }
            Instruction::FDiv(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = fdiv {lhs} {rhs}").unwrap();
            }
            Instruction::FNeg(op) => {
                let src = for_value(&op.src);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = fneg {src}").unwrap();
            }
            Instruction::CmpEq(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = cmpeq {lhs} {rhs}").unwrap();
            }
            Instruction::CmpLt(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = cmplt {lhs} {rhs}").unwrap();
            }
            Instruction::CmpLe(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = cmple {lhs} {rhs}").unwrap();
            }
            Instruction::CmpGt(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = cmpgt {lhs} {rhs}").unwrap();
            }
            Instruction::CmpGe(op) => {
                let lhs = for_value(&op.lhs);
                let rhs = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = cmpge {lhs} {rhs}").unwrap();
            }
            Instruction::Store(op) => {
                let src = for_value(&op.src);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = {src}").unwrap();
            }
            Instruction::Return(op) => {
                let value = for_value(op);
                writeln!(text, "return {value}").unwrap();
            }
            Instruction::Branch(offset) => {
                let offset = for_value(offset);
                writeln!(text, "branch {offset}").unwrap();
            }
            Instruction::BranchIfTrue(op) => {
                let condition = for_value(&op.condition);
                let offset = for_value(&op.offset);
                writeln!(text, "branch_if_true {condition} {offset}").unwrap();
            }
            Instruction::BranchIfFalse(op) => {
                let condition = for_value(&op.condition);
                let offset = for_value(&op.offset);
                writeln!(text, "branch_if_false {condition} {offset}").unwrap();
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

                writeln!(
                    text,
                    "{return_assignment} = call {object}.{function_name} {args}"
                )
                .unwrap();
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

                writeln!(
                    text,
                    "{return_assignment} = call {object}.{function_name} {args}"
                )
                .unwrap();
            }
            Instruction::Cast(op) => {
                let src = for_value(&op.src);
                let dst = for_value(&op.dst);
                writeln!(text, "{} = cast {}", dst, src).unwrap();
            }
            Instruction::Is(op) => {
                let object = for_value(&op.object);
                let class = for_value(&op.class);
                let dst = for_value(&op.return_assignment);
                writeln!(text, "{dst} = is {object} {class}").unwrap();
            }
            Instruction::ArrayLength(op) => {
                let src = for_value(&op.src);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = array_length {src}").unwrap();
            }
            Instruction::ArrayFind(op) => {
                let array_value = for_value(&op.array_value);
                let return_index = for_value(&op.return_index);
                let start_index = for_value(&op.start_index);
                let value = for_value(&op.value);
                writeln!(
                    text,
                    "{return_index} = array_find {array_value}[{start_index}..] {value}"
                )
                .unwrap();
            }
            Instruction::ArrayRemove(op) => {
                let array_value = for_value(&op.array_value);
                let index = for_value(&op.index);
                let count = for_value(&op.count);
                writeln!(text, "array_remove {array_value}[{index}..+{count}]").unwrap();
            }
            Instruction::PropertyGet(op) => {
                let property = for_value(&op.property);
                let value = for_value(&op.value);
                let return_assignment = for_value(&op.return_assignment);
                writeln!(
                    text,
                    "{return_assignment} = property_get {property} {value}"
                )
                .unwrap();
            }
            Instruction::PropertySet(op) => {
                let property = for_value(&op.property);
                let object = for_value(&op.object);
                let value = for_value(&op.value);
                writeln!(text, "property_set {property} {object} {value}").unwrap();
            }
            Instruction::StructGet(op) => {
                let struct_value = for_value(&op.struct_value);
                let member_name = for_value(&op.member_name);
                let dst = for_value(&op.return_assignment);
                writeln!(text, "{dst} = struct_get {struct_value} {member_name}").unwrap();
            }
            Instruction::StructSet(op) => {
                let struct_value = for_value(&op.struct_value);
                let member_name = for_value(&op.member_name);
                let value = for_value(&op.value);
                writeln!(text, "struct_set {struct_value} {member_name} {value}").unwrap();
            }
            Instruction::ArrayGet(op) => {
                let array = for_value(&op.lhs);
                let index = for_value(&op.rhs);
                let dst = for_value(&op.dst);
                writeln!(text, "{dst} = {array}[{index}]").unwrap();
            }
            op => todo!("{:?}", op),
        }
    }

    text
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
