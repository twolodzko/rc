use crate::{
    Algebra, ArityError, Template,
    expr::{Expr, Function, Method, Op},
    number::Number,
};
use anyhow::{Result, bail};
use core::f64;
use num::{bigint::BigInt, complex::Complex};
use pest::{
    Parser,
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
};
use std::str::FromStr;
use std::sync::LazyLock;

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct CalculatorParser;

static PRATT_PARSER: LazyLock<PrattParser<Rule>> = LazyLock::new(|| {
    use Rule::*;
    use pest::pratt_parser::{Assoc::*, Op};
    PrattParser::new()
        .op(Op::infix(logical_or, Left)) // or
        .op(Op::infix(logical_and, Left)) // and
        .op(Op::infix(same_type, Left)) // ?=
        .op(Op::infix(eq, Left) | Op::infix(ne, Left)) // = !=
        .op(Op::infix(lt, Left) | Op::infix(le, Left) | Op::infix(gt, Left) | Op::infix(ge, Left)) // < <= > >=
        .op(Op::infix(is_in, Left)) // in
        .op(Op::infix(get, Left)) // :
        .op(Op::infix(add, Left) | Op::infix(sub, Left)) // + -
        .op(Op::infix(mul, Left)
            | Op::infix(idiv, Left)
            | Op::infix(div, Left)
            | Op::infix(rem, Left)
            | Op::infix(at, Left)) // * // / % @
        .op(Op::infix(pow, Right)) // ^
        .op(Op::infix(bit_or, Left)) // |
        .op(Op::infix(bit_and, Left)) // &
        .op(Op::infix(interval, Left)) // ~
        .op(Op::prefix(neg)) // -x
        .op(Op::postfix(fact)) // x!
});

pub fn parse(input: &str) -> Result<Vec<Expr>> {
    let mut pairs = CalculatorParser::parse(Rule::program, input)?;
    let program_pair = pairs.next().unwrap();

    let mut exprs = Vec::new();
    for pair in program_pair.into_inner() {
        let expr = match pair.as_rule() {
            Rule::load => {
                let mut inner = pair.into_inner();
                let path = shellexpand::tilde(inner.next().unwrap().as_str().trim());
                Expr::Load(path.as_ref().into())
            }
            Rule::function => parse_fun(pair)?,
            Rule::expression => parse_expr(pair.into_inner())?,
            _ => continue,
        };
        exprs.push(expr);
    }
    Ok(exprs)
}

fn parse_fun(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let name = parse_var(inner.next().unwrap())?;

    let mut args = Vec::new();
    while let Some(pair) = inner.peek()
        && let Ok(arg) = parse_var(pair)
    {
        inner.next();
        args.push(arg);
    }

    let inner = inner.next().unwrap().into_inner();
    let expr = parse_expr(inner)?;
    let body = if let Expr::Block(block) = expr {
        block
    } else {
        vec![expr]
    };
    Ok(Expr::Function(Function { name, args, body }))
}

fn parse_var(pair: Pair<Rule>) -> Result<String> {
    let Rule::name = pair.as_rule() else {
        bail!("unexpected {}", pair)
    };
    Ok(pair.to_string())
}

