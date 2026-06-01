use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    data::Value,
    enviroment::{PendingPureFunction, RuntimeEnv, StaticEnv},
    function::{eval, NativeFunction},
    parcer::{compile_ast, is_expression_pure, PurityState},
};

fn memoized_body(body: Rc<Value>) -> Rc<Value> {
    match body.as_ref() {
        Value::Memoized { .. } => body,
        _ => Rc::new(Value::Memoized {
            body,
            cache: RefCell::new(HashMap::new()),
        }),
    }
}

fn clear_purity_metadata(env_rc: &Rc<RefCell<RuntimeEnv>>, name: &str) {
    let env = env_rc.borrow();
    env.pure_functions.borrow_mut().remove(name);
    env.pending_pure_functions.borrow_mut().remove(name);
}

fn purity_context(
    env_rc: &Rc<RefCell<RuntimeEnv>>,
) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
    let env = env_rc.borrow();
    let pure_functions = env.pure_functions.borrow().clone();
    let known_functions = env.global_symbols();
    let pending_functions = env
        .pending_pure_functions
        .borrow()
        .keys()
        .cloned()
        .collect();
    (pure_functions, known_functions, pending_functions)
}

fn install_function(
    env_rc: &Rc<RefCell<RuntimeEnv>>,
    name: &str,
    params_count: usize,
    body: Rc<Value>,
    captured: Vec<Vec<Rc<Value>>>,
) {
    let closure = Rc::new(Value::Closure {
        params_count,
        body,
        captured,
    });

    env_rc.borrow().define_global(name, closure);
}

fn mark_function_as_pure(
    env_rc: &Rc<RefCell<RuntimeEnv>>,
    name: &str,
    params_count: usize,
    body: Rc<Value>,
    captured: Vec<Vec<Rc<Value>>>,
) {
    let memoization_enabled = env_rc.borrow().memoization_enabled;
    let body = if memoization_enabled {
        memoized_body(body)
    } else {
        body
    };

    install_function(env_rc, name, params_count, body, captured);

    let env = env_rc.borrow();
    env.pure_functions.borrow_mut().insert(name.to_string());
    env.pending_pure_functions.borrow_mut().remove(name);
    env.profiler.borrow_mut().pure_functions_detected += 1;
}

fn store_pending_function(
    env_rc: &Rc<RefCell<RuntimeEnv>>,
    name: String,
    params_count: usize,
    body: Rc<Value>,
    captured: Vec<Vec<Rc<Value>>>,
    unresolved_dependencies: HashSet<String>,
) {
    install_function(env_rc, &name, params_count, body.clone(), captured.clone());

    let env = env_rc.borrow();
    env.pending_pure_functions.borrow_mut().insert(
        name,
        PendingPureFunction {
            params_count,
            body,
            captured,
            unresolved_dependencies,
        },
    );
}

fn refresh_pending_pure_functions(env_rc: &Rc<RefCell<RuntimeEnv>>) {
    loop {
        let pending_names = {
            let env = env_rc.borrow();
            env.pending_pure_functions
                .borrow()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };

        if pending_names.is_empty() {
            break;
        }

        let mut changed = false;

        for name in pending_names {
            let pending = {
                let env = env_rc.borrow();
                env.pending_pure_functions.borrow().get(&name).cloned()
            };

            let Some(pending) = pending else {
                continue;
            };

            let (pure_functions, known_functions, pending_functions) = purity_context(env_rc);
            let purity = is_expression_pure(
                &pending.body,
                &pure_functions,
                &known_functions,
                &pending_functions,
                &name,
            );

            match purity {
                PurityState::Pure => {
                    mark_function_as_pure(
                        env_rc,
                        &name,
                        pending.params_count,
                        pending.body.clone(),
                        pending.captured.clone(),
                    );
                    changed = true;
                }
                PurityState::Deferred(unresolved_dependencies) => {
                    let env = env_rc.borrow();
                    if let Some(entry) = env.pending_pure_functions.borrow_mut().get_mut(&name) {
                        entry.unresolved_dependencies = unresolved_dependencies;
                    }
                }
                PurityState::Impure => {
                    env_rc
                        .borrow()
                        .pending_pure_functions
                        .borrow_mut()
                        .remove(&name);
                    changed = true;
                }
            }
        }

        if changed {
            continue;
        }

        let promotable_components = find_promotable_pending_cycles(env_rc);
        if promotable_components.is_empty() {
            break;
        }

        for component in promotable_components {
            for name in component {
                let pending = {
                    let env = env_rc.borrow();
                    env.pending_pure_functions.borrow().get(&name).cloned()
                };

                let Some(pending) = pending else {
                    continue;
                };

                mark_function_as_pure(
                    env_rc,
                    &name,
                    pending.params_count,
                    pending.body,
                    pending.captured,
                );
            }
        }
    }
}

