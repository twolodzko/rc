#![allow(clippy::non_canonical_partial_ord_impl)]
// some types are not comparable, but that's ok
// we need to compare them only to order them in some cases
// but if we get the ordering wrong, it does not matter

mod algebra;
mod eval;
mod expr;
mod interval;
mod number;
mod parser;
#[cfg(test)]
mod tests;
mod vector;

pub(crate) use algebra::Algebra;
pub use eval::eval;
pub use parser::parse;

use crate::expr::Expr;
use anyhow::Result;
use expr::Function;
use std::{cell::RefCell, collections::HashMap, fs::File, io::Read, path::PathBuf, rc::Rc};

/// The number of digits after the decimal point to print
pub static mut SCALE: Option<usize> = None;
pub static mut PRINT_AS_FLOAT: bool = false;

pub type Memory = Rc<RefCell<HashMap<String, Algebra>>>;
pub type Functions = Rc<RefCell<HashMap<String, Function>>>;

pub fn init() -> (Memory, Functions) {
    use number::Number::*;
    let memory: Memory = Default::default();
    {
        let mut m = memory.borrow_mut();
        m.insert(
            "e".to_string(),
            Algebra::Scalar(Float(std::f64::consts::E.into())),
        );
        m.insert(
            "pi".to_string(),
            Algebra::Scalar(Float(std::f64::consts::PI.into())),
        );
        m.insert(
            "epsilon".to_string(),
            Algebra::Scalar(Float(f64::EPSILON.into())),
        );
        m.insert(
            "i".to_string(),
            Algebra::Scalar(Complex(num::complex::Complex::I)),
        );
    }
    let funs: Functions = Default::default();
    (memory, funs)
}

fn eval_main(exprs: &[Expr], memory: Memory, funs: Functions) -> Result<Algebra> {
    let mut last = Algebra::NAN;
    for expr in exprs {
        last = eval(expr, memory.clone(), funs.clone())?;
        memory.borrow_mut().insert("_".to_string(), last.clone());
    }
    Ok(last)
}

pub fn eval_string(script: &str, memory: Memory, funs: Functions) -> Result<Algebra> {
    let exprs = parse(script)?;
    eval_main(&exprs, memory.clone(), funs)
}

pub fn eval_file(path: &PathBuf, memory: Memory, funs: Functions) -> Result<Algebra> {
    let script = read_file(path)?;
    eval_string(&script, memory, funs)
}

fn read_file(path: &PathBuf) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

#[derive(Debug)]
pub struct ArityError {
    name: String,
    arity: usize,
    count: usize,
}

impl std::fmt::Display for ArityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let plural = if self.arity != 1 { "s" } else { "" };
        write!(
            f,
            "{} expected {} argument{}, got {}",
            self.name, self.arity, plural, self.count,
        )
    }
}

#[derive(Debug)]
pub struct AssertionError(Expr);

impl std::fmt::Display for AssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assertion {} failed", self.0)
    }
}
