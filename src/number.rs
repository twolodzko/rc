use crate::{
    COMPLEX, IntDiv, PRINT_AS_FLOAT, Pow, SCALE, expr::Method, ratio_to_integer, to_complex,
    to_float,
};
use Number::*;
use anyhow::{Result, bail};
use rug::{Assign, ops::Pow as _};
use std::{
    borrow::Cow,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

#[derive(Clone, Debug)]
pub enum Number {
    Integer(rug::Integer),
    Rational(rug::Rational),
    Float(rug::Float),
    Complex(rug::Complex),
}

macro_rules! impl_method {
    ($($t:tt)*) => ($(
        pub fn $t(self) -> Number {
            match self {
                Complex(n) => Complex(n.$t()),
                _ => {
                    if let Some(x) = self.to_float() {
                        x.$t().into()
                    } else {
                        Number::nan()
                    }
                }
            }
        }
    )*)
}

macro_rules! impl_complex_method {
    ($($t:tt)*) => ($(
        pub fn $t(self) -> Number {
            if let Complex(n) = self {
                n.clone().$t().into()
            } else if unsafe { COMPLEX } && self.is_negative() {
                self.to_complex().map(|x| Complex(x.$t())).unwrap_or_default()
            } else if let Some(x) = self.to_float() {
                x.$t().into()
            } else {
                Number::nan()
            }
        }
    )*)
}

macro_rules! impl_float_methods {
    ($($t:tt)*) => ($(
        pub fn $t(&self) -> Number {
            if let Some(x) = self.to_float() {
                Float(x.$t())
            } else {
                Number::nan()
            }
        }
    )*)
}

impl Number {
    pub const ZERO: Number = Integer(rug::Integer::ZERO);
    pub const ONE: Number = Integer(rug::Integer::ZERO);

    impl_method!(exp exp2 sin cos tan asin acos atan tanh sinh cosh asinh acosh atanh);
    impl_complex_method!(sqrt ln log2 log10);
    impl_float_methods!(erf erfc gamma ln_gamma);

    pub fn primitive(self, method: Method) -> Result<Number> {
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
            Ceil => self.ceil(),
            Cos => self.cos(),
            Cosh => self.cosh(),
            Deg => todo!(),
            Erf => self.erf(),
            Erfc => self.erfc(),
            Exp => self.exp(),
            Exp2 => self.exp2(),
            Fact => self.factorial(),
            Floor => self.floor(),
            Gamma => self.gamma(),
            Lgamma => self.ln_gamma(),
            Ln => self.ln(),
            Log10 => self.log10(),
            Log2 => self.log2(),
            Neg => self.neg(),
            Rad => todo!(),
            Round => self.round(),
            Sin => self.sin(),
            Sinh => self.sinh(),
            Sqrt => self.sqrt(),
            Tan => self.tan(),
            Tanh => self.tanh(),
        };
        Ok(val)
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Integer(x) => x.is_zero(),
            Rational(x) => x.is_zero(),
            Float(x) => *x == 0.0,
            Complex(x) => *x.real() == 0.0 && *x.imag() == 0.0,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Integer(x) => *x == 1,
            Rational(x) => *x == 1,
            Float(x) => *x == 1.0,
            Complex(x) => *x.real() == 1.0 && *x.imag() == 0.0,
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Integer(x) => x.is_negative(),
            Rational(x) => x.is_negative(),
            Float(x) => x.is_sign_negative(),
            Complex(_) => false,
        }
    }

    pub fn is_positive(&self) -> bool {
        match self {
            Integer(x) => x.is_positive(),
            Rational(x) => x.is_positive(),
            Float(x) => x.is_sign_positive(),
            Complex(_) => false,
        }
    }

    pub fn is_infinite(&self) -> bool {
        match self {
            Float(x) => x.is_infinite(),
            Complex(x) => x.real().is_infinite() || x.imag().is_infinite(),
            _ => false,
        }
    }

    pub fn is_nan(&self) -> bool {
        match self {
            Float(x) => x.is_nan(),
            Complex(x) => x.real().is_nan() || x.imag().is_nan(),
            _ => false,
        }
    }

    /// Number is an even integer
    pub fn is_even(&self) -> bool {
        self.to_integer().is_some_and(|x| x.is_even())
    }

    pub fn is_complex(&self) -> bool {
        matches!(self, Complex(_))
    }

    /// Check if number if an integer or can be casted to integer
    fn is_integer(&self) -> bool {
        match self {
            Integer(_) => true,
            Rational(x) => x.is_integer(),
            Float(x) => x.is_integer(),
            Complex(x) => x.imag().is_zero() && x.real().is_integer(),
        }
    }

    pub fn nan() -> Number {
        Float(to_float(rug::float::Special::Nan))
    }

    pub fn inf() -> Number {
        Float(to_float(rug::float::Special::Nan))
    }

    pub fn neg_inf() -> Number {
        Float(to_float(rug::float::Special::NegInfinity))
    }

    /// Attempt casting number to float or return NaN
    pub fn cast_to_float(&self) -> Number {
        self.to_float().map(Float).unwrap_or_default()
    }

    /// Cast number to rational if possible
    fn cast_to_rational(&self) -> Number {
        match self {
            Integer(i) => Rational(rug::Rational::from(i.clone())),
            Rational(_) => self.clone(),
            Float(f) if let Ok(r) = rug::Rational::try_from(f) => Rational(r),
            Complex(c)
                if c.imag().is_zero()
                    && let Some(r) = c.real().to_rational() =>
            {
                Rational(r)
            }
            _ => Number::nan(),
        }
    }

    pub fn to_integer(&self) -> Option<rug::Integer> {
        match self {
            Integer(x) => Some(x.clone()),
            Rational(x) => {
                if !x.is_integer() {
                    return None;
                }
                debug_assert_eq!(*x.denom(), 1);
                Some(x.numer().clone())
            }
            Float(x) => x.to_integer(),
            Complex(x) => {
                if x.imag().is_zero() {
                    return x.real().to_integer();
                }
                None
            }
        }
    }

    pub fn to_float(&self) -> Option<rug::Float> {
        match self {
            Integer(x) => Some(to_float(x)),
            Rational(x) => {
                let num = to_float(x.numer());
                let den = to_float(x.denom());
                Some(num / den)
            }
            Float(x) => Some(x.clone()),
            Complex(x) => {
                if x.imag().is_zero() {
                    return Some(x.real().clone());
                }
                None
            }
        }
    }

    pub fn to_complex(&self) -> Option<rug::Complex> {
        debug_assert!(unsafe { COMPLEX });
        match self {
            Integer(x) => Some(to_complex(x)),
            Rational(x) => Some(to_complex(x)),
            Float(x) => Some(to_complex(x)),
            Complex(x) => Some(x.clone()),
        }
    }

    pub fn to_usize(&self) -> Option<usize> {
        self.to_integer().map(|x| x.to_usize()).flatten()
    }

    pub fn abs(self) -> Number {
        match self {
            Integer(x) => Integer(x.clone().abs()),
            Rational(x) => Rational(x.clone().abs()),
            Float(x) => Float(x.abs().into()),
            // |a + bi| = sqrt(a^2 + b^2)
            Complex(x) => Float(x.norm().sqrt().real().clone()),
        }
    }

    fn powf(&self, rhs: &rug::Float) -> Number {
        if unsafe { COMPLEX } && self.is_negative() {
            self.to_complex()
                .map(|c| Complex(c.pow(rhs)))
                .unwrap_or_default()
        } else if let Some(x) = self.to_float() {
            x.pow(rhs).into()
        } else {
            Number::nan()
        }
    }

    fn powi(self, rhs: rug::Integer) -> Number {
        match self {
            Integer(x) => Integer(x.pow(rhs).into()),
            Rational(x) => Integer(x.pow(rhs).into()),
            Float(x) => Float(x.pow(rhs).into()),
            Complex(x) => Complex(x.pow(rhs).into()),
        }
    }

    fn inv(self) -> Number {
        match self {
            Integer(x) => Rational(rug::Rational::from((1, x.clone()))),
            Rational(x) => Rational(x.recip()),
            Float(x) => Float(x.recip().into()),
            Complex(x) => Complex(x.recip()),
        }
    }

    fn floor(self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.floor().numer().clone()),
            Float(x) => x.floor().to_integer().map(Integer).unwrap_or_default(),
            Complex(x) => to_complex((x.real().clone().floor(), x.imag().clone().floor())).into(),
        }
    }

    fn ceil(self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.ceil().numer().clone()),
            Float(x) => x.ceil().to_integer().map(Integer).unwrap_or_default(),
            Complex(x) => to_complex((x.real().clone().ceil(), x.imag().clone().ceil())).into(),
        }
    }

    fn round(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.clone().round().numer().clone()),
            Float(x) => x
                .clone()
                .round()
                .to_integer()
                .map(Integer)
                .unwrap_or_default(),
            Complex(x) => to_complex((x.real().clone().round(), x.imag().clone().round())).into(),
        }
    }

    fn trunc(&self) -> Number {
        match self {
            Integer(_) => self.clone(),
            Rational(x) => Integer(x.clone().trunc().numer().clone()),
            Float(x) => x
                .clone()
                .trunc()
                .to_integer()
                .map(Integer)
                .unwrap_or_default(),
            Complex(x) => to_complex((x.real().clone().trunc(), x.imag().clone().trunc())).into(),
        }
    }

    fn factorial(&self) -> Number {
        let Some(n) = self.to_integer() else {
            return Number::nan();
        };
        if n.is_negative() {
            return Number::nan();
        }
        if self.is_one() || self.is_zero() {
            return Number::nan();
        }

        fn recfact(start: rug::Integer, n: rug::Integer) -> rug::Integer {
            // The "just use BigInt library" algorithm from:
            // http://www.luschny.de/math/factorial/FastFactorialFunctions.htm
            if n <= 16 {
                let mut r = start.clone();
                let mut i: rug::Integer = start.clone() + 1;
                let mut this = rug::Integer::new();
                this.assign(&start + &n);
                while &i <= &this {
                    r *= &i;
                    i += 1;
                    this.assign(&start + &n);
                }
                return r;
            }
            let i: rug::Integer = n.clone() / 2;
            let start2: rug::Integer = (&start + &i).into();
            let i2: rug::Integer = (&n - &i).into();
            recfact(start, i) * recfact(start2, i2)
        }
        Integer(recfact(rug::Integer::ONE.clone(), n))
    }

    pub fn choose(&self, k: &Number) -> Number {
        let Some(n) = self.to_integer() else {
            return Number::nan();
        };
        let Some(k) = k.to_integer() else {
            return Number::nan();
        };
        if k.is_negative() || n < k {
            return Number::nan();
        }
        // see: https://en.wikipedia.org/wiki/Binomial_coefficient#Multiplicative_formula
        let mut acc = rug::Rational::from(1);
        let mut i: rug::Integer = 1.into();
        while i <= k {
            // (n+1-i)/i
            let num: rug::Integer = n.clone() + 1 - i.clone();
            acc *= rug::Rational::from((num.clone(), i.clone()));
            i += 1;
        }
        debug_assert_eq!(*acc.denom(), 1);
        Integer(acc.numer().clone())
    }

    pub fn rat(&self) -> Number {
        match self {
            Integer(_) | Rational(_) => self.clone(),
            Float(x) => x.to_rational().map(Rational).unwrap_or_default(),
            Complex(x) => {
                if x.imag().is_zero() && x.real().is_integer() {
                    return x.real().to_rational().map(Rational).unwrap_or_default();
                }
                Number::nan()
            }
        }
    }

    /// Compute x^(1/n)
    fn nth_root(self, n: &rug::Integer) -> Number {
        // https://math.stackexchange.com/a/1608619
        if let Complex(x) = self {
            complex_nth_root(x, n).into()
        } else if let Some(x) = self.to_float() {
            float_nth_root(x, n).into()
        } else {
            Number::nan()
        }
    }
}