fn pending_dependency_graph(env_rc: &Rc<RefCell<RuntimeEnv>>) -> HashMap<String, Vec<String>> {
    let env = env_rc.borrow();
    let pending = env.pending_pure_functions.borrow();
    let pending_names = pending.keys().cloned().collect::<HashSet<_>>();

    pending
        .iter()
        .map(|(name, function)| {
            let dependencies = function
                .unresolved_dependencies
                .iter()
                .filter(|dependency| pending_names.contains(*dependency))
                .cloned()
                .collect::<Vec<_>>();
            (name.clone(), dependencies)
        })
        .collect()
}

fn dfs_postorder(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    order: &mut Vec<String>,
) {
    if !visited.insert(node.to_string()) {
        return;
    }

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            dfs_postorder(neighbor, graph, visited, order);
        }
    }

    order.push(node.to_string());
}

fn transpose_graph(graph: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    let mut reversed = graph
        .keys()
        .cloned()
        .map(|name| (name, Vec::new()))
        .collect::<HashMap<_, _>>();

    for (node, neighbors) in graph {
        for neighbor in neighbors {
            reversed
                .entry(neighbor.clone())
                .or_default()
                .push(node.clone());
        }
    }

    reversed
}

fn dfs_collect_component(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    component: &mut Vec<String>,
) {
    if !visited.insert(node.to_string()) {
        return;
    }

    component.push(node.to_string());
    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            dfs_collect_component(neighbor, graph, visited, component);
        }
    }
}

fn strongly_connected_components(graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();

    for node in graph.keys() {
        dfs_postorder(node, graph, &mut visited, &mut order);
    }

    let reversed = transpose_graph(graph);
    let mut visited = HashSet::new();
    let mut components = Vec::new();

    while let Some(node) = order.pop() {
        if visited.contains(&node) {
            continue;
        }

        let mut component = Vec::new();
        dfs_collect_component(&node, &reversed, &mut visited, &mut component);
        components.push(component);
    }

    components
}

fn is_promotable_pending_cycle(
    env_rc: &Rc<RefCell<RuntimeEnv>>,
    cycle: &[String],
    base_pure_functions: &HashSet<String>,
    known_functions: &HashSet<String>,
    pending_functions: &HashSet<String>,
) -> bool {
    let assumed_pure_functions = {
        let mut assumed = base_pure_functions.clone();
        assumed.extend(cycle.iter().cloned());
        assumed
    };

    cycle.iter().all(|name| {
        let pending = {
            let env = env_rc.borrow();
            env.pending_pure_functions.borrow().get(name).cloned()
        };

        let Some(pending) = pending else {
            return false;
        };

        matches!(
            is_expression_pure(
                &pending.body,
                &assumed_pure_functions,
                known_functions,
                pending_functions,
                name,
            ),
            PurityState::Pure
        )
    })
}

fn find_promotable_pending_cycles(env_rc: &Rc<RefCell<RuntimeEnv>>) -> Vec<Vec<String>> {
    let (base_pure_functions, known_functions, pending_functions) = purity_context(env_rc);
    let dependency_graph = pending_dependency_graph(env_rc);

    strongly_connected_components(&dependency_graph)
        .into_iter()
        .filter(|component| component.len() > 1)
        .filter(|component| {
            is_promotable_pending_cycle(
                env_rc,
                component,
                &base_pure_functions,
                &known_functions,
                &pending_functions,
            )
        })
        .collect()
}

