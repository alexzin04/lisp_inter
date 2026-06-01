use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use crate::{data::Value, enviroment::RuntimeEnv, function::NativeFunction};

fn car_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "car",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("car expects exactly 1 argument".to_string());
            }

            match &*args[0] {
                Value::Pair(cons) => Ok(Rc::clone(&cons.car)),
                Value::Nil => Err("car: cannot take car of empty list".to_string()),
                _ => Err(format!("car: expected pair, got {}", args[0])),
            }
        },
    ))
    .into()
}

fn cdr_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "cdr",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("cdr expects exactly 1 argument".to_string());
            }

            match &*args[0] {
                Value::Pair(cons) => Ok(Rc::clone(&cons.cdr)),
                Value::Nil => Err("cdr: cannot take cdr of empty list".to_string()),
                _ => Err(format!("cdr: expected pair, got {}", args[0])),
            }
        },
    ))
    .into()
}

fn cons_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "cons",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 2 {
                return Err("cons expects exactly 2 arguments".to_string());
            }
            Ok(Value::cons(Rc::clone(&args[0]), Rc::clone(&args[1])))
        },
    ))
    .into()
}

fn list_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "list",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            let mut result = Value::nil();
            for arg in args.iter().rev() {
                result = Value::cons(Rc::clone(arg), result);
            }
            Ok(result)
        },
    ))
    .into()
}

pub(super) fn add_list_functions(env: &mut RefMut<'_, RuntimeEnv>) {
    env.define_global("car", car_function());
    env.define_global("cdr", cdr_function());
    env.define_global("cons", cons_function());
    env.define_global("list", list_function());
}
