use crate::{
    Algebra, ArityError, AssertionError, Functions, Memory, Template, eval_file,
    expr::{Expr, Function, Op},
    interval::Interval,
    number::Number,
    vector,
};
use anyhow::{Result, anyhow, bail};
use num::{BigInt, One};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub fn eval(expr: &Expr, mut memory: Memory, funs: Functions) -> Result<Algebra> {
    use Expr::*;
    let mut expr = expr.clone();
    loop {
        match expr {
            // values & variables
            Value(n) => return Ok(n),
            NewVec(ref exprs) => {
                let vals = eval_all(exprs, memory, funs)?;
                return Ok(Algebra::Vector(vals.into()));
            }
            Variable(ref n) => {
                return memory
                    .borrow()
                    .get(n)
                    .cloned()
                    .ok_or(anyhow!("uninitialized variable {}", n));
            }
            Primitive(m, ref e) => {
                let val = eval(e, memory, funs)?;
                return val.primitive(m);
            }
            // operations
            BinaryOp {
                ref lhs,
                op: Op::In,
                ref rhs,
            } => {
                let lhs = eval(lhs, memory.clone(), funs.clone())?;
                let rhs = eval(rhs, memory, funs)?;
                let ok = match rhs {
                    Algebra::Interval(rhs) => {
                        let Algebra::Number(ref lhs) = lhs else {
                            bail!("{} is not a number", lhs)
                        };
                        rhs.contains(lhs)
                    }
                    Algebra::Vector(rhs) => rhs.0.contains(&lhs),
                    _ => bail!("{} is not an interval or vector", rhs),
                };
                if ok {
                    return Ok(lhs.clone());
                } else {
                    bail!(AssertionError(expr));
                }
            }
            BinaryOp {
                ref lhs,
                op: Op::And,
                ref rhs,
            } => {
                // early stop
                let val = eval(lhs, memory.clone(), funs.clone())?;
                if val.is_nan() {
                    return Ok(val);
                } else {
                    // tail-call optimization
                    expr = *rhs.clone();
                };
            }
            BinaryOp {
                ref lhs,
                op: Op::Or,
                ref rhs,
            } => {
                // ignore failure
                match eval(lhs, memory.clone(), funs.clone()) {
                    Ok(val) if !val.is_nan() => return Ok(val),
                    Err(err) if !err.is::<AssertionError>() => bail!(err),
                    // tail-call optimization
                    _ => expr = *rhs.clone(),
                }
            }
            BinaryOp {
                ref lhs,
                op: Op::Eq,
                ref rhs,
            } => {
                if let Expr::Variable(k) = lhs.as_ref()
                    && !memory.borrow().contains_key(k)
                {
                    let val = eval(rhs, memory.clone(), funs)?;
                    memory.borrow_mut().insert(k.to_string(), val.clone());
                    return Ok(val);
                }
                if let Expr::Variable(k) = rhs.as_ref()
                    && !memory.borrow().contains_key(k)
                {
                    let val = eval(lhs, memory.clone(), funs)?;
                    memory.borrow_mut().insert(k.to_string(), val.clone());
                    return Ok(val);
                }

                let lhs = eval(lhs, memory.clone(), funs.clone())?;
                let rhs = eval(rhs, memory, funs)?;
                if lhs.is_nan() {
                    return Ok(lhs);
                }
                if rhs.is_nan() {
                    return Ok(rhs);
                }
                if lhs == rhs {
                    return Ok(rhs);
                } else {
                    bail!(AssertionError(expr))
                }
            }
            BinaryOp {
                ref lhs,
                op: op @ (Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge),
                ref rhs,
            } => {
                let lhs = eval(lhs, memory.clone(), funs.clone())?;
                let rhs = eval(rhs, memory, funs)?;
                if lhs.is_nan() {
                    return Ok(lhs);
                }
                if rhs.is_nan() {
                    return Ok(rhs);
                }
                return if lhs.compare(op, &rhs) {
                    Ok(rhs)
                } else {
                    bail!(AssertionError(expr))
                };
            }
            BinaryOp {
                ref lhs,
                op: Op::EqType,
                ref rhs,
            } => {
                let lhs = eval(lhs, memory.clone(), funs.clone())?;
                let rhs = eval(rhs, memory, funs)?;
                return if lhs.equal_type(&rhs) {
                    Ok(rhs)
                } else {
                    bail!(AssertionError(expr))
                };
            }
            BinaryOp {
                ref lhs,
                op: Op::Interval,
                ref rhs,
            } => {
                let lhs = &eval(lhs, memory.clone(), funs.clone())?;
                let rhs = &eval(rhs, memory, funs)?;
                let Algebra::Number(lhs) = lhs else {
                    bail!("only intervals of numbers are defined")
                };
                let Algebra::Number(rhs) = rhs else {
                    bail!("only intervals of numbers are defined")
                };
                return Ok(Algebra::Interval(Interval::checked(lhs, rhs)?));
            }
            BinaryOp {
                ref lhs,
                op: op @ Op::Dot,
                ref rhs,
            } => {
                let lhs = &eval(lhs, memory.clone(), funs.clone())?;
                let rhs = &eval(rhs, memory, funs)?;
                match (&lhs, &rhs) {
                    (Algebra::Vector(a), Algebra::Vector(b)) => {
                        return Ok(a.dot(b));
                    }
                    _ => bail!("{} can be applied only to vectors", op),
                }
            }
            BinaryOp {
                ref lhs,
                op: Op::Get,
                ref rhs,
            } => {
                if let Algebra::Vector(vec) = &eval(lhs, memory.clone(), funs.clone())? {
                    let index = &eval(rhs, memory, funs)?;
                    return extract(vec, index);
                }
                bail!("{} cannot be indexed", lhs)
            }
            BinaryOp {
                ref lhs,
                op,
                ref rhs,
            } => {
                let lhs = eval(lhs, memory.clone(), funs.clone())?;
                let rhs = eval(rhs, memory, funs)?;
                return Ok(lhs.op(op, &rhs));
            }
            // primitives
            Apply(ref name, ref exprs) if name == "rand" => {
                fn random() -> Algebra {
                    let r = rand::random_range(0.0..1.0);
                    Algebra::Number(Number::Float(r.into()))
                }
                match exprs.len() {
                    0 => return Ok(random()),
                    1 => {
                        let val = eval(&exprs[0], memory, funs)?;
                        if let Algebra::Number(ref n) = val
                            && let Number::Integer(n) = n
                        {
                            let vals: Vec<Algebra> = std::iter::from_fn(|| Some(random()))
                                .take(n.try_into()?)
                                .collect();
                            return Ok(Algebra::Vector(vals.into()));
                        }
                        bail!("{} is not a number", val)
                    }
                    _ => {
                        bail!(ArityError {
                            name: name.to_string(),
                            arity: 1,
                            count: exprs.len()
                        })
                    }
                }
            }
            Apply(ref name, ref exprs) if name == "choose" => {
                if exprs.len() != 2 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 2,
                        count: exprs.len()
                    })
                }
                let n = eval(&exprs[0], memory.clone(), funs.clone())?;
                let k = eval(&exprs[1], memory, funs)?;
                return Ok(n.choose(&k));
            }
            Apply(ref name, ref exprs) if name == "min" => {
                if exprs.len() != 1 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 1,
                        count: exprs.len()
                    })
                }
                let val = match eval(&exprs[0], memory, funs)? {
                    Algebra::Vector(ref v) => v.min(),
                    Algebra::Interval(ref a) => Algebra::Number(a.lower.clone()),
                    other => other,
                };
                return Ok(val);
            }
            Apply(ref name, ref exprs) if name == "max" => {
                if exprs.len() != 1 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 1,
                        count: exprs.len()
                    })
                }
                let val = match eval(&exprs[0], memory, funs)? {
                    Algebra::Vector(ref v) => v.max(),
                    Algebra::Interval(ref a) => Algebra::Number(a.upper.clone()),
                    other => other,
                };
                return Ok(val);
            }
            Apply(ref name, ref exprs) if name == "sum" => {
                return vec_apply(name, exprs, memory, funs, |v| v.sum());
            }
            Apply(ref name, ref exprs) if name == "prod" => {
                return vec_apply(name, exprs, memory, funs, |v| v.prod());
            }
            Apply(ref name, ref exprs) if name == "len" => {
                return Ok(Algebra::Number(vec_apply(
                    name,
                    exprs,
                    memory,
                    funs,
                    |v| v.len().into(),
                )?));
            }
            Apply(ref name, ref exprs) if name == "rev" => {
                return Ok(Algebra::Vector(vec_apply(
                    name,
                    exprs,
                    memory,
                    funs,
                    |v| v.0.iter().cloned().rev().collect::<Vec<_>>().into(),
                )?));
            }
            Apply(ref name, ref exprs) if name == "push" => {
                if exprs.len() < 2 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 1,
                        count: exprs.len()
                    })
                }
                match eval(&exprs[0], memory.clone(), funs.clone())? {
                    Algebra::Vector(v) => {
                        let mut v = v;
                        for x in eval_all(&exprs[1..], memory.clone(), funs.clone())? {
                            v.0.push(x)
                        }
                        return Ok(Algebra::Vector(v));
                    }
                    other => bail!("{} is not a vector", other),
                }
            }
            Apply(ref name, ref exprs) if name == "seq" => {
                let step = match exprs.len() {
                    2 => Number::Integer(BigInt::one()),
                    3 => eval_to_number(&exprs[2], memory.clone(), funs.clone())?,
                    _ => {
                        bail!("{} expected 2 or 3 arguments, got {}", name, exprs.len())
                    }
                };
                let mut this = eval_to_number(&exprs[0], memory.clone(), funs.clone())?;
                let end = eval_to_number(&exprs[1], memory, funs)?;
                if this.is_nan()
                    || end.is_nan()
                    || step.is_nan()
                    || step.is_zero()
                    || (&this < &end && step.is_negative())
                    || (&this > &end && step.is_positive())
                {
                    bail!(
                        "invalid {} arguments: from {} to {} by {}",
                        name,
                        this,
                        end,
                        step
                    )
                }
                let mut acc = Vec::new();
                while &this <= &end {
                    acc.push(Algebra::Number(this.clone()));
                    this = &this + &step;
                }
                return Ok(Algebra::Vector(acc.into()));
            }
            Apply(ref name, ref exprs) if name == "int" => {
                if exprs.len() != 1 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 1,
                        count: exprs.len()
                    })
                }
                let val = eval(&exprs[0], memory, funs)?;
                return Ok(val.map(|x| x.to_bigint().map(Number::Integer).unwrap_or(Number::NAN)));
            }
            Apply(ref name, ref exprs) if name == "float" => {
                if exprs.len() != 1 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 1,
                        count: exprs.len()
                    })
                }
                let val = eval(&exprs[0], memory, funs)?;
                return Ok(val.map(|x| x.to_f64().map(|f| f.into()).unwrap_or(Number::NAN)));
            }
            Apply(ref name, ref exprs) if name == "rat" => {
                if exprs.len() != 1 {
                    bail!(ArityError {
                        name: name.to_string(),
                        arity: 1,
                        count: exprs.len()
                    })
                }
                let val = eval(&exprs[0], memory, funs)?;
                return Ok(val.map(|x| x.rat()));
            }
            // tail-call optimized
            Block(ref exprs) => {
                let last = exprs.len() - 1;
                eval_all(&exprs[..last], memory.clone(), funs.clone())?;
                expr = exprs[last].clone();
            }
            Apply(ref name, ref exprs) => {
                let Some(func) = funs.borrow().get(name).cloned() else {
                    bail!("unknown function: {}", name)
                };
                let args = eval_all(exprs, memory, funs.clone())?;
                (expr, memory) = func.call(&args, funs.clone())?;
            }
            IfElse(cond, yes, no) => match eval(&cond, memory.clone(), funs.clone()) {
                Ok(val) if !val.is_nan() => expr = *yes,
                Ok(_) => expr = *no,
                Err(err) if err.is::<AssertionError>() => expr = *no,
                Err(err) => bail!(err),
            },
            // special
            Function(func) => {
                funs.borrow_mut()
                    .insert(func.name.to_string(), func.clone());
                return Ok(Algebra::NAN);
            }
            Print(ref template) => {
                let (msg, last) = eval_template(template, memory, funs)?;
                print!("{}", msg);
                return Ok(last);
            }
            Error(ref template) => {
                let (msg, _) = eval_template(template, memory, funs)?;
                bail!(msg)
            }
            Load(ref path) => return eval_file(path, memory, funs),
        }
    }
}

