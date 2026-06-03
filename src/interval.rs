use crate::{
    expr::{Method, Op},
    number::Number,
};
use anyhow::{Result, bail};
use num::{
    BigInt,
    traits::{One, Pow},
};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

#[derive(Clone, PartialEq, Eq)]
pub struct Interval {
    pub lower: Number,
    pub upper: Number,
}

impl Interval {
    pub const NAN: Interval = Interval {
        lower: Number::NAN,
        upper: Number::NAN,
    };
    pub const INFINITY: Interval = Interval {
        lower: Number::NEG_INFINITY,
        upper: Number::INFINITY,
    };
    pub const ZERO: Interval = Interval {
        lower: Number::ZERO,
        upper: Number::ZERO,
    };

    /// Create an interval while validating if it has proper bounds
    pub fn checked(lhs: &Number, rhs: &Number) -> Result<Interval> {
        let this = Interval {
            lower: lhs.clone(),
            upper: rhs.clone(),
        };
        if lhs.is_nan() || rhs.is_nan() || lhs > rhs {
            bail!("{} interval has invalid bounds", &this)
        }
        Ok(this)
    }

    /// Create interval a~b ensuring that a <= b
    pub fn ordered(lhs: Number, rhs: Number) -> Interval {
        if lhs.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        if &lhs <= &rhs {
            Interval {
                lower: lhs,
                upper: rhs,
            }
        } else {
            Interval {
                lower: rhs,
                upper: lhs,
            }
        }
    }

    pub fn primitive(&self, method: Method) -> Result<Interval> {
        let lhs = self.lower.primitive(method)?;
        let rhs = self.upper.primitive(method)?;
        Ok(Interval::ordered(lhs, rhs))
    }

    pub fn is_zero(&self) -> bool {
        self.lower.is_zero() && self.upper.is_zero()
    }

    pub fn is_one(&self) -> bool {
        self.lower.is_one() && self.upper.is_one()
    }

    /// The values in the interval are negative
    pub fn is_negative(&self) -> bool {
        self.upper.is_negative()
    }

    /// The interval contains a NaN
    pub fn is_nan(&self) -> bool {
        self.lower.is_nan() || self.upper.is_nan()
    }

    /// The interval ranges from -inf to inf
    pub fn is_infinite(&self) -> bool {
        self.lower == Number::NEG_INFINITY && self.upper == Number::INFINITY
    }

    /// The interval ranges from a to a
    pub fn is_singular(&self) -> bool {
        self.lower == self.upper
    }

    /// Intersection of two intervals
    pub fn intersection(&self, rhs: &Interval) -> Interval {
        let lower = self.lower.max(&rhs.lower);
        let upper = self.upper.min(&rhs.upper);
        if lower > upper {
            return Interval::NAN;
        }
        Interval {
            lower: lower.clone(),
            upper: upper.clone(),
        }
    }

    /// The lowermost and uppermost bounds of two intervals
    pub fn interval_hull(&self, rhs: &Interval) -> Interval {
        Interval {
            lower: self.lower.min(&rhs.lower).clone(),
            upper: self.upper.max(&rhs.upper).clone(),
        }
    }

    pub fn choose(&self, k: &Interval) -> Interval {
        if self.is_nan() || k.is_nan() {
            return Interval::NAN;
        }
        let pairs = self.cartesian(k, |a, b| a.choose(b));
        Interval {
            lower: pairs.iter().min().unwrap().clone(),
            upper: pairs.iter().max().unwrap().clone(),
        }
    }

    /// Interval contains the value
    pub fn contains(&self, value: &Number) -> bool {
        if value.is_nan() {
            return self.is_nan();
        }
        &self.lower <= value && value <= &self.upper
    }

    /// Function is applied to all the pairs of the interval bounds
    pub fn cartesian(&self, other: &Interval, fun: fn(&Number, &Number) -> Number) -> Vec<Number> {
        vec![
            fun(&self.lower, &other.lower),
            fun(&self.lower, &other.upper),
            fun(&self.upper, &other.lower),
            fun(&self.upper, &other.upper),
        ]
    }

