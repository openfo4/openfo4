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

use std::fmt::Debug;

use openfo4_pex::{
    ArrayFind, Binop, CallStatic, Class, Function, Instruction, Module, State, Unop, Value,
    VariableDefinition,
};

pub fn assemble(input: &str) -> Module {
    let tokens = tokenize(input);
    let mut cursor = Cursor { tokens: &tokens };

    let mut classes = Vec::new();

    loop {
        cursor.trim();
        match cursor.peek() {
            Some(Token::Ident("class")) => {
                let class = parse_class(&mut cursor);
                classes.push(class);
            }
            None => break,
            _ => todo!(),
        }
    }

    Module { classes }
}

fn parse_class(cursor: &mut Cursor<'_>) -> Class {
    cursor.trim();

    cursor.consume(Token::Ident("class"));
    let name = cursor.consume_ident().to_owned();
    cursor.consume(Token::Colon);
    cursor.consume(Token::Eol);

    let mut functions = Vec::new();

    loop {
        match cursor.peek() {
            Some(Token::Ident("function")) => {
                let function = parse_function(cursor);
                functions.push(function);
            }
            Some(Token::Ident("class")) | None => break,
            _ => todo!(),
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

fn parse_function(cursor: &mut Cursor<'_>) -> Function {
    cursor.trim();
    cursor.consume(Token::Ident("function"));
    let name = cursor.consume_ident().to_owned();
    cursor.consume(Token::OpenParen);

    let mut params = Vec::new();
    while cursor.peek() != Some(&Token::CloseParen) {
        let name = cursor.consume_ident().to_owned();
        cursor.consume(Token::Colon);
        let ty = cursor.consume_ident().to_owned();
        params.push(VariableDefinition { name, ty });

        if cursor.peek() == Some(&Token::Comma) {
            cursor.pop();
        }
    }
    cursor.pop();

    let return_ty = if cursor.peek() == Some(&Token::Arrow) {
        cursor.pop();
        cursor.consume_ident().to_owned()
    } else {
        "none".to_owned()
    };

    let mut is_static = false;
    let mut is_native = false;
    while cursor.peek() != Some(&Token::Colon) {
        match cursor.pop() {
            Some(Token::Ident("static")) => is_static = true,
            Some(Token::Ident("native")) => is_native = true,
            _ => todo!(),
        }
    }
    cursor.consume(Token::Colon);
    if cursor.peek() == Some(&Token::Ident("static")) {
        cursor.pop();
    }

    cursor.consume(Token::Eol);

    let mut locals = Vec::new();
    let mut instructions = Vec::new();

    loop {
        match cursor.peek() {
            Some(Token::Ident("class")) | Some(Token::Ident("function")) | None => break,
            Some(Token::Ident("local")) => {
                cursor.pop();
                let name = cursor.consume_ident().to_owned();
                cursor.consume(Token::Colon);
                let ty = cursor.consume_ident().to_owned();
                locals.push(VariableDefinition { name, ty });
            }
            _ => {
                instructions.push(parse_instruction(cursor));
            }
        }

        cursor.consume(Token::Eol);
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

fn parse_instruction(cursor: &mut Cursor<'_>) -> Instruction {
    let parse_unop = |cursor: &mut Cursor<'_>, dst| {
        let src = parse_value(cursor);
        Unop { src, dst }
    };

    let parse_binop = |cursor: &mut Cursor<'_>, dst| {
        let lhs = parse_value(cursor);
        let rhs = parse_value(cursor);
        Binop { lhs, rhs, dst }
    };

    let dst = cursor.consume_ident().to_owned();
    if cursor.peek() == Some(&Token::Eq) {
        cursor.pop();
        let opcode = cursor.consume_ident();
        let dst = Value::Ident(dst);

        match opcode {
            "iadd" => Instruction::IAdd(parse_binop(cursor, dst)),
            "isub" => Instruction::ISub(parse_binop(cursor, dst)),
            "imul" => Instruction::IMul(parse_binop(cursor, dst)),
            "idiv" => Instruction::IDiv(parse_binop(cursor, dst)),
            "imod" => Instruction::IMod(parse_binop(cursor, dst)),
            "fadd" => Instruction::FAdd(parse_binop(cursor, dst)),
            "fsub" => Instruction::FSub(parse_binop(cursor, dst)),
            "fmul" => Instruction::FMul(parse_binop(cursor, dst)),
            "fdiv" => Instruction::FDiv(parse_binop(cursor, dst)),
            "cast" => Instruction::Cast(parse_unop(cursor, dst)),
            "call_static" => {
                let class = parse_value(cursor);
                let function = parse_value(cursor);
                let mut args = Vec::new();
                while cursor.peek() != Some(&Token::Eol) {
                    args.push(parse_value(cursor));
                }

                Instruction::CallStatic(CallStatic {
                    class,
                    function,
                    return_assignment: dst,
                    args,
                })
            }
            "array_create" => Instruction::ArrayCreate(parse_unop(cursor, dst)),
            "array_find" => {
                let array_value = parse_value(cursor);
                let value = parse_value(cursor);
                let start_index = parse_value(cursor);
                Instruction::ArrayFind(ArrayFind {
                    array_value,
                    return_index: dst,
                    value,
                    start_index,
                })
            }
            v => todo!("{:?}", v),
        }
    } else {
        let opcode = dst;
        match &*opcode {
            "nop" => Instruction::Nop,
            "return" => {
                let args = parse_value(cursor);
                Instruction::Return(args)
            }
            _ => todo!(),
        }
    }
}

fn parse_value(cursor: &mut Cursor<'_>) -> Value {
    match cursor.pop() {
        Some(Token::StrLit(s)) => Value::String(s.to_string()),
        Some(Token::Ident(s)) => Value::Ident(s.to_string()),
        Some(Token::BoolLit(v)) => Value::Bool(*v),
        Some(Token::IntLit(v)) => Value::Int(*v),
        Some(Token::FloatLit(v)) => Value::Float(*v),
        _ => todo!(),
    }
}

#[derive(Clone, Debug)]
struct Cursor<'a> {
    tokens: &'a [Token<'a>],
}

impl<'a> Cursor<'a> {
    fn consume(&mut self, token: Token<'_>) {
        if self.tokens[0] == token {
            self.tokens = &self.tokens[1..];
        } else {
            todo!("nono");
        }
    }

    fn consume_ident(&mut self) -> &str {
        let v = match self.tokens[0] {
            Token::Ident(v) => v,
            _ => todo!(),
        };

        self.tokens = &self.tokens[1..];
        v
    }

    fn peek(&self) -> Option<&Token<'_>> {
        self.tokens.first()
    }

    fn pop(&mut self) -> Option<&Token<'_>> {
        self.tokens.split_off_first()
    }

    fn trim(&mut self) {
        while self.tokens.first() == Some(&Token::Eol) {
            self.tokens = &self.tokens[1..];
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token<'a> {
    Ident(&'a str),
    BoolLit(bool),
    IntLit(i32),
    FloatLit(f32),
    StrLit(&'a str),
    /// `:`
    Colon,
    /// `->`
    Arrow,
    OpenParen,
    CloseParen,
    Comma,
    Eq,
    Eol,
}

fn tokenize(mut input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();

    loop {
        // Newlines have semantics, unlike other whitespace chars.
        if !input.starts_with("\n") {
            input = input.trim_start();
        }

        if input.is_empty() {
            break;
        }

        if input.starts_with('"') {
            input = &input[1..];

            let mut end = 0;
            loop {
                end = end + 1 + input[end + 1..].find('"').unwrap();
                if end == 0 || input.as_bytes()[end - 1] != b'\\' {
                    break;
                }
            }

            tokens.push(Token::StrLit(&input[..end]));
            input = &input[end + 1..];
            continue;
        }

        let mut parse_token = |token: &str| -> bool {
            if let Some(s) = input.strip_prefix(token) {
                input = s;
                true
            } else {
                false
            }
        };

        fn find_any(s: &str, patterns: &[&str]) -> Option<usize> {
            let mut min = None;
            for p in patterns {
                let i = s.find(p);
                if min.is_none() {
                    min = i;
                }

                if let Some(i) = i
                    && let Some(m) = min
                {
                    min = Some(m.min(i));
                }
            }

            min
        }

        if parse_token(":") {
            tokens.push(Token::Colon);
        } else if parse_token("->") {
            tokens.push(Token::Arrow);
        } else if parse_token("(") {
            tokens.push(Token::OpenParen);
        } else if parse_token(")") {
            tokens.push(Token::CloseParen);
        } else if parse_token(",") {
            tokens.push(Token::Comma);
        } else if parse_token("=") {
            tokens.push(Token::Eq);
        } else if parse_token("\n") {
            if tokens.last() != Some(&Token::Eol) {
                tokens.push(Token::Eol);
            }
        } else {
            let end =
                find_any(input, &[" ", ":", "->", "(", ")", ",", "=", "\n"]).unwrap_or(input.len());
            let ident = &input[..end].trim();

            if let Some(s) = ident.strip_suffix("f")
                && let Ok(v) = s.parse::<f32>()
            {
                tokens.push(Token::FloatLit(v));
            } else if let Ok(v) = ident.parse::<i32>() {
                tokens.push(Token::IntLit(v));
            } else if let Ok(v) = ident.parse::<bool>() {
                tokens.push(Token::BoolLit(v));
            } else {
                tokens.push(Token::Ident(ident));
            }

            if end == input.len() {
                input = "";
            } else if input.as_bytes()[end] == b' ' {
                input = &input[end + 1..];
            } else {
                input = &input[end..];
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use openfo4_pex::{Binop, Instruction, Unop, Value};

    use crate::assembler::{Token, assemble, tokenize};

    #[test]
    fn test_tokenize() {
        assert_eq!(
            tokenize("hello -> world, : \"str lit\" id"),
            vec![
                Token::Ident("hello"),
                Token::Arrow,
                Token::Ident("world"),
                Token::Comma,
                Token::Colon,
                Token::StrLit("str lit"),
                Token::Ident("id"),
            ]
        );
    }

    #[test]
    fn tokenize_str_lit() {
        assert_eq!(
            tokenize("\"Hello World\""),
            vec![Token::StrLit("Hello World")]
        );
    }

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

    #[test]
    fn test_multi_class() {
        let input = r#"
        class A:
        class B:
        class C:
        "#;

        let res = assemble(input);
        assert_eq!(res.classes[0].name, "A");
        assert_eq!(res.classes[1].name, "B");
        assert_eq!(res.classes[2].name, "C");
    }
}
