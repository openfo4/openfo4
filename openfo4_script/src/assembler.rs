//! Pex assembler for testing
//!
//! TODO: Move this into a separate crate, we only need this as a test dependency.
//!
//! ## Example
//!
//! ```text
//! class MyClass:
//!
//! function MyFunction(param0: int, param1: int) -> float static native:
//! local my_int: int
//! local my_float: float
//! my_local = iadd param0 param0
//! my_float = cast my_int
//! return my_float
//!
//! ```

use std::str::Split;

use openfo4_pex::{
    ArrayFind, Binop, CallStatic, Class, Function, Instruction, Module, State, Unop, Value,
    VariableDefinition,
};

pub fn assemble(input: &str) -> Module {
    let mut classes = Vec::new();
    let class = parse_class(input);
    classes.push(class);

    Module { classes }
}

fn parse_class(mut input: &str) -> Class {
    input = input.trim().strip_prefix("class").unwrap();

    let end = input.find(":").unwrap();
    let name = input[..end].trim().to_owned();
    let end_line = input.find("\n").unwrap();
    input = &input[end_line + 1..];

    let mut functions = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("function") {
            let function = parse_function(input);
            functions.push(function);
        } else {
        }
    }

    Class {
        name,
        parent: "".to_owned(),
        structs: Vec::new(),
        states: vec![State {
            name: "".to_owned(),
            functions,
        }],
        default_state: "".to_owned(),
        variables: Vec::new(),
        properties: Vec::new(),
    }
}

fn parse_function(mut input: &str) -> Function {
    input = input.trim().strip_prefix("function").unwrap();

    let open_paren = input.find("(").unwrap();
    let name = input[..open_paren].trim().to_owned();

    let mut params = Vec::new();

    let close_paren = input.find(")").unwrap();
    for pair in input[open_paren + 1..close_paren].split(',') {
        if pair.trim().is_empty() {
            continue;
        }

        let (name, ty) = pair.split_once(':').unwrap();

        params.push(VariableDefinition {
            name: name.trim().to_owned(),
            ty: ty.trim().to_owned(),
        });
    }

    input = &input[close_paren + 1..].trim();

    dbg!(&input);
    let mut return_ty = "none".to_owned();
    if input.trim().starts_with("->") {
        input = input.trim().strip_prefix("->").unwrap();
        let (mut ty, rem) = input.trim().split_once(' ').unwrap_or((input, ""));
        if ty.trim().ends_with(":") {
            ty = ty.trim().strip_suffix(":").unwrap();
        }

        input = rem;
        return_ty = ty.to_owned();
    }

    let mut is_static = false;
    if input.trim().starts_with("static") {
        input = input.strip_prefix("static").unwrap().trim();
        is_static = true;
    }

    let mut is_native = false;
    if input.trim().starts_with("native") {
        input = input.strip_prefix("native").unwrap().trim();
        is_native = true;
    }

    let end_line = input.find("\n").unwrap_or(input.len());
    input = input.get(end_line + 1..).unwrap_or("");

    let mut locals = Vec::new();
    let mut instructions = Vec::new();
    for mut line in input.lines() {
        line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("local") {
            line = line.strip_prefix("local").unwrap().trim();

            let (name, ty) = line.split_once(":").unwrap();

            locals.push(VariableDefinition {
                name: name.trim().to_owned(),
                ty: ty.trim().to_owned(),
            });

            continue;
        }

        instructions.push(parse_instruction(line));
    }

    Function {
        name,
        is_static,
        is_native,
        params,
        return_ty,
        locals,
        instructions,
    }
}

fn parse_instruction(input: &str) -> Instruction {
    let mut words = input.split(' ');

    let parse_unop = |mut words: Split<'_, char>, dst| {
        let src = parse_value(words.next().unwrap());
        Unop { src, dst }
    };

    let parse_binop = |mut words: Split<'_, char>, dst| {
        let lhs = parse_value(words.next().unwrap());
        let rhs = parse_value(words.next().unwrap());
        Binop { lhs, rhs, dst }
    };

    match words.next().unwrap() {
        "nop" => Instruction::Nop,
        "return" => {
            let value = parse_value(words.next().unwrap());
            Instruction::Return(value)
        }
        dst => {
            let dst = parse_value(dst);

            assert_eq!(words.next().unwrap(), "=");

            match words.next().unwrap() {
                "iadd" => Instruction::IAdd(parse_binop(words, dst)),
                "isub" => Instruction::ISub(parse_binop(words, dst)),
                "imul" => Instruction::ISub(parse_binop(words, dst)),
                "idiv" => Instruction::ISub(parse_binop(words, dst)),
                "imod" => Instruction::ISub(parse_binop(words, dst)),
                "fadd" => Instruction::ISub(parse_binop(words, dst)),
                "fsub" => Instruction::ISub(parse_binop(words, dst)),
                "fmul" => Instruction::ISub(parse_binop(words, dst)),
                "fdiv" => Instruction::ISub(parse_binop(words, dst)),
                "cast" => Instruction::Cast(parse_unop(words, dst)),
                "array_create" => Instruction::ArrayCreate(parse_unop(words, dst)),
                "array_find" => {
                    let array_value = parse_value(words.next().unwrap());
                    let value = parse_value(words.next().unwrap());
                    let start_index = parse_value(words.next().unwrap());

                    Instruction::ArrayFind(ArrayFind {
                        array_value,
                        value,
                        start_index,
                        return_index: dst,
                    })
                }
                "call_static" => {
                    let class = parse_value(words.next().unwrap());
                    let function = parse_value(words.next().unwrap());
                    let args = words.map(|v| parse_value(v)).collect();
                    Instruction::CallStatic(CallStatic {
                        class,
                        function,
                        args,
                        return_assignment: dst,
                    })
                }
                op => panic!("unknown op: {}", op),
            }
        }
    }
}

fn parse_value(input: &str) -> Value {
    if input.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if input.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Some(val) = input.strip_suffix("f")
        && let Ok(val) = val.parse::<f32>()
    {
        Value::Float(val)
    } else if let Ok(val) = input.parse::<i32>() {
        Value::Int(val)
    } else if input.contains("\"") {
        todo!()
    } else {
        Value::Ident(input.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use openfo4_pex::{Binop, Instruction, Unop, Value};

    use crate::assembler::assemble;

    #[test]
    fn test_basic() {
        let input = r#"
        class MyClass:

        function MyFunction(param0: int, param1: bool) -> float:
        local my_int: int
        local my_float: float
        my_local = iadd param0 1
        my_float = cast my_int
        return my_float
        "#;

        let res = assemble(input);
        let class = &res.classes[0];
        let func = &class.states[0].functions[0];

        assert_eq!(class.name, "MyClass");
        assert_eq!(func.name, "MyFunction");
        assert_eq!(func.params[0].name, "param0");
        assert_eq!(func.params[0].ty, "int");
        assert_eq!(func.params[1].name, "param1");
        assert_eq!(func.params[1].ty, "bool");
        assert_eq!(func.return_ty, "float");

        assert_eq!(
            func.instructions,
            [
                Instruction::IAdd(Binop {
                    lhs: Value::Ident("param0".to_owned()),
                    rhs: Value::Int(1),
                    dst: Value::Ident("my_local".to_owned())
                }),
                Instruction::Cast(Unop {
                    src: Value::Ident("my_int".to_owned()),
                    dst: Value::Ident("my_float".to_owned())
                }),
                Instruction::Return(Value::Ident("my_float".to_owned())),
            ]
        );
    }
}