fn complex_nth_root(x: rug::Complex, n: &rug::Integer) -> rug::Complex {
    if *n == 2 {
        x.sqrt()
    } else {
        let n = to_float(n);
        x.pow(n.recip())
    }
}

fn float_nth_root(x: rug::Float, n: &rug::Integer) -> rug::Float {
    if *n == 2 {
        x.sqrt()
    } else if *n == 3 {
        x.cbrt()
    } else if let Ok(n) = n.try_into() {
        x.root(n)
    } else {
        x.pow(n.to_f64().recip())
    }
}

/// Return rational approximation of a float or NaN if not possible
fn to_rat(f: f64) -> Number {
    if f.is_nan() || f.is_infinite() {
        return Number::nan();
    }
    if let Ok(r) = rug::Rational::try_from(f) {
        return Rational(r);
    }
    todo!()
    // let (mantissa, exponent, sign) = f.integer_decode();
    // let sign = rug::Rational::from(sign);
    // let mantissa = rug::Rational::from(mantissa);
    // let base = rug::Rational::from(2);
    // Rational(sign * mantissa * base.pow(exponent as i32))
}

macro_rules! op {
    ( $lhs:tt, $method:tt, $rhs:tt ) => {{
        match (&$lhs, &$rhs) {
            (Integer(a), Integer(b)) => Integer(a.$method(b).into()),
            (Integer(a), Rational(b)) => Rational(rug::Rational::from(a.clone()).$method(b)),
            (Rational(a), Integer(b)) => Rational(a.$method(b).into()),
            (Rational(a), Rational(b)) => Rational(a.$method(b).into()),
            (Complex(a), Complex(b)) => Complex(a.$method(b).into()),
            (Complex(a), _) => {
                if let Ok(b) = $rhs.try_into() {
                    Complex(a.$method(b).into())
                } else {
                    Number::nan()
                }
            }
            (_, Complex(b)) => {
                if let Some(a) = $lhs.to_float() {
                    Complex(a.into().$method(b).into())
                } else {
                    Number::nan()
                }
            }
            (Float(a), _) => {
                if let Some(b) = $rhs.to_float() {
                    a.$method(b).into()
                } else {
                    Number::nan()
                }
            }
            (_, Float(b)) => {
                if let Some(a) = $lhs.to_float() {
                    a.$method(b).into()
                } else {
                    Number::nan()
                }
            }
        }
    }};
}