impl Function {
    pub(crate) fn call(self, args: &[Algebra], funs: Functions) -> Result<(Expr, Memory)> {
        if args.len() != self.args.len() {
            bail!(ArityError {
                name: self.name,
                arity: self.args.len(),
                count: args.len()
            })
        }
        let mut local = HashMap::new();
        for (key, val) in std::iter::zip(self.args.iter(), args) {
            local.insert(key.to_string(), val.clone());
        }
        let local = Rc::new(RefCell::new(local));
        // tail-call optimization
        if self.body.len() == 1 {
            return Ok((self.body[0].clone(), local));
        }
        let last = self.body.len() - 1;
        eval_keep_state(&self.body[..last], local.clone(), funs)?;
        Ok((self.body[last].clone(), local))
    }
}

/// Evaluate the expressions, save result of each expression saved to the `_` variable
pub fn eval_keep_state(exprs: &[Expr], memory: Memory, funs: Functions) -> Result<Algebra> {
    let mut last = Algebra::NAN;
    for expr in exprs {
        last = eval(expr, memory.clone(), funs.clone())?;
        memory.borrow_mut().insert("_".to_string(), last.clone());
    }
    Ok(last)
}

fn eval_all(exprs: &[Expr], memory: Memory, funs: Functions) -> Result<Vec<Algebra>> {
    exprs
        .iter()
        .map(|e| eval(e, memory.clone(), funs.clone()))
        .collect()
}

