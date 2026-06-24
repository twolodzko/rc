use crate::{
    Algebra, ArityError, COMPLEX, Template,
    expr::{Expr, Function, Method, Op},
    number::Number,
    to_complex, to_float,
};
use anyhow::{Result, bail};
use core::f64;
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
    let name = parse_var(inner.next().unwrap());

    let mut args = Vec::new();
    if inner.peek().unwrap().as_rule() == Rule::parameters {
        let iter = inner.next().unwrap().into_inner();
        for pair in iter {
            args.push(parse_var(pair));
        }
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

fn parse_var(pair: Pair<Rule>) -> String {
    debug_assert_eq!(pair.as_rule(), Rule::name);
    pair.to_string()
}

fn parse_expr(pairs: Pairs<Rule>) -> Result<Expr> {
    PRATT_PARSER
        .map_primary(parse_primary)
        .map_infix(|lhs, op, rhs| {
            let op = match op.as_rule() {
                Rule::add => Op::Add,
                Rule::at => Op::Dot,
                Rule::bit_and => Op::BitAnd,
                Rule::bit_or => Op::BitOr,
                Rule::div => Op::Div,
                Rule::eq => Op::Eq,
                Rule::ge => Op::Ge,
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
            let lhs = lhs?;
            let rhs = rhs?;
            if op == Op::Pow {
                match lhs {
                    Expr::Variable(ref n) if n == "e" => {
                        return Ok(Expr::Primitive(Method::Exp, Box::new(rhs)));
                    }
                    Expr::Value(Algebra::Number(x)) if x.to_integer().is_some_and(|x| x == 2) => {
                        return Ok(Expr::Primitive(Method::Exp2, Box::new(rhs)));
                    }
                    _ => (),
                }
            }
            Ok(Expr::BinaryOp {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
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

fn parse_primary(primary: Pair<'_, Rule>) -> Result<Expr> {
    match primary.as_rule() {
        Rule::expression | Rule::brackets => parse_expr(primary.into_inner()),
        Rule::apply => {
            let mut inner = primary.into_inner();
            let name = parse_var(inner.next().unwrap());
            let mut args = Vec::new();
            if let Some(inner) = inner.next() {
                args = parse_args(inner)?;
            }

            let method = match name.as_str() {
                "abs" => Method::Abs,
                "acos" => Method::Acos,
                "acosh" => Method::Acosh,
                "asin" => Method::Asin,
                "asinh" => Method::Asinh,
                "atan" => Method::Atan,
                "atanh" => Method::Atanh,
                "ceil" => Method::Ceil,
                "cos" => Method::Cos,
                "cosh" => Method::Cosh,
                "deg" => Method::Deg,
                "erf" => Method::Erf,
                "erfc" => Method::Erfc,
                "exp" => Method::Exp,
                "factorial" => Method::Fact,
                "floor" => Method::Floor,
                "gamma" => Method::Gamma,
                "lgamma" => Method::Lgamma,
                "ln" | "log" => Method::Ln,
                "log10" => Method::Log10,
                "log2" => Method::Log2,
                "rad" => Method::Rad,
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
        Rule::term => parse_expr(primary.into_inner()),
        Rule::name => {
            let expr = match primary.as_str() {
                "nan" => Expr::Value(Algebra::Number(Number::nan())),
                "inf" => Expr::Value(Algebra::Number(Number::inf())),
                _ => Expr::Variable(primary.to_string()),
            };
            Ok(expr)
        }
        Rule::float => {
            let s = primary.as_str();
            let number = if let Ok(value) = rug::Integer::from_str(s) {
                Expr::Value(Algebra::Number(Number::Integer(value)))
            } else {
                let value = rug::Float::parse(s)?;
                Expr::Value(Algebra::Number(Number::Float(to_float(value))))
            };
            Ok(number)
        }
        Rule::complex => {
            if unsafe { !COMPLEX } {
                bail!(
                    "attempting to use complex number {} without --complex flag",
                    primary.as_str()
                )
            }
            let mut inner = primary.into_inner();
            let first = inner.next().unwrap();
            let mut real = 0f64;
            let imag = if first.as_rule() == Rule::imaginary {
                parse_imag(first)?
            } else {
                real = f64::from_str(first.as_str())?;
                parse_imag(inner.next().unwrap())?
            };
            Ok(Expr::Value(Algebra::Number(Number::Complex(to_complex((
                real, imag,
            ))))))
        }
        Rule::vector => {
            let mut acc = Vec::new();
            if let Some(inner) = primary.into_inner().next() {
                acc = parse_args(inner)?;
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
        Rule::list_get => {
            let mut inner = primary.into_inner();
            let mut expr = parse_primary(inner.next().unwrap())?;
            for next in inner {
                debug_assert_eq!(next.as_rule(), Rule::arguments);
                let mut acc = Vec::new();
                for pair in next.into_inner() {
                    let index = parse_primary(pair)?;
                    acc.push(index);
                }
                expr = Expr::VecGet(Box::new(expr), acc)
            }
            Ok(expr)
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
    }
}

fn parse_args(primary: Pair<'_, Rule>) -> Result<Vec<Expr>> {
    debug_assert_eq!(primary.as_rule(), Rule::arguments);
    let mut acc = Vec::new();
    let inner = primary.into_inner();
    for pair in inner {
        let expr = parse_expr(pair.into_inner())?;
        acc.push(expr);
    }
    Ok(acc)
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
