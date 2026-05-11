use crate::{
    expr::{Method, Op},
    interval,
    number::{self, Number},
    vector,
};
use Algebra::*;
use anyhow::Result;
use num::traits::Pow;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

#[derive(Clone)]
pub enum Algebra {
    Scalar(number::Number),
    Interval(interval::Interval),
    Vector(vector::Vector),
}

macro_rules! impl_is_method {
    ($($t:tt)*) => ($(
        pub fn $t(&self) -> bool {
            match self {
                Scalar(x) => x.$t(),
                Interval(x) => x.$t(),
                Vector(x) => x.$t(),
            }
        }
    )*)
}

impl Algebra {
    pub const NAN: Algebra = Algebra::Scalar(number::Number::NAN);

    impl_is_method!(is_zero is_one is_negative is_nan is_infinite);

    pub fn op(&self, op: Op, rhs: &Algebra) -> Algebra {
        match op {
            Op::Add => self + rhs,
            Op::Sub => self - rhs,
            Op::Mul => self * rhs,
            Op::Div => self / rhs,
            Op::Rem => self % rhs,
            Op::Pow => self.pow(rhs),
            Op::BitOr => match (self, rhs) {
                (Scalar(a), Scalar(b)) => {
                    Interval(interval::Interval::ordered(a.clone(), b.clone()))
                }
                (Scalar(a), Interval(b)) => Interval(b.interval_hull(&a.into())),
                (Interval(a), Scalar(b)) => Interval(a.interval_hull(&b.into())),
                (Interval(a), Interval(b)) => Interval(a.interval_hull(b)),
                _ => Algebra::NAN,
            },
            Op::BitAnd => match (self, rhs) {
                (Scalar(a), Scalar(b)) => {
                    if a == b {
                        Interval(a.into())
                    } else {
                        Algebra::NAN
                    }
                }
                (Scalar(a), Interval(b)) => {
                    if b.contains(a) {
                        Interval(a.into())
                    } else {
                        Algebra::NAN
                    }
                }
                (Interval(a), Scalar(b)) => {
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
            (x @ Scalar(_), Vector(v)) => {
                iter::zip(iter::repeat(x), v.0.iter()).all(|(a, b)| a.compare(op, b))
            }
            (Vector(v), x @ Scalar(_)) => {
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
            Scalar(x) => Scalar(x.primitive(method)?),
            Interval(x) if x.is_singular() => Scalar(x.lower.primitive(method)?),
            Interval(x) => Interval(x.primitive(method)?),
            Vector(x) => Vector(x.primitive(method)?),
        };
        Ok(val)
    }

    pub fn choose(&self, other: &Algebra) -> Algebra {
        match (self, other) {
            (Scalar(a), Scalar(b)) => Scalar(a.choose(b)),
            (Scalar(a), Interval(b)) => Interval(interval::Interval::ordered(
                a.choose(&b.lower),
                a.choose(&b.upper),
            )),
            (Interval(a), Scalar(b)) => Interval(interval::Interval::ordered(
                a.lower.choose(b),
                a.upper.choose(b),
            )),
            (Interval(a), Interval(b)) => Interval(a.choose(b)),
            (Vector(a), Vector(b)) => Vector(a.zip_map(b, |(n, k)| n.choose(k))),
            (a, Vector(b)) => Vector(b.map(|x| a.choose(x))),
            (Vector(a), b) => Vector(a.map(|x| x.choose(b))),
        }
    }

    pub fn equal_type(&self, other: &Algebra) -> bool {
        use number::Number::*;
        matches!(
            (self, other),
            (Scalar(Integer(_)), Scalar(Integer(_)))
                | (Scalar(Rational(_)), Scalar(Rational(_)))
                | (Scalar(Float(_)), Scalar(Float(_)))
                | (Scalar(Complex(_)), Scalar(Complex(_)))
                | (Interval(_), Interval(_))
                | (Vector(_), Vector(_))
        )
    }

    pub fn map(&self, fun: fn(&Number) -> Number) -> Algebra {
        match self {
            Scalar(n) => Scalar(fun(n)),
            Interval(i) => Interval(interval::Interval {
                lower: fun(&i.lower),
                upper: fun(&i.upper),
            }),
            Vector(v) => Vector(v.map(|x| x.map(fun))),
        }
    }
}

macro_rules! op {
    ( $lhs:tt $op:tt $rhs:tt ) => {{
        match (&$lhs, &$rhs) {
            (Scalar(a), Scalar(b)) => Scalar(a $op b),
            (Scalar(a), Interval(b)) => Interval(interval::Interval::ordered(a $op &b.lower, a $op &b.upper)),
            (Interval(a), Scalar(b)) => Interval(interval::Interval::ordered(&a.lower $op b, &a.upper $op b)),
            (Vector(a), Vector(b)) => Vector(a $op b),
            (Interval(a), Interval(b)) => Interval(a $op b),
            (_, Vector(b)) => Vector(b.map(|x| $lhs $op x)),
            (Vector(a), _) => Vector(a.map(|x| x $op $rhs)),
        }
    }}
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
        op!(self + rhs)
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
        op!(self - rhs)
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
        op!(self * rhs)
    }
}

impl Div for &Algebra {
    type Output = Algebra;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_one() {
            return self.clone();
        }
        op!(self / rhs)
    }
}

impl Rem for &Algebra {
    type Output = Algebra;