fn vec_apply<T>(
    name: &str,
    exprs: &[Expr],
    memory: Memory,
    funs: Functions,
    fun: fn(&vector::Vector) -> T,
) -> Result<T> {
    if exprs.len() != 1 {
        bail!(ArityError {
            name: name.to_string(),
            arity: 1,
            count: exprs.len()
        })
    }
    let val = eval(&exprs[0], memory, funs)?;
    if let Algebra::Vector(ref v) = val {
        Ok(fun(v))
    } else {
        bail!("{} is not a vector", val)
    }
}

fn eval_to_number(expr: &Expr, memory: Memory, funs: Functions) -> Result<Number> {
    let val = eval(expr, memory, funs)?;
    if let Algebra::Number(val) = val {
        Ok(val)
    } else {
        bail!("{} is not a number", val)
    }
}

fn extract(vector: &vector::Vector, index: &Algebra) -> Result<Algebra> {
    use Algebra::*;
    match index {
        Number(index) => {
            if let Some(index) = index.to_usize()
                && index >= 1
            {
                return Ok(vector.0.get(index - 1).cloned().unwrap_or(Algebra::NAN));
            }
        }
        Interval(range) => {
            if let Some(lower) = range.lower.to_usize()
                && let Some(upper) = range.upper.to_usize()
                && lower >= 1
            {
                let vec = vector.0[lower - 1..upper.min(vector.len())].to_vec();
                return Ok(Vector(vec.into()));
            }
        }
        Vector(indexes) => {
            let mut acc = Vec::new();
            for i in &indexes.0 {
                if let Number(i) = i
                    && let Some(i) = i.to_usize()
                    && i >= 1
                {
                    let val = vector.0.get(i - 1).cloned().unwrap_or(Algebra::NAN);
                    acc.push(val);
                } else {
                    bail!("{} is not a valid index", index)
                }
            }
            return Ok(Vector(acc.into()));
        }
    }
    bail!("{} is not a valid index", index)
}

fn eval_template(
    template: &[Template],
    memory: Memory,
    funs: Functions,
) -> Result<(String, Algebra)> {
    let mut msg = String::new();
    let mut last = Algebra::NAN;
    for t in template {
        let s = match t {
            Template::String(s) => s,
            Template::Field(e) => {
                last = eval(e, memory.clone(), funs.clone())?;
                &last.to_string()
            }
        };
        msg.push_str(s);
    }
    Ok((msg, last))
}
