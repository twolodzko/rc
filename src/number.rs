use crate::{PRINT_AS_FLOAT, SCALE, expr::Method};
use Number::*;
use anyhow::{Result, bail};
use num::{
    bigint::{BigInt, ToBigInt},
    complex::Complex,
    rational::Ratio,
    traits::{Float, Inv, One, Pow, Signed, ToPrimitive, Zero},
};
use ordered_float::OrderedFloat;
use std::{
    borrow::Cow,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

#[derive(Clone)]
pub enum Number {
    Integer(BigInt),
    Rational(Ratio<BigInt>),
    Float(OrderedFloat<f64>),
    Complex(Complex<f64>),
}

macro_rules! impl_method {
    ($($t:tt)*) => ($(
        pub fn $t(&self) -> Number {
            match self {
                Complex(n) => Complex(n.$t()),
                _ => Float(self.to_f64().$t().into())
            }
        }
    )*)
}

macro_rules! impl_libm {
    ($($t:tt)*) => ($(
        pub fn $t(&self) -> Number {
            Float(libm::$t(self.to_f64()).into())
        }
    )*)
}

impl Number {
    pub const NAN: Number = Float(OrderedFloat(f64::NAN));
    pub const INFINITY: Number = Float(OrderedFloat(f64::INFINITY));
    pub const NEG_INFINITY: Number = Float(OrderedFloat(f64::NEG_INFINITY));
    pub const ZERO: Number = Integer(BigInt::ZERO);

    impl_method!(sqrt cbrt ln log2 log10 exp sin cos tan asin acos atan tanh sinh cosh asinh acosh atanh);
    impl_libm!(erf erfc lgamma tgamma);

    pub fn is_zero(&self) -> bool {
        match self {
            Integer(x) => x.is_zero(),
            Rational(x) => x.is_zero(),
            Float(x) => x.is_zero(),
            Complex(x) => x.is_zero(),
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Integer(x) => x.is_one(),
            Rational(x) => x.is_one(),
            Float(x) => x.is_one(),
            Complex(x) => x.is_one(),
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Integer(x) => x.is_negative(),
            Rational(x) => x.is_negative(),
            Float(x) => x.is_negative(),
            Complex(_) => false,
        }
    }

    pub fn is_positive(&self) -> bool {
        match self {
            Integer(x) => x.is_positive(),
            Rational(x) => x.is_positive(),
            Float(x) => x.is_positive(),
            Complex(_) => false,
        }
    }

    pub fn is_infinite(&self) -> bool {
        match self {
            Float(x) => x.is_infinite(),
            Complex(x) => x.is_infinite(),
            _ => false,
        }
    }

    pub fn is_nan(&self) -> bool {
        match self {
            Float(x) => x.is_nan(),
            Complex(x) => x.is_nan(),
            _ => false,
        }
    }

    pub fn is_even(&self) -> bool {
        if let Integer(x) = self.cast_to_integer() {
            return (x % 2i32).is_zero();
        }
        false
    }

    pub fn abs(&self) -> Number {
        match self {
            Integer(x) => Integer(x.abs()),
            Rational(x) => Rational(x.abs()),
            Float(x) => Float(x.abs()),
            Complex(x) => Float(x.norm().into()),
        }
    }

    fn powf(&self, rhs: f64) -> f64 {
        self.to_f64().powf(rhs)
    }

    fn powi(&self, rhs: &BigInt) -> Number {
        match self {
            Integer(x) => {
                let Ok(n) = TryInto::<u32>::try_into(rhs.abs()) else {
                    let Some(rhs) = rhs.to_f64() else {
                        return Number::NAN;
                    };
                    return Float(self.powf(rhs).into());
                };
                if rhs.is_negative() {
                    Rational(Ratio::new(BigInt::one(), x.pow(n)))
                } else {
                    Integer(x.pow(n))
                }
            }
            Rational(x) => {
                let Ok(n) = TryInto::<i32>::try_into(rhs) else {
                    let Some(rhs) = rhs.to_f64() else {
                        return Number::NAN;
                    };
                    return Float(self.powf(rhs).into());
                };
                Rational(x.pow(n))
            }
            Float(x) => {
                let Ok(n) = TryInto::<i32>::try_into(rhs) else {
                    let Some(rhs) = rhs.to_f64() else {
                        return Number::NAN;
                    };
                    return Float(self.powf(rhs).into());
                };
                Float(x.powi(n))
            }
            Complex(x) => {
                let Ok(n) = TryInto::<i32>::try_into(rhs) else {
                    let Some(rhs) = rhs.to_f64() else {
                        return Number::NAN;
                    };
                    return Complex(x.powf(rhs));
                };
                Complex(x.powi(n))
            }
        }
    }

    pub fn floor(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.floor().to_integer()),
            Float(x) => {
                let Some(i) = x.floor().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
            Complex(x) => {
                let Some(i) = x.to_f64().unwrap_or(f64::NAN).floor().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
        }
    }

    pub fn ceil(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.ceil().to_integer()),
            Float(x) => {
                let Some(i) = x.ceil().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
            Complex(x) => {
                let Some(i) = x.to_f64().unwrap_or(f64::NAN).ceil().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
        }
    }

    pub fn round(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.round().to_integer()),
            Float(x) => {
                let Some(i) = x.round().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
            Complex(x) => {
                let Some(i) = x.to_f64().unwrap_or(f64::NAN).round().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
        }
    }

    fn trunc(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.trunc().to_integer()),
            Float(x) => {
                let Some(i) = x.trunc().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
            Complex(x) => {
                let Some(i) = x.to_f64().unwrap_or(f64::NAN).trunc().to_bigint() else {
                    return Number::NAN;
                };
                Integer(i)
            }
        }
    }

    pub fn idiv(&self, rhs: &Number) -> Number {
        if rhs.is_zero() {
            return Number::NAN;
        }
        match (self, rhs) {
            (Integer(a), Integer(b)) => Integer(a / b),
            _ => (self / rhs).trunc(),
        }
    }

    pub fn factorial(&self) -> Number {
        let Integer(n) = self.cast_to_integer() else {
            return Number::NAN;
        };
        if n.is_negative() {
            return Number::NAN;
        }
        if self.is_one() || self.is_zero() {
            return Integer(BigInt::one());
        }

        fn recfact(start: &BigInt, n: &BigInt) -> BigInt {
            // The "just use BigInt library" algorithm from:
            // http://www.luschny.de/math/factorial/FastFactorialFunctions.htm
            if n <= &16.into() {
                let mut r = start.clone();
                for i in num::iter::range(start + 1, start + n) {
                    r *= i;
                }
                return r;
            }
            let i = n / 2;
            let start2 = start + &i;
            let i2 = n - &i;
            recfact(start, &i) * recfact(&start2, &i2)
        }
        Integer(recfact(&BigInt::one(), &n))
    }

    pub fn choose(&self, k: &Number) -> Number {
        let Integer(n) = self.cast_to_integer() else {
            return Number::NAN;
        };
        let Integer(k) = k.cast_to_integer() else {
            return Number::NAN;
        };
        if k < 0.into() || n < k {
            return Number::NAN;
        }
        // see: https://en.wikipedia.org/wiki/Binomial_coefficient#Multiplicative_formula
        let mut acc = Ratio::from_integer(BigInt::one());
        for i in num::iter::range_inclusive(BigInt::one(), k) {
            // (n+1-i)/i
            let numer = &n + &BigInt::one() - &i;
            acc *= Ratio::new(numer.clone(), i.clone());
        }
        Integer(acc.to_integer())
    }

    pub fn rat(&self) -> Number {
        match self {
            Integer(_) | Rational(_) => self.clone(),
            Float(x) => to_rat(x.into_inner()),
            Complex(x) => to_rat(x.to_f64().unwrap_or(f64::NAN)),
        }
    }

    pub fn primitive(&self, method: Method) -> Result<Number> {
        use Method::*;
        if matches!(self, Complex(_)) && matches!(method, Erf | Erfc | Gamma | Lgamma) {
            bail!("{} is not implemented for complex numbers", method)
        }
        let val = match method {
            Abs => self.abs(),
            Acos => self.acos(),
            Acosh => self.acosh(),
            Asin => self.asin(),
            Asinh => self.asinh(),
            Atan => self.atan(),
            Atanh => self.atanh(),
            Cbrt => self.cbrt(),
            Ceil => self.ceil(),
            Cos => self.cos(),
            Cosh => self.cosh(),
            Erf => self.erf(),
            Erfc => self.erfc(),
            Exp => self.exp(),
            Fact => self.factorial(),
            Floor => self.floor(),
            Gamma => self.tgamma(),
            Lgamma => self.lgamma(),
            Ln => self.ln(),
            Log10 => self.log10(),
            Log2 => self.log2(),
            Neg => self.neg(),
            Round => self.round(),
            Sin => self.sin(),
            Sinh => self.sinh(),
            Sqrt => self.sqrt(),
            Tan => self.tan(),
            Tanh => self.tanh(),
        };
        Ok(val)
    }

    /// Attempt casting number to float or return NaN
    pub fn cast_to_float(&self) -> Number {
        match self {
            Integer(i) => Float(i.to_f64().unwrap_or(f64::NAN).into()),
            Rational(r) => Float(r.to_f64().unwrap_or(f64::NAN).into()),
            Complex(c) => Float(c.to_f64().unwrap_or(f64::NAN).into()),
            Float(_) => self.clone(),
        }
    }

    /// Cast number to rational if possible
    fn cast_to_rational(&self) -> Number {
        match self {
            Integer(i) => Rational(Ratio::from_integer(i.clone())),
            Rational(_) => self.clone(),
            _ => Number::NAN,
        }
    }

    /// Cast number to integer or return NaN
    pub(crate) fn cast_to_integer(&self) -> Number {
        if !self.is_integerish() {
            return Number::NAN;
        }
        match self {
            Integer(_) => self.clone(),
            Rational(x) => {
                if !x.is_integer() {
                    return Number::NAN;
                }
                Integer(x.to_integer())
            }
            Float(x) => {
                let Some(i) = x.to_i128() else {
                    return Number::NAN;
                };
                Integer(i.into())
            }
            Complex(x) => {
                let Some(i) = x.to_i128() else {
                    return Number::NAN;
                };
                Integer(i.into())
            }
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Integer(x) => x.to_f64(),
            Rational(x) => x.to_f64(),
            Float(x) => return x.into_inner(),
            Complex(x) => x.to_f64(),
        }
        .unwrap_or(f64::NAN)
    }

    pub fn to_complex(&self) -> Complex<f64> {
        match self {
            Integer(x) => Complex::new(x.to_f64().unwrap_or(f64::NAN), 0f64),
            Rational(x) => Complex::new(x.to_f64().unwrap_or(f64::NAN), 0f64),
            Float(x) => Complex::new(x.into_inner(), 0f64),
            Complex(x) => *x,
        }
    }

    /// Check if number if an integer or can be casted to integer
    fn is_integerish(&self) -> bool {
        match self {
            Integer(_) => true,
            Rational(x) => x.is_integer(),
            Float(x) => x.floor() == *x,
            Complex(x) => x.to_f64().is_some_and(|f| f.floor() == f),
        }
    }

    pub(crate) fn to_usize(&self) -> Option<usize> {
        match self {
            Integer(x) => x.to_usize(),
            Float(x) => x.to_usize(),
            Rational(x) => x.to_usize(),
            Complex(x) => x.to_usize(),
        }
    }
}

