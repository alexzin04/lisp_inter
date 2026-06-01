use super::eval::eval;
use crate::{data::Value, enviroment::RuntimeEnv, function::NativeFunction};
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

fn not_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "not",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("not expects exactly 1 argument".to_string());
            }
            Ok(Value::bool(!args[0].is_truthy()))
        },
    ))
    .into()
}

fn and_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "and",
        |args: &[Rc<Value>], env: &Rc<RefCell<RuntimeEnv>>| {
            for arg in args {
                let val = eval(arg.clone(), env)?;
                if !val.is_truthy() {
                    return Ok(val);
                }
            }
            Ok(if let Some(last) = args.last() {
                eval(last.clone(), env)?
            } else {
                Value::bool(true)
            })
        },
    ))
    .into()
}

fn or_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "or",
        |args: &[Rc<Value>], env: &Rc<RefCell<RuntimeEnv>>| {
            for arg in args {
                let val = eval(arg.clone(), env)?;
                if val.is_truthy() {
                    return Ok(val);
                }
            }
            Ok(Value::bool(false))
        },
    ))
    .into()
}

pub(super) fn add_logic_functions(env: &mut RefMut<'_, RuntimeEnv>) {
    env.define_global("not", not_function());
    env.define_global("and", and_function());
    env.define_global("or", or_function());
}