pub(super) fn define_function() -> Rc<Value> {
    Value::Function(NativeFunction::new(
        "define",
        |args: &[Rc<Value>], env_rc: &Rc<RefCell<RuntimeEnv>>| {
            if args.len() < 2 {
                return Err("define expects at least 2 arguments".to_string());
            }

            match &*args[0] {
                Value::Symbol(name) => {
                    clear_purity_metadata(env_rc, name);
                    let value = eval(args[1].clone(), env_rc)?;
                    env_rc.borrow().define_global(name, value);
                    refresh_pending_pure_functions(env_rc);
                    Ok(Value::nil())
                }

                Value::Pair(signature) => {
                    let mut iter = signature.cars();
                    let name_val = iter.next().ok_or("define: missing name")?;
                    let name = match &**name_val {
                        Value::Symbol(s) => s.clone(),
                        _ => return Err("Name must be symbol".to_string()),
                    };

                    let params: Vec<String> = iter
                        .map(|p| match &**p {
                            Value::Symbol(s) => Ok(s.clone()),
                            _ => Err("Param must be symbol".to_string()),
                        })
                        .collect::<Result<_, _>>()?;

                    let params_count = params.len();

                    let mut static_env = StaticEnv::new();
                    static_env.push_frame(params);

                    let body_to_analyze = args[1].clone();
                    let analyzed_body = compile_ast(body_to_analyze, &mut static_env, true)?;
                    let captured = env_rc.borrow().get_locals_clone();
                    clear_purity_metadata(env_rc, &name);

                    let (pure_functions, known_functions, pending_functions) =
                        purity_context(env_rc);
                    match is_expression_pure(
                        &analyzed_body,
                        &pure_functions,
                        &known_functions,
                        &pending_functions,
                        &name,
                    ) {
                        PurityState::Pure => {
                            mark_function_as_pure(
                                env_rc,
                                &name,
                                params_count,
                                analyzed_body,
                                captured,
                            );
                        }
                        PurityState::Deferred(unresolved_dependencies) => {
                            store_pending_function(
                                env_rc,
                                name.clone(),
                                params_count,
                                analyzed_body,
                                captured,
                                unresolved_dependencies,
                            );
                        }
                        PurityState::Impure => {
                            install_function(env_rc, &name, params_count, analyzed_body, captured);
                        }
                    }

                    refresh_pending_pure_functions(env_rc);
                    Ok(Value::nil())
                }
                _ => Err("define error: invalid syntax".to_string()),
            }
        },
    ))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{function::add_all_basic_func, parcer::read_str};

    #[test]
    fn promotes_pending_function_after_dependency_becomes_pure() {
        let mut env = RuntimeEnv::new_global();
        add_all_basic_func(&mut env);

        let expressions = read_str(
            "
            (define (foo x)
              (bar x))

            (define (bar x)
              (+ x 1))
            ",
        )
        .unwrap();

        eval(expressions[0].clone(), &env).unwrap();

        {
            let runtime = env.borrow();

            let pending = runtime.pending_pure_functions.borrow();
            let foo_pending = pending.get("foo").expect("foo should be pending");
            assert!(foo_pending.unresolved_dependencies.contains("bar"));

            let foo = runtime.lookup_global("foo").expect("foo should be defined");
            match foo.as_ref() {
                Value::Closure { body, .. } => {
                    assert!(!matches!(body.as_ref(), Value::Memoized { .. }));
                }
                other => panic!("expected foo closure, got {other:?}"),
            }
        }

        eval(expressions[1].clone(), &env).unwrap();

        {
            let runtime = env.borrow();

            assert!(runtime.pending_pure_functions.borrow().is_empty());
            assert!(runtime.pure_functions.borrow().contains("foo"));
            assert!(runtime.pure_functions.borrow().contains("bar"));

            let foo = runtime
                .lookup_global("foo")
                .expect("foo should stay defined");
            let bar = runtime.lookup_global("bar").expect("bar should be defined");

            for function in [foo, bar] {
                match function.as_ref() {
                    Value::Closure { body, .. } => {
                        assert!(matches!(body.as_ref(), Value::Memoized { .. }));
                    }
                    other => panic!("expected closure, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn promotes_mutually_recursive_pure_functions_together() {
        let mut env = RuntimeEnv::new_global();
        add_all_basic_func(&mut env);

        let expressions = read_str(
            "
            (define (step-a x y)
              (step-b (+ x 1) y))

            (define (step-b x y)
              (step-a x (+ y 1)))
            ",
        )
        .unwrap();

        eval(expressions[0].clone(), &env).unwrap();
        eval(expressions[1].clone(), &env).unwrap();

        let runtime = env.borrow();
        assert!(runtime.pending_pure_functions.borrow().is_empty());
        assert!(runtime.pure_functions.borrow().contains("step-a"));
        assert!(runtime.pure_functions.borrow().contains("step-b"));

        let step_a = runtime
            .lookup_global("step-a")
            .expect("step-a should be defined");
        let step_b = runtime
            .lookup_global("step-b")
            .expect("step-b should be defined");

        for function in [step_a, step_b] {
            match function.as_ref() {
                Value::Closure { body, .. } => {
                    assert!(matches!(body.as_ref(), Value::Memoized { .. }));
                }
                other => panic!("expected closure, got {other:?}"),
            }
        }
    }

    #[test]
    fn promotes_two_function_cycle_a_to_b_to_a() {
        let mut env = RuntimeEnv::new_global();
        add_all_basic_func(&mut env);

        let expressions = read_str(
            "
            (define (a x)
              (b x))

            (define (b x)
              (a x))
            ",
        )
        .unwrap();

        eval(expressions[0].clone(), &env).unwrap();
        eval(expressions[1].clone(), &env).unwrap();

        let runtime = env.borrow();
        assert!(runtime.pending_pure_functions.borrow().is_empty());
        assert!(runtime.pure_functions.borrow().contains("a"));
        assert!(runtime.pure_functions.borrow().contains("b"));

        let a = runtime.lookup_global("a").expect("a should be defined");
        let b = runtime.lookup_global("b").expect("b should be defined");

        for function in [a, b] {
            match function.as_ref() {
                Value::Closure { body, .. } => {
                    assert!(matches!(body.as_ref(), Value::Memoized { .. }));
                }
                other => panic!("expected closure, got {other:?}"),
            }
        }
    }
}
