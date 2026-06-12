use crate::{
    expr::{Method, Op},
    interval, number, vector,
};
use Algebra::*;
use anyhow::Result;
use num::traits::Pow;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

#[derive(Debug, Clone)]
pub enum Algebra {
    Number(number::Number),
    Interval(interval::Interval),
    Vector(vector::Vector),
}

macro_rules! impl_is_method {
    ($($method:tt)*) => ($(
        pub fn $method(&self) -> bool {
            match self {
                Number(x) => x.$method(),
                Interval(x) => x.$method(),
                Vector(x) => {
                    if x.is_empty() {
                        return false;
                    }
                    x.all(|v| v.$method())
                },
            }
        }
    )*)
}

macro_rules! apply {
    ( $lhs:tt, $method:tt, $rhs:tt ) => {{
        match (&$lhs, &$rhs) {
            (Number(a), Number(b)) => Number(a.$method(b)),
            (Number(a), Interval(b)) => Interval(interval::Interval::ordered(
                a.$method(&b.lower),
                a.$method(&b.upper),
            )),
            (Interval(a), Number(b)) => Interval(interval::Interval::ordered(
                a.lower.$method(b),
                a.upper.$method(b),
            )),
            (Vector(a), Vector(b)) => Vector(a.zip_map(b, |(x, y)| x.$method(y))),
            (Interval(a), Interval(b)) => Interval(a.$method(b)),
            (_, Vector(b)) => Vector(b.map(|x| $lhs.$method(x))),
            (Vector(a), _) => Vector(a.map(|x| x.$method($rhs))),
        }
    }};
}

impl Algebra {
    pub const NAN: Algebra = Algebra::Number(number::Number::NAN);

    impl_is_method!(is_zero is_one is_negative is_nan is_infinite);

    pub fn op(&self, op: Op, rhs: &Algebra) -> Algebra {
        match op {
            Op::Add => self + rhs,
            Op::Sub => self - rhs,
            Op::Mul => self * rhs,
            Op::Div => self / rhs,
            Op::Idiv => self.idiv(rhs),
            Op::Rem => self % rhs,
            Op::Pow => self.pow(rhs),
            Op::BitOr => match (self, rhs) {
                (Number(a), Number(b)) => {
                    Interval(interval::Interval::ordered(a.clone(), b.clone()))
                }
                (Number(a), Interval(b)) => Interval(b.interval_hull(&a.into())),
                (Interval(a), Number(b)) => Interval(a.interval_hull(&b.into())),
                (Interval(a), Interval(b)) => Interval(a.interval_hull(b)),
                _ => Algebra::NAN,
            },
            Op::BitAnd => match (self, rhs) {
                (Number(a), Number(b)) => {
                    if a == b {
                        Interval(a.into())
                    } else {
                        Algebra::NAN
                    }
                }
                (Number(a), Interval(b)) => {
                    if b.contains(a) {
                        Interval(a.into())
                    } else {
                        Algebra::NAN
                    }
                }
                (Interval(a), Number(b)) => {
                    if a.contains(b) {
                        Interval(b.into())
                    } else {
                        Algebra::NAN
                    }
                }
                (Interval(a), Interval(b)) => Interval(a.intersection(b)),
                _ => Algebra::NAN,
            },
            _ => unreachable!(),
        }
    }

    pub fn compare(&self, op: Op, other: &Algebra) -> bool {
        use Op::*;
        use std::iter;
        match (self, other) {
            (x @ Number(_), Vector(v)) => {
                iter::zip(iter::repeat(x), v.0.iter()).all(|(a, b)| a.compare(op, b))
            }
            (Vector(v), x @ Number(_)) => {
                iter::zip(v.0.iter(), iter::repeat(x)).all(|(a, b)| a.compare(op, b))
            }
            (Vector(a), Vector(b)) if op == Ne => a != b,
            (Vector(a), Vector(b)) => a.zip(b).all(|(a, b)| a.compare(op, b)),
            _ => match op {
                Ne => self != other,
                Lt => self < other,
                Le => self <= other,
                Gt => self > other,
                Ge => self >= other,
                _ => unreachable!(),
            },
        }
    }

    pub fn primitive(&self, method: Method) -> Result<Algebra> {
        let val = match self {
            Number(x) => Number(x.primitive(method)?),
            Interval(x) if x.is_singular() => Number(x.lower.primitive(method)?),
            Interval(x) => Interval(x.primitive(method)?),
            Vector(x) => Vector(x.primitive(method)?),
        };
        Ok(val)
    }

