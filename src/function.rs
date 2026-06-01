use core::fmt;
use std::cell::RefCell;
use std::rc::Rc;

mod arifmetic;
mod compare;
mod define;
mod eval;
mod list;
mod logic;

use crate::data::Value;

pub(crate) type LispNativeFn =
    fn(&[Rc<Value>], &Rc<RefCell<RuntimeEnv>>) -> Result<Rc<Value>, String>;

use crate::enviroment::RuntimeEnv;

pub(crate) use eval::eval;

pub(crate) trait LispFunction: fmt::Debug {
    fn call(&self, args: &[Rc<Value>], env: &Rc<RefCell<RuntimeEnv>>) -> Result<Rc<Value>, String>;
}

#[derive(Clone)]
pub(crate) struct NativeFunction {
    name: String,
    func: LispNativeFn,
}

impl NativeFunction {
    pub(crate) fn new(name: &str, func: LispNativeFn) -> Rc<Self> {
        NativeFunction {
            name: name.to_string(),
            func,
        }
        .into()
    }
}

impl LispFunction for NativeFunction {
    fn call(&self, args: &[Rc<Value>], env: &Rc<RefCell<RuntimeEnv>>) -> Result<Rc<Value>, String> {
        (self.func)(args, env)
    }
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native-function: {}>", self.name)
    }
}

pub(crate) fn add_all_basic_func(env: &mut Rc<RefCell<RuntimeEnv>>) {
    let mut env: std::cell::RefMut<'_, RuntimeEnv> = env.borrow_mut();
    env.define_global("eval", eval::eval_function());
    env.define_global("define", define::define_function());
    env.define_global("quote", eval::quote_function());

    arifmetic::add_arifmetic_functions(&mut env);
    compare::add_compare_functions(&mut env);
    list::add_list_functions(&mut env);
    logic::add_logic_functions(&mut env);
}