    fn rem(self, rhs: Self) -> Self::Output {
        op!(self % rhs)
    }
}

impl Pow<&Algebra> for &Algebra {
    type Output = Algebra;

    fn pow(self, rhs: &Algebra) -> Self::Output {
        if rhs.is_one() {
            return self.clone();
        }
        match (&self, &rhs) {
            (Scalar(a), Scalar(b)) => Scalar(a.pow(b)),
            (Scalar(a), Interval(b)) => Interval(interval::Interval::ordered(
                a.pow(&b.lower),
                a.pow(&b.upper),
            )),
            (Interval(a), Scalar(b)) => {
                Interval(interval::Interval::ordered(a.lower.pow(b), a.upper.pow(b)))
            }
            (Interval(a), Interval(b)) => Interval(a.pow(b)),
            (Vector(a), Vector(b)) => Vector(a.zip_map(b, |(n, k)| n.pow(k))),
            (_, Vector(b)) => Vector(b.map(|x| self.pow(x))),
            (Vector(a), _) => Vector(a.map(|x| x.pow(rhs))),
        }
    }
}

impl Neg for &Algebra {
    type Output = Algebra;

    fn neg(self) -> Self::Output {
        match self {
            Scalar(x) => Scalar(-x),
            Interval(x) => Interval(-x),
            Vector(x) => Vector(-x),
        }
    }
}

impl std::cmp::PartialEq for Algebra {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Scalar(a), Scalar(b)) => a == b,
            (Scalar(a), Interval(b)) if b.is_singular() => a == &b.lower,
            (Interval(a), Scalar(b)) if a.is_singular() => &a.lower == b,
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
            (Scalar(a), Scalar(b)) => a.partial_cmp(&b),
            (Scalar(a), Interval(b)) => {
                if a < &b.lower {
                    Some(std::cmp::Ordering::Less)
                } else if a > &b.upper {
                    Some(std::cmp::Ordering::Greater)
                } else {
                    None
                }
            }
            (Interval(_), Scalar(_)) => other.partial_cmp(self).map(|o| o.reverse()),
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
            Scalar(x) => write!(f, "{}", x),
            Interval(x) => write!(f, "{}", x),
            Vector(x) => write!(f, "{}", x),
        }
    }
}

impl std::fmt::Debug for Algebra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scalar(x) => write!(f, "{:?}", x),
            Interval(x) => write!(f, "{:?}", x),
            Vector(x) => write!(f, "{:?}", x),
        }
    }
}