macro_rules! impl_op {
    ( $trait:ident, $method:ident) => {
        impl $trait for Number {
            type Output = Number;

            fn $method(self, rhs: Number) -> Self::Output {
                op!(self, $method, rhs)
            }
        }

        impl $trait<&Number> for &Number {
            type Output = Number;

            fn $method(self, rhs: &Number) -> Self::Output {
                op!(self, $method, rhs)
            }
        }
    };
}

impl_op!(Add, add);
impl_op!(Sub, sub);
impl_op!(Mul, mul);

impl Div for &Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            return Number::nan();
        }
        if self.is_one() {
            return rhs.inv();
        }
        match (self, rhs) {
            (Integer(n), Integer(d)) => Rational(rug::Rational::from((n.clone(), d.clone()))),
            (a, b) => op!(a, div, b),
        }
    }
}

impl Rem for &Number {
    type Output = Number;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Complex(_), Complex(_)) => Number::nan(),
            (a, b) => op!(a, rem, b),
        }
    }
}

impl Pow for &Number {
    type Output = Number;

    fn pow(self, rhs: &Number) -> Self::Output {
        if rhs.is_zero() {
            return Number::ONE;
        }
        if rhs.is_one() {
            return self.clone();
        }
        if self.is_zero() {
            if rhs.is_negative() {
                return Number::nan();
            } else {
                return Number::ZERO;
            }
        }
        match (&self, &rhs) {
            (_, Integer(p)) => self.clone().powi(p.clone()),
            (_, Rational(p)) => {
                // Special case for calculating nth roots. This is not the same as fractional powers,
                // because they can't handle negative bases.
                // see: https://users.rust-lang.org/t/properly-compute-the-nth-root-of-a-negative-number/42232
                // x^(m/n) = root(x^m, n) = root(x, n)^m
                let m = p.numer();
                let n = p.denom();
                let xm = self.clone().powi(m.clone());
                if unsafe { COMPLEX } && xm.is_negative() && n.is_even() {
                    xm.to_complex()
                        .map(|ref xm| complex_nth_root(xm.clone(), n).into())
                        .unwrap_or_default()
                } else {
                    xm.nth_root(n)
                }
            }
            // complex powers
            (Complex(x), Float(p)) => Complex(x.clone().pow(p)),
            (Complex(x), Complex(p)) => Complex(x.clone().pow(p)),
            (_, Complex(p)) => self
                .to_complex()
                .map(|c| Complex(c.clone().pow(p)))
                .unwrap_or_default(),
            // float powers
            (_, Float(rhs)) if *rhs == 0.5 => self.clone().sqrt(),
            _ => rhs.to_float().map(|ref x| self.powf(x)).unwrap_or_default(),
        }
    }
}