/// Return rational approximation of a float or NaN if not possible
fn to_rat(f: f64) -> Number {
    if f.is_nan() || f.is_infinite() {
        return Number::NAN;
    }
    let (mantissa, exponent, sign) = f.integer_decode();
    let sign = Ratio::from_integer(sign.into());
    let mantissa = Ratio::from_integer(mantissa.into());
    let base = Ratio::from_integer(BigInt::from(2));
    Rational(sign * mantissa * base.pow(exponent as i32))
}
macro_rules! op {
    ( $lhs:tt, $method:tt, $rhs:tt ) => {{
        match (&$lhs, &$rhs) {
            (Integer(a), Integer(b)) => Integer(a.$method(b)),
            (Integer(a), Rational(b)) => Rational(Ratio::from_integer(a.clone()).$method(b)),
            (Rational(a), Integer(b)) => Rational(a.$method(b)),
            (Rational(a), Rational(b)) => Rational(a.$method(b)),
            (Complex(a), Complex(b)) => Complex(a.$method(b)),
            (Complex(a), _) => Complex(a.$method($rhs.to_f64())),
            (_, Complex(b)) => Complex($lhs.to_f64().$method(b)),
            (Float(a), _) => Float(a.$method($rhs.to_f64())),
            (_, Float(b)) => Float(OrderedFloat::from($lhs.to_f64()).$method(b)),
        }
    }};
}

