//! Expression parser

use crate::{Value, ValueExpression, VariableSet};

pub fn parse_expr(input: &str, variables: &VariableSet) -> ValueExpression {
    let tokens = tokenize(input);
    let mut tokens = &*tokens;

    parse_expr_tokens(&mut tokens, variables)
}

fn parse_expr_tokens(tokens: &mut &[Token<'_>], variables: &VariableSet) -> ValueExpression {
    let mut stacks = vec![ExpressionParseStack::new()];

    while let Some(token) = tokens.split_off_first() {
        let mut stack = stacks.last_mut().unwrap();

        match token {
            Token::OpenParen => {
                stacks.push(ExpressionParseStack::new());

                continue;
            }
            Token::CloseParen => {
                let expr = stacks.pop().unwrap().finalize();

                stack = stacks.last_mut().unwrap();
                stack.shift(ExprOrOp::Expr(expr));
            }
            Token::Plus => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::Add)));
            }
            Token::Minus => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::Sub)));
            }
            Token::Star => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::Mul)));
            }
            Token::Slash => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::Div)));
            }
            Token::Percent => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::Mod)));
            }
            Token::And if tokens.first() == Some(&Token::And) => {
                tokens.split_off_first().unwrap();
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::LogicalAnd)));
            }
            Token::And => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::BitAnd)));
            }
            Token::Or if tokens.first() == Some(&Token::Or) => {
                tokens.split_off_first().unwrap();
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::LogicalOr)));
            }
            Token::Or => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::BitOr)));
            }
            Token::Literal(lit) => {
                let value = parse_literal(*lit, variables);
                stack.shift(ExprOrOp::Expr(value));
            }
            Token::Eq if tokens.first() == Some(&Token::Eq) => {
                tokens.split_off_first().unwrap();
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::CmpEq)));
            }
            Token::Bang if tokens.first() == Some(&Token::Eq) => {
                tokens.split_off_first().unwrap();
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::CmpNe)));
            }
            Token::Lt if tokens.first() == Some(&Token::Eq) => {
                tokens.split_off_first().unwrap();
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::CmpLe)));
            }
            Token::Lt => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::CmpLt)));
            }
            Token::Gt if tokens.first() == Some(&Token::Eq) => {
                tokens.split_off_first().unwrap();
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::CmpGe)));
            }
            Token::Gt => {
                stack.shift(ExprOrOp::Op(Op::Binop(Binop::CmpGt)));
            }
            Token::Bang => {
                stack.shift(ExprOrOp::Op(Op::Unop(Unop::Not)));
            }
            Token::Eq => todo!(),
        }

        stack.reduce(&tokens);
    }

    assert_eq!(stacks.len(), 1);
    stacks.pop().unwrap().finalize()
}

#[derive(Clone, Debug)]
struct ExpressionParseStack {
    stack: Vec<ExprOrOp>,
}

