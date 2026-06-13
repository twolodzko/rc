use crate::{COMPLEX, PRINT_AS_FLOAT, SCALE, expr::Method};
use Number::*;
use anyhow::{Result, bail};
use num::{
    Integer,
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

#[derive(Clone, Debug)]
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
                _ => {
                    if let Some(x) = self.to_f64() {
                        x.$t().into()
                    } else {
                        Number::NAN
                    }
                }
            }
        }
    )*)
}

macro_rules! impl_complex_method {
    ($($t:tt)*) => ($(
        pub fn $t(&self) -> Number {
            if let Complex(n) = self {
                n.$t().into()
            } else if unsafe { COMPLEX } && self.is_negative() {
                self.to_complex().map(|x| x.$t().into()).unwrap_or(Number::NAN)
            } else if let Some(x) = self.to_f64() {
                x.$t().into()
            } else {
                Number::NAN
            }
        }
    )*)
}

macro_rules! impl_libm {
    ($($t:tt)*) => ($(
        pub fn $t(&self) -> Number {
            if let Some(x) = self.to_f64() {
                libm::$t(x).into()
            } else {
                Number::NAN
            }
        }
    )*)
}

impl Number {
    pub const NAN: Number = Float(OrderedFloat(f64::NAN));
    pub const INFINITY: Number = Float(OrderedFloat(f64::INFINITY));
    pub const NEG_INFINITY: Number = Float(OrderedFloat(f64::NEG_INFINITY));
    pub const ZERO: Number = Integer(BigInt::ZERO);

    impl_method!(cbrt exp sin cos tan asin acos atan tanh sinh cosh asinh acosh atanh);
    impl_complex_method!(sqrt ln log2 log10);
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

    /// Number is an even integer
    pub fn is_even(&self) -> bool {
        self.to_bigint().is_some_and(|x| x.is_even())
    }

    pub fn is_complex(&self) -> bool {
        matches!(self, Complex(_))
    }

    pub fn abs(&self) -> Number {
        match self {
            Integer(x) => Integer(x.abs()),
            Rational(x) => Rational(x.abs()),
            Float(x) => Float(x.abs()),
            Complex(x) => x.norm().into(),
        }
    }

    fn powf(&self, rhs: f64) -> Number {
        if unsafe { COMPLEX } && self.is_negative() {
            self.to_complex()
                .map(|c| Complex(c.powf(rhs)))
                .unwrap_or(Number::NAN)
        } else if let Some(x) = self.to_f64() {
            Float(x.powf(rhs).into())
        } else {
            Number::NAN
        }
    }

    fn powi(&self, rhs: &BigInt) -> Number {
        match self {
            Integer(x) => {
                if let Ok(n) = TryInto::<u32>::try_into(rhs.abs()) {
                    if rhs.is_negative() {
                        Rational(Ratio::new(BigInt::one(), x.pow(n)))
                    } else {
                        Integer(x.pow(n))
                    }
                } else if let Some(rhs) = rhs.to_f64() {
                    self.powf(rhs)
                } else {
                    Number::NAN
                }
            }
            Rational(x) => {
                if let Ok(n) = TryInto::<i32>::try_into(rhs) {
                    Rational(x.pow(n))
                } else if let Some(rhs) = rhs.to_f64() {
                    self.powf(rhs)
                } else {
                    Number::NAN
                }
            }
            Float(x) => {
                if let Ok(n) = TryInto::<i32>::try_into(rhs) {
                    Float(x.powi(n))
                } else if let Some(rhs) = rhs.to_f64() {
                    self.powf(rhs)
                } else {
                    Number::NAN
                }
            }
            Complex(x) => {
                if let Ok(n) = TryInto::<i32>::try_into(rhs) {
                    x.powi(n).into()
                } else if let Some(rhs) = rhs.to_f64() {
                    x.powf(rhs).into()
                } else {
                    Number::NAN
                }
            }
        }
    }