macro_rules! impl_op {
    ( $trait:ident, $method:ident) => {
        impl $trait for &Number {
            type Output = Number;

            fn $method(self, rhs: Self) -> Self::Output {
                op!(self, $method, rhs)
            }
        }
    };
}

impl_op!(Add, add);
impl_op!(Sub, sub);
impl_op!(Mul, mul);
impl_op!(Rem, rem);

impl Div for &Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            return Number::NAN;
        }
        if self.is_one() {
            return rhs.inv();
        }
        match (self, rhs) {
            (Integer(n), Integer(d)) => Rational(Ratio::new(n.clone(), d.clone())),
            (a, b) => op!(a, div, b),
        }
    }
}

impl Pow<&Number> for &Number {
    type Output = Number;

    fn pow(self, rhs: &Number) -> Number {
        if rhs.is_zero() {
            return Integer(BigInt::one());
        }
        if rhs.is_one() {
            return self.clone();
        }
        if self.is_zero() {
            if rhs.is_negative() {
                return Number::NAN;
            } else {
                return Number::ZERO;
            }
        }
        match (&self, &rhs) {
            (_, Integer(p)) => self.powi(p),
            (_, Rational(p)) => {
                // Special case for calculating nth roots. This is not the same as fractional powers,
                // because they can't handle negative bases.
                // see: https://users.rust-lang.org/t/properly-compute-the-nth-root-of-a-negative-number/42232
                // x^(m/n) = root(x^m, n) = root(x, n)^m
                let m = p.numer();
                let n = p.denom();
                let xm = self.powi(m);
                Float(nth_root(xm, n).into())
            }
            // complex powers
            (Complex(x), Float(p)) => Complex(x.powf(p.into_inner())),
            (Complex(x), Complex(p)) => Complex(x.powc(*p)),
            (_, Complex(p)) => Complex(self.to_complex().powc(*p)),
            // float powers
            (_, Float(rhs)) if *rhs == 0.5 => self.sqrt(),
            _ => Float(self.powf(rhs.to_f64()).into()),
        }
    }
}