impl ExpressionParseStack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn shift(&mut self, next: ExprOrOp) {
        self.stack.push(next);
    }

    fn reduce(&mut self, next: &[Token<'_>]) {
        let next = match next {
            [Token::Plus, ..] => Some(Op::Binop(Binop::Add)),
            [Token::Minus, ..] => Some(Op::Binop(Binop::Sub)),
            [Token::Star, ..] => Some(Op::Binop(Binop::Mul)),
            [Token::Slash, ..] => Some(Op::Binop(Binop::Div)),
            [Token::Percent, ..] => Some(Op::Binop(Binop::Mod)),
            [Token::Eq, Token::Eq, ..] => Some(Op::Binop(Binop::CmpEq)),
            [Token::Bang, Token::Eq, ..] => Some(Op::Binop(Binop::CmpNe)),
            [Token::Lt, Token::Eq, ..] => Some(Op::Binop(Binop::CmpLe)),
            [Token::Lt, ..] => Some(Op::Binop(Binop::CmpLt)),
            [Token::Gt, Token::Eq, ..] => Some(Op::Binop(Binop::CmpGe)),
            [Token::Gt, ..] => Some(Op::Binop(Binop::CmpGt)),
            [Token::And, Token::And, ..] => Some(Op::Binop(Binop::LogicalAnd)),
            [Token::And, ..] => Some(Op::Binop(Binop::BitAnd)),
            [Token::Or, Token::Or, ..] => Some(Op::Binop(Binop::LogicalOr)),
            [Token::Or, ..] => Some(Op::Binop(Binop::BitOr)),
            _ => None,
        };

        const PRECEDENCE: &[&[Op]] = &[
            &[Op::Unop(Unop::Not)],
            &[
                Op::Binop(Binop::Mul),
                Op::Binop(Binop::Div),
                Op::Binop(Binop::Mod),
            ],
            &[Op::Binop(Binop::Add), Op::Binop(Binop::Sub)],
            &[
                Op::Binop(Binop::CmpEq),
                Op::Binop(Binop::CmpNe),
                Op::Binop(Binop::CmpLt),
                Op::Binop(Binop::CmpLe),
                Op::Binop(Binop::CmpGt),
                Op::Binop(Binop::CmpGe),
            ],
            &[Op::Binop(Binop::BitAnd)],
            &[Op::Binop(Binop::BitOr)],
            &[Op::Binop(Binop::LogicalAnd)],
            &[Op::Binop(Binop::LogicalOr)],
        ];

        // If no next token exists we need to collapse the entire
        // stack.
        let Some(next) = next else {
            let ops: Vec<_> = PRECEDENCE.iter().copied().flatten().copied().collect();
            while self.ends_with_any_op(&ops) {
                self.reduce_top();
            }

            return;
        };

        for (index, tokens) in PRECEDENCE.iter().enumerate() {
            if !tokens.contains(&next) {
                continue;
            }

            let stronger_ops: Vec<_> = if index == 0 {
                PRECEDENCE[0].iter().copied().collect()
            } else {
                PRECEDENCE[..index]
                    .iter()
                    .copied()
                    .flatten()
                    .copied()
                    .collect()
            };

            while self.ends_with_any_op(&stronger_ops) {
                self.reduce_top();
            }
        }
    }

    fn reduce_top(&mut self) {
        let rhs = self.stack.pop().unwrap().into_expr();
        let op = self.stack.pop().unwrap();

        let expr = match op {
            ExprOrOp::Op(Op::Unop(op)) => match op {
                Unop::Not => ValueExpression::Not(Box::new(rhs)),
            },
            ExprOrOp::Op(Op::Binop(op)) => {
                let lhs = self.stack.pop().unwrap().into_expr();

                match op {
                    Binop::Add => ValueExpression::Add(Box::new(lhs), Box::new(rhs)),
                    Binop::Sub => ValueExpression::Sub(Box::new(lhs), Box::new(rhs)),
                    Binop::Mul => ValueExpression::Mul(Box::new(lhs), Box::new(rhs)),
                    Binop::Div => ValueExpression::Div(Box::new(lhs), Box::new(rhs)),
                    Binop::Mod => ValueExpression::Mod(Box::new(lhs), Box::new(rhs)),
                    Binop::CmpEq => ValueExpression::CmpEq(Box::new(lhs), Box::new(rhs)),
                    Binop::CmpNe => ValueExpression::CmpNe(Box::new(lhs), Box::new(rhs)),
                    Binop::CmpLe => ValueExpression::CmpLe(Box::new(lhs), Box::new(rhs)),
                    Binop::CmpLt => ValueExpression::CmpLt(Box::new(lhs), Box::new(rhs)),
                    Binop::CmpGt => ValueExpression::CmpGt(Box::new(lhs), Box::new(rhs)),
                    Binop::CmpGe => ValueExpression::CmpGe(Box::new(lhs), Box::new(rhs)),
                    Binop::BitAnd => ValueExpression::And(Box::new(lhs), Box::new(rhs)),
                    Binop::LogicalAnd => ValueExpression::And(Box::new(lhs), Box::new(rhs)),
                    Binop::BitOr => ValueExpression::Or(Box::new(lhs), Box::new(rhs)),
                    Binop::LogicalOr => ValueExpression::Or(Box::new(lhs), Box::new(rhs)),
                }
            }
            ExprOrOp::Expr(_) => todo!(),
        };

        self.stack.push(ExprOrOp::Expr(expr));
    }

    fn ends_with_any_op(&mut self, ops: &[Op]) -> bool {
        for op in ops {
            let op_len = match op {
                Op::Unop(_) => 2,
                Op::Binop(_) => 3,
            };

            if self.stack.len() < op_len {
                continue;
            }

            let len = self.stack.len();
            let tail = &self.stack[len - op_len..];

            let is_valid_op = match op {
                Op::Unop(_) => tail[0] == ExprOrOp::Op(*op) && matches!(tail[1], ExprOrOp::Expr(_)),
                Op::Binop(_) => {
                    matches!(tail[0], ExprOrOp::Expr(_))
                        && tail[1] == ExprOrOp::Op(*op)
                        && matches!(tail[2], ExprOrOp::Expr(_))
                }
            };

            if is_valid_op {
                return true;
            }
        }

        false
    }

    fn finalize(mut self) -> ValueExpression {
        assert_eq!(self.stack.len(), 1);
        match self.stack.pop().unwrap() {
            ExprOrOp::Expr(expr) => expr,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ExprOrOp {
    Expr(ValueExpression),
    Op(Op),
}

impl ExprOrOp {
    fn into_expr(self) -> ValueExpression {
        match self {
            Self::Expr(e) => e,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Op {
    Unop(Unop),
    Binop(Binop),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Binop {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    LogicalAnd,
    LogicalOr,
    CmpEq,
    CmpNe,
    CmpLt,
    CmpLe,
    CmpGt,
    CmpGe,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Unop {
    Not,
}

fn parse_literal(lit: Literal<'_>, variables: &VariableSet) -> ValueExpression {
    match lit {
        Literal::Str(ident) => {
            let variable = variables.lookup_variable_name(ident).unwrap();
            ValueExpression::Variable(variable)
        }
        Literal::Int { base, value } => ValueExpression::Const(Value::I32(value)),
        Literal::Float { value } => ValueExpression::Const(Value::F32(value)),
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Token<'a> {
    Literal(Literal<'a>),
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `&`
    And,
    /// `|`
    Or,
    /// `=`
    Eq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `!`
    Bang,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Literal<'a> {
    Str(&'a str),
    Int { base: u8, value: i32 },
    Float { value: f32 },
}

fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();

    let mut literal_start = 0;

    for (index, char) in input.char_indices() {
        let mut close_literal = || {
            let lit = &input[literal_start..index].trim();

            if !lit.is_empty() {
                match lit.parse::<i32>() {
                    Ok(mut value) => {
                        while tokens.last() == Some(&Token::Minus) {
                            tokens.pop().unwrap();
                            value = -value;
                        }

                        tokens.push(Token::Literal(Literal::Int { base: 10, value }));
                    }
                    Err(_) => match lit.parse::<f32>() {
                        Ok(mut value) => {
                            while tokens.last() == Some(&Token::Minus) {
                                tokens.pop().unwrap();
                                value = -value;
                            }

                            tokens.push(Token::Literal(Literal::Float { value }));
                        }
                        Err(_) => {
                            tokens.push(Token::Literal(Literal::Str(&lit)));
                        }
                    },
                }
            }

            literal_start = index + 1;
        };

        let token = match char {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '&' => Token::And,
            '|' => Token::Or,
            '=' => Token::Eq,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '!' => Token::Bang,
            ' ' => {
                close_literal();
                continue;
            }
            _ => {
                continue;
            }
        };

        close_literal();
        tokens.push(token);
    }

    let lit = &input[literal_start..input.len()].trim();
    if !lit.is_empty() {
        match lit.parse::<i32>() {
            Ok(value) => {
                tokens.push(Token::Literal(Literal::Int { base: 10, value }));
            }
            Err(_) => match lit.parse::<f32>() {
                Ok(value) => {
                    tokens.push(Token::Literal(Literal::Float { value }));
                }
                Err(_) => {
                    tokens.push(Token::Literal(Literal::Str(lit)));
                }
            },
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use crate::expr::parse_expr;
    use crate::{Value, ValueExpression, VariableSet};

    use super::{Literal, Token, tokenize};

    #[test]
    fn tokenize_basic() {
        let input = "a&b<> ( c)  -2   2.3";
        assert_eq!(
            tokenize(input),
            vec![
                Token::Literal(Literal::Str("a")),
                Token::And,
                Token::Literal(Literal::Str("b")),
                Token::Lt,
                Token::Gt,
                Token::OpenParen,
                Token::Literal(Literal::Str("c")),
                Token::CloseParen,
                Token::Literal(Literal::Int {
                    base: 10,
                    value: -2
                }),
                Token::Literal(Literal::Float { value: 2.3 }),
            ]
        );
    }

    #[test]
    fn parse_expr_basic() {
        let input = "1 + 2 * 3 == 4 && 5 || 6";
        let variables = VariableSet::default();

        let expr = parse_expr(input, &variables);

        assert_eq!(
            expr,
            ValueExpression::Or(
                Box::new(ValueExpression::And(
                    Box::new(ValueExpression::CmpEq(
                        Box::new(ValueExpression::Add(
                            Box::new(ValueExpression::Const(Value::I32(1))),
                            Box::new(ValueExpression::Mul(
                                Box::new(ValueExpression::Const(Value::I32(2))),
                                Box::new(ValueExpression::Const(Value::I32(3))),
                            ))
                        )),
                        Box::new(ValueExpression::Const(Value::I32(4))),
                    ),),
                    Box::new(ValueExpression::Const(Value::I32(5))),
                )),
                Box::new(ValueExpression::Const(Value::I32(6))),
            )
        );
    }

    #[test]
    fn parse_expr_parens() {
        let input = "(((1 + 2) * 3))";
        let variables = VariableSet::default();

        let expr = parse_expr(input, &variables);

        assert_eq!(
            expr,
            ValueExpression::Mul(
                Box::new(ValueExpression::Add(
                    Box::new(ValueExpression::Const(Value::I32(1))),
                    Box::new(ValueExpression::Const(Value::I32(2))),
                )),
                Box::new(ValueExpression::Const(Value::I32(3)))
            )
        );
    }

    #[test]
    fn parse_expr_unop() {
        let input = "!1 * 2";
        let variables = VariableSet::default();

        let expr = parse_expr(input, &variables);

        assert_eq!(
            expr,
            ValueExpression::Mul(
                Box::new(ValueExpression::Not(Box::new(ValueExpression::Const(
                    Value::I32(1)
                )))),
                Box::new(ValueExpression::Const(Value::I32(2))),
            )
        );
    }
}
