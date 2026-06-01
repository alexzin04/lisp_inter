use std::{cell::RefCell, rc::Rc};

use crate::{
    data::{Cons, Value},
    enviroment::RuntimeEnv,
    function::NativeFunction,
};

pub(super) fn eval_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "eval",
        |args: &[Rc<Value>], env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("eval expects exactly 1 argument".to_string());
            }

            eval(args[0].clone(), env)
        },
    ))
    .into()
}

pub(crate) fn eval(
    mut current_expr: Rc<Value>,
    env: &Rc<RefCell<RuntimeEnv>>,
) -> Result<Rc<Value>, String> {
    loop {
        let next_step = match current_expr.as_ref() {
            Value::Number(_) | Value::String(_) | Value::Bool(_) | Value::Nil => {
                return Ok(current_expr);
            }

            Value::LocalVar(depth, index) => {
                return env.borrow().lookup_local(*depth, *index).ok_or_else(|| {
                    format!(
                        "Runtime Error: Local var not found at d:{}, i:{}",
                        depth, index
                    )
                });
            }

            Value::Symbol(sym) => {
                return env
                    .borrow()
                    .lookup_global(sym)
                    .ok_or_else(|| format!("Undefined global symbol: {}", sym));
            }

            Value::Lambda { params_count, body } => {
                let captured = env.borrow().get_locals_clone();
                return Ok(Rc::new(Value::Closure {
                    params_count: *params_count,
                    body: Rc::clone(body),
                    captured,
                }));
            }

            Value::TailCall { func, args } => {
                env.borrow().profiler.borrow_mut().tail_calls_optimized += 1;

                let f_val = eval(func.clone(), env)?;

                let mut eval_args = Vec::with_capacity(args.len());
                for arg in args {
                    eval_args.push(eval(arg.clone(), env)?);
                }

                match f_val.as_ref() {
                    Value::Closure {
                        params_count,
                        body,
                        captured,
                    } => {
                        if eval_args.len() != *params_count {
                            return Err(format!(
                                "Expected {} args, got {}",
                                params_count,
                                eval_args.len()
                            ));
                        }

                        let mut new_locals = captured.clone();
                        new_locals.push(eval_args);

                        Some((body.clone(), new_locals))
                    }
                    Value::Function(f) => {
                        return f.call(&eval_args, env);
                    }
                    _ => return Err(format!("Object is not callable: {:?}", f_val)),
                }
            }

            Value::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let condition_result = eval(cond.clone(), env)?;

                let is_truthy = match condition_result.as_ref() {
                    Value::Nil => false,
                    Value::Bool(b) => *b,
                    _ => true,
                };

                if is_truthy {
                    current_expr = then_branch.clone();
                } else {
                    current_expr = else_branch.clone();
                }

                continue;
            }

            Value::Memoized { body, cache } => {
                let args_key = env.borrow().get_current_frame_clone();

                if let Some(hit) = cache.borrow().get(&args_key) {
                    env.borrow().profiler.borrow_mut().memo_cache_hits += 1;
                    return Ok(hit.clone());
                }
                env.borrow().profiler.borrow_mut().memo_cache_misses += 1;

                let args = env.borrow().get_current_frame_clone();

                {
                    let cache_borrow = cache.borrow();
                    if let Some(hit) = cache_borrow.get(&args) {
                        return Ok(hit.clone());
                    }
                }

                let result = eval(body.clone(), env)?;

                cache.borrow_mut().insert(args, result.clone());

                return Ok(result);
            }

            Value::Pair(cons) => return eval_list(cons, env),
            Value::Function(_) | Value::Closure { .. } => return Ok(current_expr),
            Value::Error(msg) => return Err(msg.clone()),
        };

        if let Some((next_body, next_locals)) = next_step {
            env.borrow_mut().set_locals(next_locals);
            current_expr = next_body;

            continue;
        } else {
            return Ok(current_expr);
        }
    }
}

fn eval_list(cons: &Cons, env: &Rc<RefCell<RuntimeEnv>>) -> Result<Rc<Value>, String> {
    if let Value::Symbol(sym) = &*cons.car {
        if sym == "define" {
            let mut raw_args = Vec::new();
            let mut current = &cons.cdr;
            while let Value::Pair(c) = current.as_ref() {
                raw_args.push(Rc::clone(&c.car));
                current = &c.cdr;
            }

            let define_fn = env
                .borrow()
                .lookup_global("define")
                .ok_or("Global 'define' not found")?;

            if let Value::Function(f) = &*define_fn {
                return f.call(&raw_args, env);
            }
        }
        if sym == "quote" {
            let define_fn = env
                .borrow()
                .lookup_global("quote")
                .ok_or("Global 'quote' not found")?;

            if let Value::Function(f) = &*define_fn {
                return f.call(&[cons.cdr.clone()], env);
            }
        }
    }

    let func_val = eval(cons.car.clone(), env)?;

    let mut evaluated_args = Vec::new();
    let mut current = &cons.cdr;
    while let Value::Pair(c) = current.as_ref() {
        evaluated_args.push(eval(c.car.clone(), env)?);
        current = &c.cdr;
    }

    match func_val.as_ref() {
        Value::Function(f) => f.call(&evaluated_args, env),

        Value::Closure {
            params_count,
            body,
            captured,
        } => {
            if evaluated_args.len() != *params_count {
                return Err(format!(
                    "Expected {} args, got {}",
                    params_count,
                    evaluated_args.len()
                ));
            }

            let original_locals = {
                let mut env_mut = env.borrow_mut();
                let old = env_mut.get_locals_clone();
                let mut new_locals = captured.clone();
                new_locals.push(evaluated_args);
                env_mut.set_locals(new_locals);
                old
            };

            let result = eval(body.clone(), env);

            env.borrow_mut().set_locals(original_locals);
            result
        }

        _ => Err(format!("Object is not callable: {:?}", func_val)),
    }
}