/// Compute x^(1/n)
fn nth_root(x: Number, n: &BigInt) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if !x.is_negative() && n == &BigInt::from(2) {
        x.to_f64().sqrt()
    } else if n == &BigInt::from(3) {
        x.to_f64().cbrt()
    } else {
        let even_exp = (n % BigInt::from(2)).is_zero();
        let exp = n.to_f64().unwrap_or(f64::NAN).inv();
        let r = x.abs().powf(exp);
        if x.is_negative() && !even_exp { -r } else { r }
    }
}

impl std::ops::Neg for &Number {
    type Output = Number;

    fn neg(self) -> Self::Output {
        match self {
            Integer(x) => Integer(-x),
            Rational(x) => Rational(-x),
            Float(x) => Float(-x),
            Complex(x) => Complex(-x),
        }
    }
}

impl Inv for &Number {
    type Output = Number;

    fn inv(self) -> Self::Output {
        match self {
            Integer(x) => Rational(Ratio::new(BigInt::one(), x.clone())),
            Rational(x) => Rational(x.inv()),
            Float(x) => Float(x.recip()),
            Complex(x) => Complex(x.inv()),
        }
    }
}

impl std::cmp::PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        let (lhs, rhs) = same_types(self, other);
        match (lhs.as_ref(), rhs.as_ref()) {
            (Integer(a), Integer(b)) => a == b,
            (Rational(a), Rational(b)) => a == b,
            (Float(a), Float(b)) => a == b,
            (Complex(a), Complex(b)) => a == b,
            _ => false,
        }
    }
}

