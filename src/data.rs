use crate::function::LispFunction;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) enum Value {
    Number(f64),
    String(String),
    Symbol(String),
    Bool(bool),
    Nil,
    Pair(Rc<Cons>),
    Function(Rc<dyn LispFunction>),
    LocalVar(usize, usize),
    Lambda {
        params_count: usize,
        body: Rc<Value>,
    },

    Closure {
        params_count: usize,
        body: Rc<Value>,
        captured: Vec<Vec<Rc<Value>>>,
    },

    TailCall {
        func: Rc<Value>,
        args: Vec<Rc<Value>>,
    },

    If {
        cond: Rc<Value>,
        then_branch: Rc<Value>,
        else_branch: Rc<Value>,
    },

    Memoized {
        body: Rc<Value>,

        cache: RefCell<HashMap<Vec<Rc<Value>>, Rc<Value>>>,
    },

    Error(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b || (a.is_nan() && b.is_nan()),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::LocalVar(d1, i1), Value::LocalVar(d2, i2)) => d1 == d2 && i1 == i2,
            (Value::Pair(a), Value::Pair(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Memoized { body: b1, .. }, Value::Memoized { body: b2, .. }) => {
                Rc::ptr_eq(b1, b2)
            }

            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Value {}

use std::hash::{Hash, Hasher};

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Value::Number(n) => n.to_bits().hash(state),
            Value::String(s) | Value::Symbol(s) => s.hash(state),
            Value::Bool(b) => b.hash(state),
            Value::Nil => {}
            Value::LocalVar(d, i) => {
                d.hash(state);
                i.hash(state);
            }
            Value::Pair(p) => std::ptr::hash(Rc::as_ptr(p), state),
            Value::Function(f) => std::ptr::hash(Rc::as_ptr(f), state),
            Value::Memoized { body, .. } => Rc::as_ptr(body).hash(state),
            Value::Error(e) => e.hash(state),

            Value::Lambda { body, .. } | Value::Closure { body, .. } => {
                Rc::as_ptr(body).hash(state);
            }
            Value::TailCall { func, .. } => {
                Rc::as_ptr(func).hash(state);
            }
            Value::If { cond, .. } => {
                Rc::as_ptr(cond).hash(state);
            }
        }
    }
}

impl Value {
    pub(crate) fn number(n: f64) -> Rc<Self> {
        Rc::new(Value::Number(n))
    }

    pub(crate) fn string(s: &str) -> Rc<Self> {
        Rc::new(Value::String(s.to_string()))
    }

    pub(crate) fn symbol(s: &str) -> Rc<Self> {
        Rc::new(Value::Symbol(s.to_string()))
    }

    pub(crate) fn bool(b: bool) -> Rc<Self> {
        Rc::new(Value::Bool(b))
    }

    pub(crate) fn nil() -> Rc<Self> {
        Rc::new(Value::Nil)
    }

    pub(crate) fn is_truthy(&self) -> bool {
        !matches!(self, Value::Bool(false) | Value::Nil)
    }

    pub(crate) fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub(crate) fn cons(car: Rc<Value>, cdr: Rc<Value>) -> Rc<Self> {
        Rc::new(Value::Pair(Cons::new(car, cdr)))
    }

    #[cfg(test)]
    pub(crate) fn list(values: &[Rc<Value>]) -> Rc<Self> {
        let mut result = Value::nil();

        for value in values.iter().rev() {
            result = Value::cons(Rc::clone(value), result);
        }

        result
    }

    pub(crate) fn is_pair(&self) -> bool {
        matches!(self, Value::Pair(_))
    }

    #[cfg(test)]
    pub(crate) fn is_list(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Pair(cons) => cons.is_proper_list(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Cons {
    pub(crate) car: Rc<Value>,
    pub(crate) cdr: Rc<Value>,
}

impl Cons {
    pub(crate) fn new(car: Rc<Value>, cdr: Rc<Value>) -> Rc<Self> {
        Rc::new(Cons { car, cdr })
    }

    pub(crate) fn is_proper_list(&self) -> bool {
        match &*self.cdr {
            Value::Nil => true,
            Value::Pair(next) => next.is_proper_list(),
            _ => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn to_vec(&self) -> Option<Vec<Rc<Value>>> {
        if !self.is_proper_list() {
            return None;
        }

        let mut result = Vec::new();
        result.push(Rc::clone(&self.car));

        let mut current = &self.cdr;
        while let Value::Pair(cons) = &**current {
            result.push(Rc::clone(&cons.car));
            current = &cons.cdr;
        }

        Some(result)
    }

    pub(crate) fn cars(&self) -> ConsCarsIter {
        ConsCarsIter {
            current: Some(self),
        }
    }
}

impl fmt::Display for Cons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_proper_list() {
            return self.fmt_as_list(f);
        }

        write!(f, "({} . {})", self.car, self.cdr)
    }
}

impl Cons {
    fn fmt_as_list(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        self.fmt_list_contents(f)?;
        write!(f, ")")
    }

    fn fmt_list_contents(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.car)?;

        match &*self.cdr {
            Value::Nil => Ok(()),
            Value::Pair(next) => {
                write!(f, " ")?;
                next.fmt_list_contents(f)
            }
            other => {
                write!(f, " . {}", other)
            }
        }
    }
}

pub(crate) struct ConsCarsIter<'a> {
    current: Option<&'a Cons>,
}

impl<'a> Iterator for ConsCarsIter<'a> {
    type Item = &'a Rc<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.take()?;
        let result = &current.car;