    pub fn equal_type(&self, other: &Algebra) -> bool {
        if self.is_nan() {
            return other.is_nan();
        } else if other.is_nan() {
            return false;
        }
        use number::Number::*;
        matches!(
            (self, other),
            (Number(Integer(_)), Number(Integer(_)))
                | (Number(Rational(_)), Number(Rational(_)))
                | (Number(Float(_)), Number(Float(_)))
                | (Number(Complex(_)), Number(Complex(_)))
                | (Interval(_), Interval(_))
                | (Vector(_), Vector(_))
        )
    }

    pub fn map(&self, fun: fn(&number::Number) -> number::Number) -> Algebra {
        match self {
            Number(n) => Number(fun(n)),
            Interval(i) => Interval(interval::Interval {
                lower: fun(&i.lower),
                upper: fun(&i.upper),
            }),
            Vector(v) => Vector(v.map(|x| x.map(fun))),
        }
    }

    pub fn choose(&self, other: &Algebra) -> Algebra {
        apply!(self, choose, other)
    }

    pub fn idiv(&self, other: &Algebra) -> Algebra {
        apply!(self, idiv, other)
    }
}

impl Add for &Algebra {
    type Output = Algebra;

    fn add(self, rhs: Self) -> Self::Output {
        if self.is_zero() {
            return rhs.clone();
        }
        if rhs.is_zero() {
            return self.clone();
        }
        apply!(self, add, rhs)
    }
}

impl Sub for &Algebra {
    type Output = Algebra;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.is_zero() {
            return rhs.neg();
        }
        if rhs.is_zero() {
            return self.clone();
        }
        apply!(self, sub, rhs)
    }
}

impl Mul for &Algebra {
    type Output = Algebra;

    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_one() {
            return rhs.clone();
        }
        if rhs.is_one() {
            return self.clone();
        }
        apply!(self, mul, rhs)
    }
}

impl Div for &Algebra {
    type Output = Algebra;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_one() {
            return self.clone();
        }
        apply!(self, div, rhs)
    }
}

impl Rem for &Algebra {
    type Output = Algebra;

    fn rem(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (Number(a), Number(b)) => Number(a % b),
            (Interval(a), Number(b)) => Interval(a % &interval::Interval::from(b)),
            (Number(a), Interval(b)) => Interval(&interval::Interval::from(a) % b),
            (Interval(a), Interval(b)) => Interval(a % b),
            (Vector(a), Vector(b)) => Vector(a.zip_map(b, |(x, y)| x % y)),
            (_, Vector(b)) => Vector(b.map(|x| self % x)),
            (Vector(a), _) => Vector(a.map(|x| x % rhs)),
        }
    }
}

impl Pow<&Algebra> for &Algebra {
    type Output = Algebra;

    fn pow(self, rhs: &Algebra) -> Self::Output {
        if rhs.is_one() {
            return self.clone();
        }
        match (&self, &rhs) {
            (Number(a), Number(b)) => Number(a.pow(b)),
            (Interval(a), Number(b)) => Interval(a.pow(b)),
            (Number(a), Interval(b)) => Interval(interval::Interval::from(a).pow(b)),
            (Interval(a), Interval(b)) => Interval(a.pow(b)),
            (Vector(a), Vector(b)) => Vector(a.zip_map(b, |(x, y)| x.pow(y))),
            (_, Vector(b)) => Vector(b.map(|x| self.pow(x))),
            (Vector(a), _) => Vector(a.map(|x| x.pow(rhs))),
        }
    }
}

impl Neg for &Algebra {
    type Output = Algebra;

    fn neg(self) -> Self::Output {
        match self {
            Number(x) => Number(-x),
            Interval(x) => Interval(-x),
            Vector(x) => Vector(x.map(|v| -v)),
        }
    }
}

impl std::cmp::PartialEq for Algebra {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number(a), Number(b)) => a == b,
            (Number(a), Interval(b)) if b.is_singular() => a == &b.lower,
            (Interval(a), Number(b)) if a.is_singular() => &a.lower == b,
            (Interval(a), Interval(b)) => a == b,
            (Vector(a), Vector(b)) => a == b,
            _ => false,
        }
    }
}

impl std::cmp::Eq for Algebra {}

impl std::cmp::PartialOrd for &Algebra {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number(a), Number(b)) => a.partial_cmp(&b),
            (Number(a), Interval(b)) => {
                if a < &b.lower {
                    Some(std::cmp::Ordering::Less)
                } else if a > &b.upper {
                    Some(std::cmp::Ordering::Greater)
                } else {
                    None
                }
            }
            (Interval(_), Number(_)) => other.partial_cmp(self).map(|o| o.reverse()),
            (Interval(a), Interval(b)) => a.partial_cmp(&b),
            _ => None,
        }
    }
}

impl std::cmp::Ord for &Algebra {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl std::fmt::Display for Algebra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number(x) => write!(f, "{}", x),
            Interval(x) => write!(f, "{}", x),
            Vector(x) => write!(f, "{}", x),
        }
    }
}