fn parse_expr(pairs: Pairs<Rule>) -> Result<Expr> {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::apply => {
                let mut inner = primary.into_inner();
                let name = parse_var(inner.next().unwrap())?;
                let mut args = Vec::new();
                for pair in inner {
                    let expr = parse_expr(pair.into_inner())?;
                    args.push(expr);
                }
                let method = match name.as_str() {
                    "abs" => Method::Abs,
                    "acos" => Method::Acos,
                    "acosh" => Method::Acosh,
                    "asin" => Method::Asin,
                    "asinh" => Method::Asinh,
                    "atan" => Method::Atan,
                    "atanh" => Method::Atanh,
                    "cbrt" => Method::Cbrt,
                    "ceil" => Method::Ceil,
                    "cos" => Method::Cos,
                    "cosh" => Method::Cosh,
                    "erf" => Method::Erf,
                    "erfc" => Method::Erfc,
                    "exp" => Method::Exp,
                    "floor" => Method::Floor,
                    "factorial" => Method::Fact,
                    "gamma" => Method::Gamma,
                    "lgamma" => Method::Lgamma,
                    "ln" | "log" => Method::Ln,
                    "log10" => Method::Log10,
                    "log2" => Method::Log2,
                    "round" => Method::Round,
                    "sin" => Method::Sin,
                    "sinh" => Method::Sinh,
                    "sqrt" => Method::Sqrt,
                    "tan" => Method::Tan,
                    "tanh" => Method::Tanh,
                    _ => return Ok(Expr::Apply(name, args)),
                };
                if args.len() != 1 {
                    bail!(ArityError {
                        name,
                        arity: 1,
                        count: args.len()
                    });
                }
                Ok(Expr::Primitive(method, Box::new(args[0].clone())))
            }
            Rule::block => {
                let mut acc = Vec::new();
                let inner = primary.into_inner();
                for pair in inner {
                    let expr = parse_expr(pair.into_inner())?;
                    acc.push(expr);
                }
                if acc.len() == 1 {
                    Ok(acc.pop().unwrap())
                } else {
                    Ok(Expr::Block(acc))
                }
            }
            Rule::abs => {
                let inner = primary.into_inner();
                let expr = parse_expr(inner)?;
                Ok(Expr::Primitive(Method::Abs, Box::new(expr)))
            }
            Rule::expression => parse_expr(primary.into_inner()),
            Rule::term => parse_expr(primary.into_inner()),
            Rule::name => {
                let expr = match primary.as_str().to_lowercase().as_str() {
                    "pi" => {
                        Expr::Value(Algebra::Number(Number::Float(std::f64::consts::PI.into())))
                    }
                    "e" => Expr::Value(Algebra::Number(Number::Float(std::f64::consts::E.into()))),
                    "i" => Expr::Value(Algebra::Number(Number::Complex(num::complex::Complex::I))),
                    "epsilon" => Expr::Value(Algebra::Number(Number::Float(f64::EPSILON.into()))),
                    "nan" => Expr::Value(Algebra::Number(Number::NAN)),
                    "inf" => Expr::Value(Algebra::Number(Number::INFINITY)),
                    _ => Expr::Variable(primary.to_string()),
                };
                Ok(expr)
            }
            Rule::float => {
                let s = primary.as_str();
                let number = if let Ok(value) = BigInt::from_str(s) {
                    Expr::Value(Algebra::Number(Number::Integer(value)))
                } else if let Ok(value) = f64::from_str(s) {
                    Expr::Value(Algebra::Number(Number::Float(value.into())))
                } else {
                    Expr::Value(Algebra::Number(Number::NAN))
                };
                Ok(number)
            }
            Rule::complex => {
                let mut inner = primary.into_inner();
                let first = inner.next().unwrap();
                let mut real = 0f64;
                let imag = if first.as_rule() == Rule::imaginary {
                    parse_imag(first)?
                } else {
                    real = f64::from_str(first.as_str())?;
                    parse_imag(inner.next().unwrap())?
                };
                Ok(Expr::Value(Algebra::Number(Number::Complex(Complex::new(
                    real, imag,
                )))))
            }
            Rule::vector => {
                let inner = primary.into_inner();
                let mut acc = Vec::new();
                for pair in inner {
                    let expr = parse_expr(pair.into_inner())?;
                    acc.push(expr);
                }
                Ok(Expr::NewVec(acc))
            }
            Rule::ifelse => {
                let mut inner = primary.into_inner();
                let cond = parse_expr(inner.next().unwrap().into_inner())?;
                let yes = parse_expr(inner.next().unwrap().into_inner())?;
                let no = parse_expr(inner.next().unwrap().into_inner())?;
                Ok(Expr::IfElse(Box::new(cond), Box::new(yes), Box::new(no)))
            }
            Rule::print_fn => {
                let inner = primary.into_inner().next().unwrap().into_inner();
                let acc = parse_template(inner)?;
                Ok(Expr::Print(acc))
            }
            Rule::error_fn => {
                let inner = primary.into_inner().next().unwrap().into_inner();
                let acc = parse_template(inner)?;
                Ok(Expr::Error(acc))
            }
            rule => bail!("unexpected {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            let op = match op.as_rule() {
                Rule::add => Op::Add,
                Rule::at => Op::Dot,
                Rule::bit_and => Op::BitAnd,
                Rule::bit_or => Op::BitOr,
                Rule::div => Op::Div,
                Rule::eq => Op::Eq,
                Rule::ge => Op::Ge,
                Rule::get => Op::Get,
                Rule::gt => Op::Gt,
                Rule::idiv => Op::Idiv,
                Rule::interval => Op::Interval,
                Rule::is_in => Op::In,
                Rule::le => Op::Le,
                Rule::logical_and => Op::And,
                Rule::logical_or => Op::Or,
                Rule::lt => Op::Lt,
                Rule::mul => Op::Mul,
                Rule::ne => Op::Ne,
                Rule::pow => Op::Pow,
                Rule::rem => Op::Rem,
                Rule::same_type => Op::EqType,
                Rule::sub => Op::Sub,
                rule => bail!("unexpected operator {:?}", rule),
            };
            Ok(Expr::BinaryOp {
                lhs: Box::new(lhs?),
                op,
                rhs: Box::new(rhs?),
            })
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::neg => {
                let rhs = rhs?;
                if let Expr::Value(ref x) = rhs {
                    Ok(Expr::Value(-x))
                } else {
                    Ok(Expr::Primitive(Method::Neg, Box::new(rhs)))
                }
            }
            _ => unreachable!(),
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            Rule::fact => Ok(Expr::Primitive(Method::Fact, Box::new(lhs?))),
            _ => unreachable!(),
        })
        .parse(pairs)
}

