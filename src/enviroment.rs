use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{data::Value, profiler::Profiler};

#[derive(Debug, Clone)]
pub(crate) struct PendingPureFunction {
    pub params_count: usize,
    pub body: Rc<Value>,
    pub captured: Vec<Vec<Rc<Value>>>,
    pub unresolved_dependencies: HashSet<String>,
}

#[derive(Debug)]
pub(crate) struct RuntimeEnv {
    frames: Vec<Vec<Rc<Value>>>,

    globals: RefCell<HashMap<String, Rc<Value>>>,
    pub pure_functions: RefCell<HashSet<String>>,
    pub pending_pure_functions: RefCell<HashMap<String, PendingPureFunction>>,
    pub profiler: RefCell<Profiler>,
    pub memoization_enabled: bool,
}

impl RuntimeEnv {
    pub(crate) fn new_global() -> Rc<RefCell<Self>> {
        let env = Rc::new(RefCell::new(RuntimeEnv {
            frames: Vec::new(),
            globals: RefCell::new(HashMap::new()),
            pure_functions: RefCell::new(HashSet::new()),
            pending_pure_functions: RefCell::new(HashMap::new()),
            profiler: RefCell::new(Profiler::new()),
            memoization_enabled: true,
        }));

        {
            let b = env.borrow();

            {
                let mut pure = b.pure_functions.borrow_mut();
                let basic_pure = vec![
                    "+", "-", "*", "/", "=", "<", ">", "<=", ">=", "car", "cdr", "cons", "null?",
                    "not", "list", "pair?", "eq?", "equal?", "and", "or",
                ];
                for name in basic_pure {
                    pure.insert(name.to_string());
                }
            }

            let mut g = b.globals.borrow_mut();
            g.insert("nil".to_string(), Rc::new(Value::Nil));
            g.insert("#t".to_string(), Rc::new(Value::Bool(true)));
            g.insert("#f".to_string(), Rc::new(Value::Bool(false)));
            g.insert(
                "pi".to_string(),
                Rc::new(Value::Number(std::f64::consts::PI)),
            );
            g.insert("e".to_string(), Rc::new(Value::Number(std::f64::consts::E)));
        }

        env
    }

    pub(crate) fn lookup_local(&self, depth: usize, index: usize) -> Option<Rc<Value>> {
        let frame_idx = self.frames.len().checked_sub(1 + depth)?;
        self.frames.get(frame_idx)?.get(index).cloned()
    }

    pub(crate) fn lookup_global(&self, symbol: &str) -> Option<Rc<Value>> {
        self.globals.borrow().get(symbol).cloned()
    }

    pub(crate) fn define_global(&self, symbol: &str, value: Rc<Value>) {
        self.globals.borrow_mut().insert(symbol.to_string(), value);
    }

    pub(crate) fn global_symbols(&self) -> HashSet<String> {
        self.globals.borrow().keys().cloned().collect()
    }

    pub(crate) fn get_locals_clone(&self) -> Vec<Vec<Rc<Value>>> {
        self.frames.clone()
    }

    pub fn set_locals(&mut self, new_locals: Vec<Vec<Rc<Value>>>) {
        self.frames = new_locals;
    }

    pub fn get_current_frame(&self) -> &Vec<Rc<Value>> {
        self.frames
            .last()
            .expect("Runtime Error: Attempt to access local frame outside of function context")
    }

    pub fn get_current_frame_clone(&self) -> Vec<Rc<Value>> {
        self.get_current_frame().clone()
    }
}
#[derive(Debug)]
pub(crate) struct StaticEnv {
    pub frames: Vec<Vec<String>>,
}

impl StaticEnv {
    pub fn new() -> Self {
        StaticEnv { frames: Vec::new() }
    }

    pub fn push_frame(&mut self, params: Vec<String>) {
        self.frames.push(params);
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn lookup(&self, name: &str) -> Option<(usize, usize)> {
        for (depth, frame) in self.frames.iter().rev().enumerate() {
            if let Some(index) = frame.iter().position(|x| x == name) {
                return Some((depth, index));
            }
        }

        None
    }
}
