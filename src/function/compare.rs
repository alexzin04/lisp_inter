use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use crate::{data::Value, enviroment::RuntimeEnv, function::NativeFunction};

fn is_null_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "null?",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("null? expects exactly 1 argument".to_string());
            }
            Ok(Value::bool(args[0].is_nil()))
        },
    ))
    .into()
}

fn is_pair_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "pair?",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("pair? expects exactly 1 argument".to_string());
            }
            Ok(Value::bool(args[0].is_pair()))
        },
    ))
    .into()
}

fn eq_question_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "eq?",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 2 {
                return Err("eq? expects exactly 2 arguments".to_string());
            }
            Ok(Value::bool(args[0] == args[1]))
        },
    ))
    .into()
}

fn equal_question_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "equal?",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 2 {
                return Err("equal? expects exactly 2 arguments".to_string());
            }
            Ok(Value::bool(args[0] == args[1]))
        },
    ))
    .into()
}

pub(super) fn add_compare_functions(env: &mut RefMut<'_, RuntimeEnv>) {
    env.define_global("null?", is_null_function());
    env.define_global("pair?", is_pair_function());
    env.define_global("eq?", eq_question_function());
    env.define_global("equal?", equal_question_function());
}
