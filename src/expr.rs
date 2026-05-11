use crate::algebra::Algebra;
use std::path::PathBuf;

#[derive(Clone)]
pub enum Expr {
    Value(Algebra),
    NewVec(Vec<Expr>),
    Variable(String),
    Primitive(Method, Box<Expr>),
    BinaryOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>,
    },
    Function(Function),
    Apply(String, Vec<Expr>),
    IfElse(Box<Expr>, Box<Expr>, Box<Expr>),
    Load(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub(crate) name: String,
    pub(crate) args: Vec<String>,
    pub(crate) body: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    And,
    BitAnd,
    BitOr,
    Both,
    Div,
    Dot,
    Eq,
    EqType,
    Ge,
    Get,
    Gt,
    In,
    Le,
    Lt,
    Mul,
    Ne,
    Or,
    Pow,
    Rem,
    Sub,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Abs,
    Acos,
    Acosh,
    Asin,
    Asinh,
    Atan,
    Atanh,
    Cbrt,
    Ceil,
    Cos,
    Cosh,
    Erf,
    Erfc,
    Exp,
    Fact,
    Floor,
    Gamma,
    Lgamma,
    Ln,
    Log10,
    Log2,
    Neg,
    Round,
    Sin,
    Sinh,
    Sqrt,
    Tan,
    Tanh,
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Expr::*;
        match self {
            Value(n) => write!(f, "{}", n),
            NewVec(exprs) => {
                let values = exprs
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "[{}]", values)
            }
            Variable(s) => write!(f, "{}", s),
            Primitive(m, e) => {
                use self::Method::*;
                match m {
                    Neg => write!(f, "-{}", e),
                    Fact => write!(f, "{}!", e),
                    _ => write!(f, "{}({})", m, e),
                }
            }
            BinaryOp { lhs, op, rhs } => write!(f, "{} {} {}", lhs, op, rhs),
            Function(self::Function { name, args, body }) => {
                let args = args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let body = body
                    .iter()
                    .map(|e| format!("\t{}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "fun {}({}) {{\n{}\n}}", name, args, body)
            }
            Apply(n, a) => write!(
                f,
                "{}({})",
                n,
                a.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            IfElse(cond, yes, no) => write!(f, "if {} then {} else {}", cond, yes, no),
            Load(path) => write!(f, "load({})", path.to_str().unwrap_or_default()),
        }
    }
}

impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Expr::*;
        match self {
            BinaryOp { lhs, op, rhs } => {
                let lhs = if matches!(
                    lhs.as_ref(),
                    Expr::BinaryOp {
                        lhs: _,
                        op: _,
                        rhs: _
                    }
                ) {
                    format!("({:?})", lhs)
                } else {
                    lhs.to_string()
                };
                let rhs = if matches!(
                    rhs.as_ref(),
                    Expr::BinaryOp {
                        lhs: _,
                        op: _,
                        rhs: _
                    }
                ) {
                    format!("({:?})", rhs)
                } else {
                    rhs.to_string()
                };
                write!(f, "{} {} {}", lhs, op, rhs)
            }
            _ => write!(f, "{}", self),
        }
    }
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Op::*;
        match self {
            Add => write!(f, "+"),
            And => write!(f, "and"),
            Dot => write!(f, "@"),
            BitAnd => write!(f, "&"),
            BitOr => write!(f, "|"),
            Both => write!(f, "~"),
            Div => write!(f, "/"),
            Eq => write!(f, "="),
            EqType => write!(f, "?="),
            Ge => write!(f, ">="),
            Get => write!(f, ":"),
            Gt => write!(f, ">"),
            In => write!(f, "in"),
            Le => write!(f, "<="),
            Lt => write!(f, "<"),
            Mul => write!(f, "*"),
            Ne => write!(f, "!="),
            Or => write!(f, "or"),
            Pow => write!(f, "^"),
            Rem => write!(f, "%"),
            Sub => write!(f, "-"),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Method::*;
        match self {
            Abs => write!(f, "abs"),
            Acos => write!(f, "acos"),
            Acosh => write!(f, "acosh"),
            Asin => write!(f, "asin"),
            Asinh => write!(f, "asinh"),
            Atan => write!(f, "atan"),
            Atanh => write!(f, "atanh"),
            Cbrt => write!(f, "cbrt"),
            Ceil => write!(f, "ceil"),
            Cos => write!(f, "cos"),
            Cosh => write!(f, "cosh"),
            Erf => write!(f, "erf"),
            Erfc => write!(f, "erfc"),
            Exp => write!(f, "exp"),
            Fact => write!(f, "factorial"),
            Floor => write!(f, "floor"),
            Gamma => write!(f, "gamma"),
            Lgamma => write!(f, "lgamma"),
            Ln => write!(f, "ln"),
            Log10 => write!(f, "log10"),
            Log2 => write!(f, "log2"),
            Neg => write!(f, "negate"),
            Round => write!(f, "round"),
            Sin => write!(f, "sin"),
            Sinh => write!(f, "sinh"),
            Sqrt => write!(f, "sqrt"),
            Tan => write!(f, "tan"),
            Tanh => write!(f, "tanh"),
        }
    }
}