fn parse_template(inner: Pairs<'_, Rule>) -> Result<Vec<Template>> {
    let mut acc = Vec::new();
    for val in inner {
        let val = match val.as_rule() {
            Rule::field => {
                let pairs = val.into_inner();
                let expr = parse_expr(pairs)?;
                Template::Field(expr)
            }
            Rule::string => Template::String(val.to_string()),
            Rule::escaped => {
                let Some(s) = unescape(val.as_str()) else {
                    bail!("invalid escape sequence: {}", val.as_str())
                };
                Template::String(s.to_string())
            }
            _ => bail!("unexpected {}", val),
        };
        acc.push(val);
    }
    Ok(acc)
}

fn parse_imag(pair: Pair<Rule>) -> Result<f64> {
    let mut s = pair.to_string();
    s.pop();
    if s.is_empty() {
        return Ok(1f64);
    }
    Ok(f64::from_str(&s)?)
}

fn unescape(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let Some('\\') = chars.next() else {
        return None;
    };
    let c = match chars.next().unwrap() {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '0' => '\0',
        'b' => '\u{000C}',
        // \xNN
        'x' => {
            let mut s = String::new();
            for _ in 0..2 {
                if let Some(c) = chars.next() {
                    s.push(c);
                } else {
                    return None;
                };
            }
            let Ok(u) = u32::from_str_radix(&s, 16) else {
                return None;
            };
            return char::from_u32(u);
        }
        // \uNNNN
        'u' => {
            let mut s = String::new();
            for _ in 0..4 {
                if let Some(c) = chars.next() {
                    s.push(c);
                } else {
                    return None;
                };
            }
            let Ok(u) = u32::from_str_radix(&s, 16) else {
                return None;
            };
            return char::from_u32(u);
        }
        // in particular: \' \" \) \{
        c => c,
    };
    Some(c)
}