    pub fn idiv(&self, rhs: &Interval) -> Interval {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        if self.is_zero() {
            return Interval::ZERO;
        }
        if rhs.is_zero() {
            return Interval::NAN;
        }
        if self.is_singular() {
            let a = self.lower.idiv(&rhs.lower);
            let b = self.lower.idiv(&rhs.upper);
            return Interval::ordered(a, b);
        } else if rhs.is_singular() {
            let a = self.lower.idiv(&rhs.lower);
            let b = self.upper.idiv(&rhs.lower);
            return Interval::ordered(a, b);
        }
        let pairs = self.cartesian(rhs, |a, b| a.idiv(b));
        Interval {
            lower: pairs.iter().min().unwrap().clone(),
            upper: pairs.iter().max().unwrap().clone(),
        }
    }
}

impl Add for &Interval {
    type Output = Interval;

    fn add(self, rhs: Self) -> Self::Output {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        Interval {
            lower: &self.lower + &rhs.lower,
            upper: &self.upper + &rhs.upper,
        }
    }
}

impl Sub for &Interval {
    type Output = Interval;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        Interval {
            lower: &self.lower - &rhs.upper,
            upper: &self.upper - &rhs.lower,
        }
    }
}

impl Mul for &Interval {
    type Output = Interval;

    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        if self.is_zero() || rhs.is_zero() {
            return Interval::ZERO;
        }
        if self.is_singular() {
            let a = self.lower.mul(&rhs.lower);
            let b = self.lower.mul(&rhs.upper);
            return Interval::ordered(a, b);
        } else if rhs.is_singular() {
            let a = self.lower.mul(&rhs.lower);
            let b = self.upper.mul(&rhs.lower);
            return Interval::ordered(a, b);
        }
        let pairs = self.cartesian(rhs, |a, b| a * b);
        Interval {
            lower: pairs.iter().min().unwrap().clone(),
            upper: pairs.iter().max().unwrap().clone(),
        }
    }
}

impl Div for &Interval {
    type Output = Interval;

    fn div(self, rhs: Self) -> Self::Output {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        if self.is_zero() {
            return Interval::ZERO;
        }
        if rhs.is_zero() {
            return Interval::NAN;
        }
        if self.is_singular() {
            let a = self.lower.div(&rhs.lower);
            let b = self.lower.div(&rhs.upper);
            return Interval::ordered(a, b);
        } else if rhs.is_singular() {
            let a = self.lower.div(&rhs.lower);
            let b = self.upper.div(&rhs.lower);
            return Interval::ordered(a, b);
        }
        let pairs = self.cartesian(rhs, |a, b| a / b);
        Interval {
            lower: pairs.iter().min().unwrap().clone(),
            upper: pairs.iter().max().unwrap().clone(),
        }
    }
}

impl Rem for &Interval {
    type Output = Interval;

    /// Calculates the widest bounds for the reminders
    fn rem(self, rhs: Self) -> Self::Output {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        if self.is_zero() {
            return Interval::ZERO;
        }
        if rhs.is_zero() {
            return Interval::NAN;
        }
        if self.is_singular() {
            let a = self.lower.rem(&rhs.lower);
            let b = self.lower.rem(&rhs.upper);
            return Interval::ordered(a, b);
        }

        let value = Interval::ordered(self.lower.abs(), self.upper.abs());
        let modulus = Interval::ordered(rhs.lower.abs(), rhs.upper.abs());

        if &value.upper < &modulus.lower {
            // inside the bounds
            self.clone()
        } else {
            // reminder applied
            let lower = if self.lower.is_negative() {
                // the farthest we can get on the negative side
                let almost_bound = Number::Float(modulus.upper.neg().to_f64().next_up().into());
                self.lower.max(&almost_bound).clone()
            } else {
                // both values are positive
                Number::ZERO
            };
            let upper = if self.upper.is_negative() {
                // both values are negative
                Number::ZERO
            } else {
                // the farthest we can get on the positive side
                let almost_bound = Number::Float(modulus.upper.to_f64().next_down().into());
                self.upper.min(&almost_bound).clone()
            };
            Interval { lower, upper }
        }
    }
}