impl std::cmp::Eq for Number {}

impl std::cmp::PartialOrd for &Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let (lhs, rhs) = same_types(self, other);
        match (lhs.as_ref(), rhs.as_ref()) {
            (Integer(a), Integer(b)) => a.partial_cmp(b),
            (Rational(a), Rational(b)) => a.partial_cmp(b),
            (Float(a), Float(b)) => a.partial_cmp(b),
            // complex numbers cannot be ordered
            // https://math.stackexchange.com/questions/487997/total-ordering-on-complex-numbers
            _ => None,
        }
    }
}

impl std::cmp::Ord for &Number {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl From<usize> for Number {
    fn from(value: usize) -> Self {
        Number::Integer(value.into())
    }
}

macro_rules! write_scaled {
    ( $f:expr, $n:expr ) => {
        if let Some(s) = unsafe { SCALE } {
            write!($f, "{:.s$}", $n)
        } else {
            write!($f, "{}", $n)
        }
    };
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer(n) => write!(f, "{}", n),
            Rational(n) => {
                if unsafe { PRINT_AS_FLOAT } {
                    // to_f64 will still produce inf and nan, so this does not hurt
                    let n = n.to_f64().unwrap_or(f64::NAN);
                    write_scaled!(f, n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Float(n) => write_scaled!(f, n),
            Complex(n) => write_scaled!(f, n),
        }
    }
}

impl std::fmt::Debug for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer(n) => write!(f, "{}", n),
            Rational(n) => write!(f, "{}", n),
            Float(n) => {
                if n.is_nan() || n.is_infinite() {
                    write!(f, "{}", n)
                } else {
                    let (mantissa, exponent, sign) = n.integer_decode();
                    write!(f, "{}*{}*2^{}", sign, mantissa, exponent)
                }
            }
            Complex(n) => write!(f, "{}", n),
        }
    }
}

/// Unify types of two numbers
fn same_types<'a, 'b>(lhs: &'a Number, rhs: &'b Number) -> (Cow<'a, Number>, Cow<'b, Number>) {
    match (lhs, rhs) {
        (Float(_), _) => (Cow::Borrowed(lhs), Cow::Owned(rhs.cast_to_float())),
        (_, Float(_)) => (Cow::Owned(lhs.cast_to_float()), Cow::Borrowed(rhs)),
        (Complex(_), _) => (Cow::Borrowed(lhs), Cow::Owned(Complex(rhs.to_complex()))),
        (_, Complex(_)) => (Cow::Owned(Complex(lhs.to_complex())), Cow::Borrowed(rhs)),
        (Rational(x), Integer(_)) if x.is_integer() => {
            (Cow::Owned(Integer(x.to_integer())), Cow::Borrowed(rhs))
        }
        (Integer(_), Rational(x)) if x.is_integer() => {
            (Cow::Borrowed(lhs), Cow::Owned(Integer(x.to_integer())))
        }
        (Rational(_), _) => (Cow::Borrowed(lhs), Cow::Owned(rhs.cast_to_rational())),
        (_, Rational(_)) => (Cow::Owned(lhs.cast_to_rational()), Cow::Borrowed(rhs)),
        (_, _) => (Cow::Borrowed(lhs), Cow::Borrowed(rhs)),
    }
}