    pub fn floor(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.floor().to_integer()),
            Float(x) => x.floor().to_bigint().map(Integer).unwrap_or(Number::NAN),
            Complex(x) => num::Complex::new(x.re.floor(), x.im.floor()).into(),
        }
    }

    pub fn ceil(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.ceil().to_integer()),
            Float(x) => x.ceil().to_bigint().map(Integer).unwrap_or(Number::NAN),
            Complex(x) => num::Complex::new(x.re.ceil(), x.im.ceil()).into(),
        }
    }

    pub fn round(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.round().to_integer()),
            Float(x) => x.round().to_bigint().map(Integer).unwrap_or(Number::NAN),
            Complex(x) => num::Complex::new(x.re.round(), x.im.round()).into(),
        }
    }

    fn trunc(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.trunc().to_integer()),
            Float(x) => x.trunc().to_bigint().map(Integer).unwrap_or(Number::NAN),
            Complex(x) => num::Complex::new(x.re.trunc(), x.im.trunc()).into(),
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
        let Some(n) = self.to_bigint() else {
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
        let Some(n) = self.to_bigint() else {
            return Number::NAN;
        };
        let Some(k) = k.to_bigint() else {
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
            Float(x) => to_rat(x.0),
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
            Integer(x) => x.to_f64().unwrap_or(f64::NAN).into(),
            Rational(x) => x.to_f64().unwrap_or(f64::NAN).into(),
            Complex(x) => x.to_f64().unwrap_or(f64::NAN).into(),
            Float(_) => self.clone(),
        }
    }

    /// Cast number to rational if possible
    fn cast_to_rational(&self) -> Number {
        match self {
            Integer(i) => Rational(Ratio::from_integer(i.clone())),
            Rational(_) => self.clone(),
            Float(f) if let Some(r) = Ratio::from_float(f.0) => Rational(r),
            Complex(c)
                if let Some(f) = c.to_f64()
                    && let Some(r) = Ratio::from_float(f) =>
            {
                Rational(r)
            }
            _ => Number::NAN,
        }
    }

    pub fn to_bigint(&self) -> Option<BigInt> {
        if !self.is_integerish() {
            return None;
        }
        match self {
            Integer(x) => Some(x.clone()),
            Rational(x) => {
                if !x.is_integer() {
                    return None;
                }
                Some(x.to_integer())
            }
            Float(x) => x.to_i128().map(|i| i.into()),
            Complex(x) => x.to_i128().map(|i| i.into()),
        }
    }

    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Integer(x) => x.to_f64(),
            Rational(x) => x.to_f64(),
            Float(x) => Some(x.0),
            Complex(x) => x.to_f64(),
        }
    }

    pub fn to_complex(&self) -> Option<Complex<f64>> {
        debug_assert!(unsafe { COMPLEX });
        match self {
            Integer(x) => Some(Complex::from(x.to_f64()?)),
            Rational(x) => Some(Complex::from(x.to_f64()?)),
            Float(x) => Some(Complex::from(x.0)),
            Complex(x) => Some(*x),
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

    /// Compute x^(1/n)
    fn nth_root(&self, n: &BigInt) -> Number {
        // https://math.stackexchange.com/a/1608619
        if let Complex(x) = self {
            complex_nth_root(x, n).into()
        } else if let Some(x) = self.to_f64() {
            f64_nth_root(x, n).into()
        } else {
            Number::NAN
        }
    }
}

fn complex_nth_root(x: &num::Complex<f64>, n: &BigInt) -> num::Complex<f64> {
    if n == &BigInt::from(2) {
        x.sqrt()
    } else if n == &BigInt::from(3) {
        x.cbrt()
    } else if let Some(n) = n.to_f64() {
        x.powf(n.inv())
    } else {
        f64::NAN.into()
    }
}

fn f64_nth_root(x: f64, n: &BigInt) -> f64 {
    if n == &BigInt::from(2) {
        x.sqrt()
    } else if n == &BigInt::from(3) {
        x.cbrt()
    } else if let Some(n) = n.to_f64() {
        x.powf(n.inv())
    } else {
        f64::NAN
    }
}

/// Return rational approximation of a float or NaN if not possible
fn to_rat(f: f64) -> Number {
    if f.is_nan() || f.is_infinite() {
        return Number::NAN;
    }
    if let Some(r) = Ratio::from_float(f) {
        return Rational(r);
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
            (Complex(a), _) => {
                if let Some(b) = $rhs.to_f64() {
                    a.$method(b).into()
                } else {
                    Number::NAN
                }
            }
            (_, Complex(b)) => {
                if let Some(a) = $lhs.to_f64() {
                    a.$method(b).into()
                } else {
                    Number::NAN
                }
            }
            (Float(a), _) => {
                if let Some(b) = $rhs.to_f64() {
                    a.0.$method(b).into()
                } else {
                    Number::NAN
                }
            }
            (_, Float(b)) => {
                if let Some(a) = $lhs.to_f64() {
                    a.$method(b.0).into()
                } else {
                    Number::NAN
                }
            }
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
                if unsafe { COMPLEX } && xm.is_negative() && n.is_even() {
                    xm.to_complex()
                        .map(|ref xm| complex_nth_root(xm, n).into())
                        .unwrap_or(Number::NAN)
                } else {
                    xm.nth_root(n)
                }
            }
            // complex powers
            (Complex(x), Float(p)) => x.powf(p.0).into(),
            (Complex(x), Complex(p)) => x.powc(*p).into(),
            (_, Complex(p)) => self
                .to_complex()
                .map(|c| c.powc(*p).into())
                .unwrap_or(Number::NAN),
            // float powers
            (_, Float(rhs)) if *rhs == 0.5 => self.sqrt(),
            _ => rhs.to_f64().map(|x| self.powf(x)).unwrap_or(Number::NAN),
        }
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
            Complex(x) => x.inv().into(),
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

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Number::Float(value.into())
    }
}