        self.current = match &*current.cdr {
            Value::Pair(next) => Some(next),
            _ => None,
        };

        Some(result)
    }
}

impl PartialEq for Cons {
    fn eq(&self, other: &Self) -> bool {
        self.car == other.car && self.cdr == other.cdr
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Bool(true) => write!(f, "#t"),
            Value::Bool(false) => write!(f, "#f"),
            Value::Nil => write!(f, "()"),
            Value::Pair(cons) => {
                if cons.is_proper_list() {
                    write!(f, "(")?;

                    let mut current = cons;
                    let mut first = true;

                    loop {
                        if !first {
                            write!(f, " ")?;
                        }
                        first = false;

                        write!(f, "{}", current.car)?;

                        match &*current.cdr {
                            Value::Nil => break,
                            Value::Pair(next) => current = next,
                            _ => {
                                write!(f, " . {}", current.cdr)?;
                                break;
                            }
                        }
                    }

                    write!(f, ")")
                } else {
                    write!(f, "({} . {})", cons.car, cons.cdr)
                }
            }
            Value::Function(_) => write!(f, "<function>"),
            Value::Error(msg) => write!(f, "<error: {}>", msg),
            Value::LocalVar(u1, u2) => write!(f, "<LocalVar: {} {}>", u1, u2),
            Value::Lambda { params_count, body } => {
                write!(f, "<Lambda params_count:{} body: {}>", params_count, body)
            }
            Value::Closure { .. } => write!(f, "<Clousure >"),
            Value::TailCall { func, args } => write!(f, "<TailCall {} {:?}>", func, args),
            Value::If {
                cond,
                then_branch,
                else_branch,
            } => write!(f, "<if {} {} {}>", cond, then_branch, else_branch),
            Value::Memoized { body, cache } => {
                write!(f, "<memorized {} {:?} >", body, cache)
            }
        }
    }
}

#[cfg(test)]
pub fn list_from_vec(values: Vec<Rc<Value>>) -> Rc<Value> {
    Value::list(&values)
}

#[cfg(test)]
pub fn assert_list_eq(actual: &Rc<Value>, expected: &[Rc<Value>]) {
    if let Value::Pair(cons) = &**actual {
        let items = cons.to_vec().expect("Expected proper list");

        assert_eq!(items.len(), expected.len());

        for (i, (actual_item, expected_item)) in items.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                actual_item, expected_item,
                "Mismatch at position {}: expected {:?}, got {:?}",
                i, expected_item, actual_item
            );
        }
    } else {
        panic!("Expected pair, got {:?}", actual);
    }
}

impl Value {
    fn is_pure_function(name: &str) -> bool {
        matches!(name, "+" | "-" | "*" | "/" | "=" | ">" | "<" | "not")
    }

    fn try_fold_constants(op: &str, args_value: Rc<Value>) -> Rc<Value> {
        let mut args = Vec::new();
        let mut current = args_value;

        while let Value::Pair(cons) = &*current {
            args.push(cons.car.clone().optimize());
            current = cons.cdr.clone();
        }

        let all_numbers = args.iter().all(|a| matches!(**a, Value::Number(_)));

        if !all_numbers && !args.is_empty() {
            return Self::rebuild_list(op, args);
        }

        let nums: Vec<f64> = args
            .iter()
            .map(|a| if let Value::Number(n) = **a { n } else { 0.0 })
            .collect();

        match op {
            "+" => Rc::new(Value::Number(nums.iter().sum())),
            "*" => Rc::new(Value::Number(nums.iter().product())),

            "-" => {
                if nums.is_empty() {
                    Rc::new(Value::Error(
                        "'-' requires at least one argument".to_string(),
                    ))
                } else if nums.len() == 1 {
                    Rc::new(Value::Number(-nums[0]))
                } else {
                    let res = nums[1..].iter().fold(nums[0], |acc, &x| acc - x);
                    Rc::new(Value::Number(res))
                }
            }

            "/" => {
                if nums.is_empty() {
                    Rc::new(Value::Error(
                        "'/' requires at least one argument".to_string(),
                    ))
                } else if nums.len() == 1 {
                    if nums[0] == 0.0 {
                        Rc::new(Value::Error("Division by zero".to_string()))
                    } else {
                        Rc::new(Value::Number(1.0 / nums[0]))
                    }
                } else {
                    let mut res = nums[0];
                    for &x in &nums[1..] {
                        if x == 0.0 {
                            return Rc::new(Value::Error("Division by zero".to_string()));
                        }
                        res /= x;
                    }
                    Rc::new(Value::Number(res))
                }
            }

            _ => Self::rebuild_list(op, args),
        }
    }

    fn rebuild_list(op: &str, args: Vec<Rc<Value>>) -> Rc<Value> {
        let mut res = Rc::new(Value::Nil);
        for arg in args.into_iter().rev() {
            res = Rc::new(Value::Pair(Rc::new(Cons { car: arg, cdr: res })));
        }
        Rc::new(Value::Pair(Rc::new(Cons {
            car: Rc::new(Value::Symbol(op.to_string())),
            cdr: res,
        })))
    }

    pub fn optimize(self: Rc<Self>) -> Rc<Self> {
        match &*self {
            Value::Pair(cons) => {
                let car = cons.car.clone().optimize();
                let cdr = cons.cdr.clone().optimize();

                if let Value::Symbol(s) = &*car {
                    if Self::is_pure_function(s) {
                        return Self::try_fold_constants(s, cdr);
                    }
                }

                Rc::new(Value::Pair(Rc::new(Cons { car, cdr })))
            }

            _ => self,
        }
    }
}
