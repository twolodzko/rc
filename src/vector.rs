use crate::{Algebra, expr::Method, number::Number};
use anyhow::Result;
use num::{BigInt, One};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Vector(pub Vec<Algebra>);

impl Vector {
    pub fn primitive(&self, method: Method) -> Result<Vector> {
        let vals = self
            .0
            .iter()
            .map(|v| v.primitive(method))
            .collect::<Result<Vec<Algebra>>>()?;
        Ok(Vector(vals))
    }

    pub fn map(&self, fun: impl Fn(&Algebra) -> Algebra) -> Vector {
        Vector(self.0.iter().map(fun).collect())
    }

    pub fn zip_map(&self, rhs: &Vector, fun: fn((&Algebra, &Algebra)) -> Algebra) -> Vector {
        Vector(self.zip(rhs).map(fun).collect())
    }

    pub fn zip<'a, 'b>(
        &'a self,
        rhs: &'b Vector,
    ) -> impl Iterator<Item = (&'a Algebra, &'b Algebra)> {
        let ord = self.len().cmp(&rhs.len());
        let lhs: Box<dyn Iterator<Item = &Algebra>> = if ord == std::cmp::Ordering::Less {
            Box::new(self.0.iter().cycle())
        } else {
            Box::new(self.0.iter())
        };
        let rhs: Box<dyn Iterator<Item = &Algebra>> = if ord == std::cmp::Ordering::Greater {
            Box::new(rhs.0.iter().cycle())
        } else {
            Box::new(rhs.0.iter())
        };
        std::iter::zip(lhs, rhs)
    }

    pub fn all(&self, fun: impl Fn(&Algebra) -> bool) -> bool {
        self.0.iter().all(fun)
    }

    pub fn dot(&self, rhs: &Vector) -> Algebra {
        self.zip(rhs)
            .map(|(a, b)| a * b)
            .reduce(|ref acc, ref e| acc + e)
            .unwrap_or(Algebra::Number(Number::ZERO))
    }

    pub fn min(&self) -> Algebra {
        self.0.iter().min().cloned().unwrap_or(Algebra::NAN)
    }

    pub fn max(&self) -> Algebra {
        self.0.iter().max().cloned().unwrap_or(Algebra::NAN)
    }

    pub fn sum(&self) -> Algebra {
        if self.is_empty() {
            return Algebra::Number(Number::ZERO);
        }
        let mut sum = Algebra::Number(Number::ZERO);
        for v in &self.0 {
            sum = &sum + v;
        }
        sum
    }

    pub fn prod(&self) -> Algebra {
        if self.is_empty() {
            return Algebra::Number(Number::Integer(BigInt::one()));
        }
        let mut prod = Algebra::Number(Number::Integer(BigInt::one()));
        for v in &self.0 {
            prod = &prod * v;
        }
        prod
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<Algebra>> for Vector {
    fn from(value: Vec<Algebra>) -> Self {
        Vector(value)
    }
}

impl std::fmt::Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vals = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "[{}]", vals)
    }
}

impl std::fmt::Debug for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vals = self
            .0
            .iter()
            .map(|v| format!("{:?}", v))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "[{}]", vals)
    }
}