impl From<usize> for Number {
    fn from(value: usize) -> Self {
        Number::Integer(value.into())
    }
}

impl From<Complex<f64>> for Number {
    fn from(value: Complex<f64>) -> Self {
        Complex(value)
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

/// Unify types of two numbers
fn same_types<'a, 'b>(lhs: &'a Number, rhs: &'b Number) -> (Cow<'a, Number>, Cow<'b, Number>) {
    match (lhs, rhs) {
        // cast to complex
        (Complex(_), _) => {
            let rhs = rhs.to_complex().map(Complex).unwrap_or(Number::NAN);
            (Cow::Borrowed(lhs), Cow::Owned(rhs))
        }
        (_, Complex(_)) => {
            let lhs = lhs.to_complex().map(Complex).unwrap_or(Number::NAN);
            (Cow::Owned(lhs), Cow::Borrowed(rhs))
        }
        // cast to floats
        (Float(_), _) => (Cow::Borrowed(lhs), Cow::Owned(rhs.cast_to_float())),
        (_, Float(_)) => (Cow::Owned(lhs.cast_to_float()), Cow::Borrowed(rhs)),
        // cast to integers
        (Rational(x), Integer(_)) if x.is_integer() => {
            (Cow::Owned(Integer(x.to_integer())), Cow::Borrowed(rhs))
        }
        (Integer(_), Rational(x)) if x.is_integer() => {
            (Cow::Borrowed(lhs), Cow::Owned(Integer(x.to_integer())))
        }
        // cast to rationals
        (Rational(_), _) => (Cow::Borrowed(lhs), Cow::Owned(rhs.cast_to_rational())),
        (_, Rational(_)) => (Cow::Owned(lhs.cast_to_rational()), Cow::Borrowed(rhs)),
        // take as-is
        (_, _) => (Cow::Borrowed(lhs), Cow::Borrowed(rhs)),
    }
}