impl Pow<&Interval> for &Interval {
    type Output = Interval;

    fn pow(self, rhs: &Interval) -> Self::Output {
        if self.is_infinite() || rhs.is_infinite() {
            return Interval::INFINITY;
        }
        if self.is_nan() || rhs.is_nan() {
            return Interval::NAN;
        }
        if rhs.is_zero() {
            return Interval {
                lower: Number::Integer(BigInt::one()),
                upper: Number::Integer(BigInt::one()),
            };
        }
        if rhs.is_one() {
            return self.clone();
        }
        if self.is_singular() {
            let a = self.lower.pow(&rhs.lower);
            let b = self.lower.pow(&rhs.upper);
            return Interval::ordered(a, b);
        } else if rhs.is_singular() {
            return self.pow(&rhs.lower);
        }

        // "Interval Arithmetic Specification" by Chiriaev et al (1998)
        // "The Extended Real Interval System" by Walster (1970)
        if self.lower.is_positive() {
            // base is on the positive side
            let pairs = self.cartesian(rhs, |a, b| a.pow(b));
            Interval {
                lower: pairs.iter().min().unwrap().clone(),
                upper: pairs.iter().max().unwrap().clone(),
            }
        } else if self.upper.is_negative() {
            // base is on the negative side
            let pairs = self.neg().cartesian(rhs, |a, b| a.pow(b));
            let upper = pairs.iter().max().unwrap().clone();
            Interval {
                lower: upper.neg(),
                upper,
            }
        } else if !rhs.lower.is_negative() {
            // base interval crosses zero, the exponent is non-negative
            let lower = self
                .lower
                .neg()
                .pow(&rhs.lower)
                .neg()
                .min(&self.lower.neg().pow(&rhs.upper).neg())
                .clone();
            let upper = lower
                .neg()
                .max(&self.upper.pow(&rhs.lower))
                .max(&self.upper.pow(&rhs.upper))
                .clone();
            Interval { lower, upper }
        } else {
            // base can be anything, the exponent is negative
            // so it becomes 1/inf all the way to 1/-inf
            Interval {
                lower: Number::NEG_INFINITY,
                upper: Number::INFINITY,
            }
        }
    }
}

impl Pow<&Number> for &Interval {
    type Output = Interval;

    fn pow(self, rhs: &Number) -> Self::Output {
        // TODO: check if we are handling correctly situation of crossing zero in (-x~x)^y
        if self.upper.is_negative() || self.lower.is_positive() {
            let a = self.lower.pow(rhs);
            let b = self.upper.pow(rhs);
            Interval::ordered(a, b)
        } else if rhs.is_even() {
            let a = self.lower.pow(rhs);
            let b = self.upper.pow(rhs);
            Interval {
                lower: Number::ZERO,
                upper: a.max(&b).clone(),
            }
        } else {
            let a = self.lower.pow(rhs);
            let b = self.upper.pow(rhs);
            Interval::ordered(a, b)
        }
    }
}

impl Neg for &Interval {
    type Output = Interval;

    fn neg(self) -> Self::Output {
        Interval::ordered(self.lower.neg(), self.upper.neg())
    }
}

impl From<&Number> for Interval {
    fn from(value: &Number) -> Self {
        Interval {
            lower: value.clone(),
            upper: value.clone(),
        }
    }
}

impl TryFrom<(&Number, &Number)> for Interval {
    type Error = anyhow::Error;

    fn try_from(value: (&Number, &Number)) -> Result<Self, Self::Error> {
        Interval::checked(value.0, value.1)
    }
}

impl std::cmp::PartialOrd for &Interval {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if &self.upper < &other.lower {
            Some(std::cmp::Ordering::Less)
        } else if &self.lower > &other.upper {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

impl std::cmp::Ord for &Interval {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.lower, Op::Interval, self.upper)
    }
}

impl std::fmt::Debug for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} {:?}", self.lower, Op::Interval, self.upper)
    }
}