impl IntDiv for &Number {
    type Output = Number;

    fn idiv(self, rhs: &Number) -> Self::Output {
        if rhs.is_zero() {
            return Number::nan();
        }
        match (&self, &rhs) {
            (Integer(a), Integer(b)) => Integer((a / b).into()),
            _ => (self / rhs).trunc(),
        }
    }
}

impl std::ops::Neg for Number {
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

impl From<rug::Complex> for Number {
    fn from(value: rug::Complex) -> Self {
        Complex(value)
    }
}

impl From<rug::Float> for Number {
    fn from(value: rug::Float) -> Self {
        Float(value)
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
                    // to_float will still produce inf and nan, so this does not hurt
                    let n = n.to_f64();
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

impl Default for Number {
    fn default() -> Self {
        Number::nan()
    }
}

/// Unify types of two numbers
fn same_types<'a, 'b>(lhs: &'a Number, rhs: &'b Number) -> (Cow<'a, Number>, Cow<'b, Number>) {
    match (lhs, rhs) {
        // cast to complex
        (Complex(_), _) => {
            let rhs = rhs.to_complex().map(Complex).unwrap_or_default();
            (Cow::Borrowed(lhs), Cow::Owned(rhs))
        }
        (_, Complex(_)) => {
            let lhs = lhs.to_complex().map(Complex).unwrap_or_default();
            (Cow::Owned(lhs), Cow::Borrowed(rhs))
        }
        // cast to floats
        (Float(_), _) => (Cow::Borrowed(lhs), Cow::Owned(rhs.cast_to_float())),
        (_, Float(_)) => (Cow::Owned(lhs.cast_to_float()), Cow::Borrowed(rhs)),
        // cast to integers
        (Rational(x), Integer(_)) if x.is_integer() => (
            Cow::Owned(ratio_to_integer(x.clone()).map(Integer).unwrap_or_default()),
            Cow::Borrowed(rhs),
        ),
        (Integer(_), Rational(x)) if x.is_integer() => (
            Cow::Borrowed(lhs),
            Cow::Owned(ratio_to_integer(x.clone()).map(Integer).unwrap_or_default()),
        ),
        // cast to rationals
        (Rational(_), _) => (Cow::Borrowed(lhs), Cow::Owned(rhs.cast_to_rational())),
        (_, Rational(_)) => (Cow::Owned(lhs.cast_to_rational()), Cow::Borrowed(rhs)),
        // take as-is
        (_, _) => (Cow::Borrowed(lhs), Cow::Borrowed(rhs)),
    }
}
