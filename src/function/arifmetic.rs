use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use crate::{data::Value, enviroment::RuntimeEnv, function::NativeFunction};

fn add_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "+",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.is_empty() {
                return Ok(Value::number(0.0));
            }

            let mut sum = 0.0;
            for arg in args {
                if let Value::Number(n) = &**arg {
                    sum += n;
                } else {
                    return Err(format!("+ expects numbers, got {}", arg));
                }
            }
            Ok(Value::number(sum))
        },
    ))
    .into()
}

fn sub_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "-",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| match args.len() {
            0 => Err("- expects at least 1 argument".to_string()),
            1 => {
                if let Value::Number(n) = &*args[0] {
                    Ok(Value::number(-n))
                } else {
                    Err(format!("- expects number, got {}", args[0]))
                }
            }
            _ => {
                let mut result = if let Value::Number(n) = &*args[0] {
                    *n
                } else {
                    return Err(format!("- expects numbers, got {}", args[0]));
                };

                for arg in &args[1..] {
                    if let Value::Number(n) = &**arg {
                        result -= n;
                    } else {
                        return Err(format!("- expects numbers, got {}", arg));
                    }
                }
                Ok(Value::number(result))
            }
        },
    ))
    .into()
}

fn mul_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "*",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.is_empty() {
                return Ok(Value::number(1.0));
            }

            let mut product = 1.0;
            for arg in args {
                if let Value::Number(n) = &**arg {
                    product *= n;
                } else {
                    return Err(format!("* expects numbers, got {}", arg));
                }
            }
            Ok(Value::number(product))
        },
    ))
    .into()
}

fn div_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "/",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| match args.len() {
            0 => Err("/ expects at least 1 argument".to_string()),
            1 => {
                if let Value::Number(n) = &*args[0] {
                    if *n == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    Ok(Value::number(1.0 / n))
                } else {
                    Err(format!("/ expects number, got {}", args[0]))
                }
            }
            _ => {
                let mut result = if let Value::Number(n) = &*args[0] {
                    *n
                } else {
                    return Err(format!("/ expects numbers, got {}", args[0]));
                };

                for arg in &args[1..] {
                    if let Value::Number(n) = &**arg {
                        if *n == 0.0 {
                            return Err("Division by zero".to_string());
                        }
                        result /= n;
                    } else {
                        return Err(format!("/ expects numbers, got {}", arg));
                    }
                }
                Ok(Value::number(result))
            }
        },
    ))
    .into()
}

fn eq_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "=",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() < 2 {
                return Err("= expects at least 2 arguments".to_string());
            }

            let first = &args[0];
            for arg in &args[1..] {
                match (&**first, &**arg) {
                    (Value::Number(a), Value::Number(b)) if a == b => continue,
                    _ => return Ok(Value::bool(false)),
                }
            }
            Ok(Value::bool(true))
        },
    ))
    .into()
}

fn lt_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "<",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() < 2 {
                return Err("< expects at least 2 arguments".to_string());
            }

            for window in args.windows(2) {
                match (&*window[0], &*window[1]) {
                    (Value::Number(a), Value::Number(b)) if a < b => continue,
                    _ => return Ok(Value::bool(false)),
                }
            }
            Ok(Value::bool(true))
        },
    ))
    .into()
}

fn gt_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        ">",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() < 2 {
                return Err("> expects at least 2 arguments".to_string());
            }

            for window in args.windows(2) {
                match (&*window[0], &*window[1]) {
                    (Value::Number(a), Value::Number(b)) if a > b => continue,
                    _ => return Ok(Value::bool(false)),
                }
            }
            Ok(Value::bool(true))
        },
    ))
    .into()
}

fn le_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "<=",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() < 2 {
                return Err("<= expects at least 2 arguments".to_string());
            }

            for window in args.windows(2) {
                match (&*window[0], &*window[1]) {
                    (Value::Number(a), Value::Number(b)) if a <= b => continue,
                    _ => return Ok(Value::bool(false)),
                }
            }
            Ok(Value::bool(true))
        },
    ))
    .into()
}

fn ge_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        ">=",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() < 2 {
                return Err(">= expects at least 2 arguments".to_string());
            }

            for window in args.windows(2) {
                match (&*window[0], &*window[1]) {
                    (Value::Number(a), Value::Number(b)) if a >= b => continue,
                    _ => return Ok(Value::bool(false)),
                }
            }
            Ok(Value::bool(true))
        },
    ))
    .into()
}

pub(super) fn add_arifmetic_functions(env: &mut RefMut<'_, RuntimeEnv>) {
    env.define_global("+", add_function());
    env.define_global("-", sub_function());
    env.define_global("*", mul_function());
    env.define_global("/", div_function());
    env.define_global("=", eq_function());
    env.define_global("<", lt_function());
    env.define_global(">", gt_function());
    env.define_global("<=", le_function());
    env.define_global(">=", ge_function());
}