pub(crate) fn quote_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "quote",
        |args: &[Rc<Value>], _env: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() != 1 {
                return Err("quote expects exactly 1 argument".to_string());
            }
            Ok(args[0].clone())
        },
    ))
    .into()
}

#[cfg(test)]
mod test {

    use crate::{
        data::{assert_list_eq, list_from_vec, Value},
        enviroment::RuntimeEnv,
        function::{
            eval::{eval, quote_function},
            NativeFunction,
        },
    };

    #[test]
    fn test_eval_basic() {
        let env = RuntimeEnv::new_global();

        let num = Value::number(42.0);
        let result = eval(num, &env).unwrap();
        assert_eq!(result, Value::number(42.0));

        let str_val = Value::string("hello");
        let result = eval(str_val, &env).unwrap();
        assert_eq!(result, Value::string("hello"));

        let bool_true = Value::bool(true);
        let result = eval(bool_true, &env).unwrap();
        assert_eq!(result, Value::bool(true));

        let nil = Value::nil();
        let result = eval(nil, &env).unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_eval_symbol() {
        let env = RuntimeEnv::new_global();

        env.borrow_mut().define_global("x", Value::number(10.0));

        let sym = Value::symbol("x");
        let result = eval(sym, &env).unwrap();
        assert_eq!(result, Value::number(10.0));

        let undefined = Value::symbol("undefined");
        let result = eval(undefined, &env);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Undefined symbol"));
    }

    #[test]
    fn test_eval_arithmetic() {
        use std::rc::Rc;
        let env = RuntimeEnv::new_global();

        env.borrow_mut().define_global(
            "+",
            Rc::new(Value::Function(NativeFunction::new("+", |args, _env| {
                let mut sum = 0.0;
                for arg in args {
                    if let Value::Number(n) = &**arg {
                        sum += n;
                    } else {
                        return Err("+ expects numbers".to_string());
                    }
                }
                Ok(Value::number(sum))
            }))),
        );

        env.borrow_mut().define_global(
            "*",
            Rc::new(Value::Function(NativeFunction::new("*", |args, _env| {
                let mut product = 1.0;
                for arg in args {
                    if let Value::Number(n) = &**arg {
                        product *= n;
                    } else {
                        return Err("* expects numbers".to_string());
                    }
                }
                Ok(Value::number(product))
            }))),
        );

        let expr = list_from_vec(vec![
            Value::symbol("+"),
            Value::number(1.0),
            Value::number(2.0),
            Value::number(3.0),
        ]);

        let result = eval(expr, &env).unwrap();
        assert_eq!(result, Value::number(6.0));

        let expr2 = list_from_vec(vec![
            Value::symbol("*"),
            Value::number(2.0),
            Value::number(3.0),
            Value::number(4.0),
        ]);

        let result2 = eval(expr2, &env).unwrap();
        assert_eq!(result2, Value::number(24.0));
    }

    #[test]
    fn test_eval_nested() {
        use std::rc::Rc;
        let env = RuntimeEnv::new_global();

        env.borrow_mut().define_global(
            "+",
            Rc::new(Value::Function(NativeFunction::new("+", |args, _env| {
                let mut sum = 0.0;
                for arg in args {
                    if let Value::Number(n) = &**arg {
                        sum += n;
                    } else {
                        return Err("+ expects numbers".to_string());
                    }
                }
                Ok(Value::number(sum))
            }))),
        );

        env.borrow_mut().define_global(
            "*",
            Rc::new(Value::Function(NativeFunction::new("*", |args, _env| {
                let mut product = 1.0;
                for arg in args {
                    if let Value::Number(n) = &**arg {
                        product *= n;
                    } else {
                        return Err("* expects numbers".to_string());
                    }
                }
                Ok(Value::number(product))
            }))),
        );

        let expr = list_from_vec(vec![
            Value::symbol("*"),
            list_from_vec(vec![
                Value::symbol("+"),
                Value::number(1.0),
                Value::number(2.0),
            ]),
            list_from_vec(vec![
                Value::symbol("+"),
                Value::number(3.0),
                Value::number(4.0),
            ]),
        ]);

        let result = eval(expr, &env).unwrap();
        assert_eq!(result, Value::number(21.0));
    }

    #[test]
    fn test_eval_quote() {
        let env = RuntimeEnv::new_global();

        env.borrow_mut().define_global("quote", quote_function());

        let expr = list_from_vec(vec![
            Value::symbol("quote"),
            list_from_vec(vec![
                Value::symbol("+"),
                Value::number(1.0),
                Value::number(2.0),
            ]),
        ]);

        let result = eval(expr, &env).unwrap();

        assert_list_eq(
            &result,
            &[Value::symbol("+"), Value::number(1.0), Value::number(2.0)],
        );
    }
}
