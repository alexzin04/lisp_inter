use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::iter::Peekable;
use std::rc::Rc;
use std::str::Chars;

use crate::data::{Cons, Value};
use crate::enviroment::StaticEnv;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    LParen,
    RParen,
    Quote,
    Dot,
    Nil,
    String(String),
    Symbol(String),
    Number(f64),
    Boolean(bool),
}

pub(crate) struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
    current_line: usize,
    current_column: usize,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Tokenizer {
            chars: input.chars().peekable(),
            current_line: 1,
            current_column: 1,
        }
    }

    pub(crate) fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while let Some(&ch) = self.chars.peek() {
            match ch {
                ' ' | '\t' | '\r' => {
                    self.chars.next();
                    self.current_column += 1;
                }

                '\n' => {
                    self.chars.next();
                    self.current_line += 1;
                    self.current_column = 1;
                }

                ';' => {
                    self.skip_comment();
                }

                '(' => {
                    tokens.push(Token::LParen);
                    self.chars.next();
                    self.current_column += 1;
                }
                ')' => {
                    tokens.push(Token::RParen);
                    self.chars.next();
                    self.current_column += 1;
                }
                '\'' => {
                    tokens.push(Token::Quote);
                    self.chars.next();
                    self.current_column += 1;
                }
                '.' => {
                    tokens.push(Token::Dot);
                    self.chars.next();
                    self.current_column += 1;
                }

                '"' => {
                    let string = self.read_string()?;
                    tokens.push(Token::String(string));
                }

                _ => {
                    if ch.is_ascii_digit() || (ch == '-' && self.peek_next_is_digit()) {
                        let number = self.read_number()?;
                        tokens.push(Token::Number(number));
                    } else if self.is_identifier_start(ch) {
                        let ident = self.read_identifier();

                        match ident.as_str() {
                            "#t" => tokens.push(Token::Boolean(true)),
                            "#f" => tokens.push(Token::Boolean(false)),
                            "nil" => tokens.push(Token::Nil),
                            _ => tokens.push(Token::Symbol(ident)),
                        }
                    } else {
                        return Err(format!(
                            "Unexpected character '{}' at line {}, column {}",
                            ch, self.current_line, self.current_column
                        ));
                    }
                }
            }
        }

        Ok(tokens)
    }

    fn skip_comment(&mut self) {
        for ch in self.chars.by_ref() {
            self.current_column += 1;
            if ch == '\n' {
                self.current_line += 1;
                self.current_column = 1;
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let mut result = String::new();
        self.chars.next();
        self.current_column += 1;

        while let Some(ch) = self.chars.next() {
            self.current_column += 1;

            match ch {
                '"' => {
                    return Ok(result);
                }
                '\\' => {
                    if let Some(next_ch) = self.chars.next() {
                        self.current_column += 1;
                        match next_ch {
                            'n' => result.push('\n'),
                            't' => result.push('\t'),
                            'r' => result.push('\r'),
                            '"' => result.push('"'),
                            '\\' => result.push('\\'),
                            _ => {
                                result.push('\\');
                                result.push(next_ch);
                            }
                        }
                    } else {
                        return Err("Unterminated escape sequence".to_string());
                    }
                }
                '\n' => {
                    self.current_line += 1;
                    self.current_column = 1;
                    result.push(ch);
                }
                _ => result.push(ch),
            }
        }

        Err("Unterminated string literal".to_string())
    }

    fn read_number(&mut self) -> Result<f64, String> {
        let mut buffer = String::new();
        let mut has_dot = false;

        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() {
                buffer.push(ch);
                self.chars.next();
                self.current_column += 1;
            } else if ch == '.' && !has_dot {
                buffer.push(ch);
                has_dot = true;
                self.chars.next();
                self.current_column += 1;
            } else if ch == '-' && buffer.is_empty() {
                buffer.push(ch);
                self.chars.next();
                self.current_column += 1;
            } else {
                break;
            }
        }

        buffer
            .parse::<f64>()
            .map_err(|e| format!("Invalid number '{}': {}", buffer, e))
    }

    fn read_identifier(&mut self) -> String {
        let mut buffer = String::new();

        while let Some(&ch) = self.chars.peek() {
            if self.is_identifier_char(ch) {
                buffer.push(ch);
                self.chars.next();
                self.current_column += 1;
            } else {
                break;
            }
        }

        buffer
    }

    fn peek_next_is_digit(&mut self) -> bool {
        let mut clone = self.chars.clone();
        clone.next();
        if let Some(&ch) = clone.peek() {
            ch.is_ascii_digit()
        } else {
            false
        }
    }

    fn is_identifier_start(&self, ch: char) -> bool {
        ch.is_alphabetic() || "+-*/<=>!?:$%_&~^#".contains(ch)
    }

    fn is_identifier_char(&self, ch: char) -> bool {
        self.is_identifier_start(ch) || ch.is_ascii_digit()
    }
}

pub(crate) struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    pub(crate) fn parse(&mut self) -> Result<Vec<Rc<Value>>, String> {
        let mut expressions = Vec::new();

        while self.position < self.tokens.len() {
            expressions.push(self.read_expr()?);
        }

        Ok(expressions)
    }

    fn read_expr(&mut self) -> Result<Rc<Value>, String> {
        match self.peek_token() {
            Some(Token::LParen) => self.read_list(),
            Some(Token::Quote) => self.read_quote(),
            Some(Token::Number(n)) => {
                self.next_token();
                Ok(Value::number(n))
            }
            Some(Token::String(s)) => {
                self.next_token();
                Ok(Value::string(&s))
            }
            Some(Token::Symbol(s)) => {
                self.next_token();
                Ok(Value::symbol(&s))
            }
            Some(Token::Boolean(b)) => {
                self.next_token();
                Ok(Value::bool(b))
            }
            Some(Token::Nil) => {
                self.next_token();
                Ok(Value::Nil.into())
            }
            Some(token) => Err(format!("Unexpected token: {:?}", token)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn read_list(&mut self) -> Result<Rc<Value>, String> {
        self.expect_token(Token::LParen)?;

        if self.peek_token() == Some(Token::RParen) {
            self.next_token();
            return Ok(Value::nil());
        }

        let first = self.read_expr()?;

        self.read_list_tail(first)
    }

    fn read_list_tail(&mut self, first: Rc<Value>) -> Result<Rc<Value>, String> {
        match self.peek_token() {
            Some(Token::RParen) => {
                self.next_token();
                Ok(Value::cons(first, Value::nil()))
            }
            Some(Token::Dot) => {
                self.next_token();
                let cdr = self.read_expr()?;
                self.expect_token(Token::RParen)?;
                Ok(Value::cons(first, cdr))
            }
            Some(_) => {
                let next = self.read_expr()?;
                let rest = self.read_list_tail(next)?;
                Ok(Value::cons(first, rest))
            }
            None => Err("Unclosed parenthesis".to_string()),
        }
    }
    fn read_quote(&mut self) -> Result<Rc<Value>, String> {
        self.expect_token(Token::Quote)?;

        let quoted_expr = self.read_expr()?;

        Ok(Value::cons(Value::symbol("quote"), quoted_expr))
    }

    fn peek_token(&self) -> Option<Token> {
        self.tokens.get(self.position).cloned()
    }

    fn next_token(&mut self) -> Option<Token> {
        if self.position < self.tokens.len() {
            let token = self.tokens[self.position].clone();
            self.position += 1;
            Some(token)
        } else {
            None
        }
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), String> {
        if let Some(token) = self.next_token() {
            if token == expected {
                Ok(())
            } else {
                Err(format!("Expected {:?}, got {:?}", expected, token))
            }
        } else {
            Err(format!("Expected {:?}, got end of input", expected))
        }
    }
}

pub(crate) fn read_str(input: &str) -> Result<Vec<Rc<Value>>, String> {
    let mut tokenizer = Tokenizer::new(input);
    let tokens = tokenizer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}
pub fn compile_ast(
    expr: Rc<Value>,
    env: &mut StaticEnv,
    is_tail: bool,
) -> Result<Rc<Value>, String> {
    match &*expr {
        Value::Number(_) | Value::String(_) | Value::Bool(_) | Value::Nil => Ok(expr.clone()),

        Value::Symbol(s) => {
            if let Some((depth, index)) = env.lookup(s) {
                Ok(Rc::new(Value::LocalVar(depth, index)))
            } else {
                Ok(expr.clone())
            }
        }

        Value::Pair(cons) => {
            if let Value::Symbol(s) = &*cons.car {
                match s.as_str() {
                    "lambda" => return compile_lambda(cons.cdr.clone(), env, false),
                    "if" => return compile_if(cons.cdr.clone(), env, is_tail),
                    "define" => return compile_define(cons.cdr.clone(), env),
                    _ => {}
                }
            }

            let func = compile_ast(cons.car.clone(), env, false)?;

            let mut args = Vec::new();
            let mut current = &cons.cdr;
            while let Value::Pair(c) = current.as_ref() {
                args.push(compile_ast(c.car.clone(), env, false)?);
                current = &c.cdr;
            }

            if is_tail {
                Ok(Rc::new(Value::TailCall { func, args }))
            } else {
                Ok(Rc::new(Value::Pair(Rc::new(Cons {
                    car: func,
                    cdr: vec_to_list(args),
                }))))
            }
        }

        _ => Ok(expr.clone()),
    }
}
pub(crate) fn vec_to_list(vec: Vec<Rc<Value>>) -> Rc<Value> {
    let mut list = Rc::new(Value::Nil);

    for val in vec.into_iter().rev() {
        list = Rc::new(Value::Pair(Rc::new(Cons {
            car: val,
            cdr: list,
        })));
    }

    list
}

pub(crate) fn extract_param_names(args_expr: Rc<Value>) -> Vec<String> {
    let mut params = Vec::new();
    let mut current = args_expr;

    while let Value::Pair(cons) = &*current {
        if let Value::Symbol(name) = &*cons.car {
            params.push(name.clone());
        }

        current = cons.cdr.clone();
    }

    params
}

fn compile_lambda(
    rest: Rc<Value>,
    env: &mut StaticEnv,
    is_pure: bool,
) -> Result<Rc<Value>, String> {
    if let Value::Pair(args_and_body) = rest.as_ref() {
        let params = extract_param_names(args_and_body.car.clone());
        let params_count = params.len();

        env.push_frame(params);

        let body_expr = match args_and_body.cdr.as_ref() {
            Value::Pair(c) => c.car.clone(),
            _ => Rc::new(Value::Nil),
        };

        let compiled_body = compile_ast(body_expr, env, true)?;
        let final_body = if is_pure {
            Rc::new(Value::Memoized {
                body: compiled_body,
                cache: RefCell::new(HashMap::new()),
            })
        } else {
            compiled_body
        };

        env.pop_frame();

        Ok(Rc::new(Value::Lambda {
            params_count,
            body: final_body,
        }))
    } else {
        Err("Invalid lambda syntax".to_string())
    }
}
fn compile_if(rest: Rc<Value>, env: &mut StaticEnv, is_tail: bool) -> Result<Rc<Value>, String> {
    let mut parts = Vec::new();
    let mut current = rest;
    while let Value::Pair(c) = &*current {
        parts.push(c.car.clone());
        current = c.cdr.clone();
    }

    if parts.len() < 2 {
        return Err("if expects at least cond and then branches".into());
    }

    let cond = compile_ast(parts[0].clone(), env, false)?;
    let then_branch = compile_ast(parts[1].clone(), env, is_tail)?;
    let else_branch = if parts.len() > 2 {
        compile_ast(parts[2].clone(), env, is_tail)?
    } else {
        Rc::new(Value::Nil)
    };

    Ok(Rc::new(Value::If {
        cond,
        then_branch,
        else_branch,
    }))
}
fn compile_define(rest: Rc<Value>, env: &mut StaticEnv) -> Result<Rc<Value>, String> {
    let mut parts = Vec::new();
    let mut current = rest;
    while let Value::Pair(c) = &*current {
        parts.push(c.car.clone());
        current = c.cdr.clone();
    }

    let val = compile_ast(parts[1].clone(), env, false)?;
    Ok(Rc::new(Value::Pair(Rc::new(Cons {
        car: Rc::new(Value::Symbol("define".to_string())),
        cdr: vec_to_list(vec![parts[0].clone(), val]),
    }))))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PurityState {
    Pure,
    Deferred(HashSet<String>),
    Impure,
}

fn deferred_dependency(name: &str) -> PurityState {
    let mut dependencies = HashSet::new();
    dependencies.insert(name.to_string());
    PurityState::Deferred(dependencies)
}

fn merge_purity(left: PurityState, right: PurityState) -> PurityState {
    match (left, right) {
        (PurityState::Impure, _) | (_, PurityState::Impure) => PurityState::Impure,
        (PurityState::Pure, PurityState::Pure) => PurityState::Pure,
        (PurityState::Deferred(mut left), PurityState::Deferred(right)) => {
            left.extend(right);
            PurityState::Deferred(left)
        }
        (PurityState::Deferred(waiting), PurityState::Pure)
        | (PurityState::Pure, PurityState::Deferred(waiting)) => PurityState::Deferred(waiting),
    }
}

fn analyze_call_target_purity(
    func: &Rc<Value>,
    pure_functions: &HashSet<String>,
    known_functions: &HashSet<String>,
    pending_functions: &HashSet<String>,
    current_func_name: &str,
) -> PurityState {
    match func.as_ref() {
        Value::Symbol(name) => {
            if name == "quote" {
                PurityState::Pure
            } else if name == "define" || name == "eval" {
                PurityState::Impure
            } else if name == current_func_name || pure_functions.contains(name) {
                PurityState::Pure
            } else if pending_functions.contains(name) || !known_functions.contains(name) {
                deferred_dependency(name)
            } else {
                PurityState::Impure
            }
        }
        Value::Lambda { body, .. } | Value::Closure { body, .. } | Value::Memoized { body, .. } => {
            is_expression_pure(
                body,
                pure_functions,
                known_functions,
                pending_functions,
                current_func_name,
            )
        }
        _ => PurityState::Impure,
    }
}

fn analyze_call_arguments<'a>(
    args: impl IntoIterator<Item = &'a Rc<Value>>,
    pure_functions: &HashSet<String>,
    known_functions: &HashSet<String>,
    pending_functions: &HashSet<String>,
    current_func_name: &str,
) -> PurityState {
    args.into_iter().fold(PurityState::Pure, |state, arg| {
        merge_purity(
            state,
            is_expression_pure(
                arg,
                pure_functions,
                known_functions,
                pending_functions,
                current_func_name,
            ),
        )
    })
}

pub(crate) fn is_expression_pure(
    expr: &Rc<Value>,
    pure_functions: &HashSet<String>,
    known_functions: &HashSet<String>,
    pending_functions: &HashSet<String>,
    current_func_name: &str,
) -> PurityState {
    match expr.as_ref() {
        Value::Number(_)
        | Value::String(_)
        | Value::Bool(_)
        | Value::Nil
        | Value::LocalVar(_, _)
        | Value::Symbol(_)
        | Value::Function(_)
        | Value::Lambda { .. }
        | Value::Closure { .. } => PurityState::Pure,

        Value::TailCall { func, args } => {
            if matches!(func.as_ref(), Value::Symbol(name) if name == "quote") {
                return PurityState::Pure;
            }

            merge_purity(
                analyze_call_target_purity(
                    func,
                    pure_functions,
                    known_functions,
                    pending_functions,
                    current_func_name,
                ),
                analyze_call_arguments(
                    args.iter(),
                    pure_functions,
                    known_functions,
                    pending_functions,
                    current_func_name,
                ),
            )
        }

        Value::If {
            cond,
            then_branch,
            else_branch,
        } => merge_purity(
            merge_purity(
                is_expression_pure(
                    cond,
                    pure_functions,
                    known_functions,
                    pending_functions,
                    current_func_name,
                ),
                is_expression_pure(
                    then_branch,
                    pure_functions,
                    known_functions,
                    pending_functions,
                    current_func_name,
                ),
            ),
            is_expression_pure(
                else_branch,
                pure_functions,
                known_functions,
                pending_functions,
                current_func_name,
            ),
        ),

        Value::Memoized { body, .. } => is_expression_pure(
            body,
            pure_functions,
            known_functions,
            pending_functions,
            current_func_name,
        ),

        Value::Pair(cons) => {
            if matches!(cons.car.as_ref(), Value::Symbol(name) if name == "quote") {
                return PurityState::Pure;
            }

            let mut state = analyze_call_target_purity(
                &cons.car,
                pure_functions,
                known_functions,
                pending_functions,
                current_func_name,
            );

            let mut current = &cons.cdr;
            while let Value::Pair(arg_cons) = current.as_ref() {
                state = merge_purity(
                    state,
                    is_expression_pure(
                        &arg_cons.car,
                        pure_functions,
                        known_functions,
                        pending_functions,
                        current_func_name,
                    ),
                );
                current = &arg_cons.cdr;
            }

            if matches!(current.as_ref(), Value::Nil) {
                state
            } else {
                PurityState::Impure
            }
        }

        _ => PurityState::Impure,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tokenizer_basic() {
        let input = "(define x (+ 1 2))";
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Symbol("define".to_string()),
                Token::Symbol("x".to_string()),
                Token::LParen,
                Token::Symbol("+".to_string()),
                Token::Number(1.0),
                Token::Number(2.0),
                Token::RParen,
                Token::RParen,
            ]
        );
    }

    #[test]
    fn test_tokenizer_with_strings() {
        let input = r#"(print "Hello, world!")"#;
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Symbol("print".to_string()),
                Token::String("Hello, world!".to_string()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn test_tokenizer_with_quotes() {
        let input = "'(1 2 3) 'symbol";
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::Quote,
                Token::LParen,
                Token::Number(1.0),
                Token::Number(2.0),
                Token::Number(3.0),
                Token::RParen,
                Token::Quote,
                Token::Symbol("symbol".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenizer_booleans() {
        let input = "#t #f";
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(tokens, vec![Token::Boolean(true), Token::Boolean(false),]);
    }

    #[test]
    fn test_tokenizer_comments() {
        let input = "; This is a comment\n(define x 42) ; another comment";
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Symbol("define".to_string()),
                Token::Symbol("x".to_string()),
                Token::Number(42.0),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn test_parser_simple() {
        let input = "(+ 1 2)";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 1);

        let expr = &ast[0];
        assert!(expr.is_pair());

        if let Value::Pair(cons) = &**expr {
            let items = cons.to_vec().expect("Expected proper list");
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::symbol("+"));
            assert_eq!(items[1], Value::number(1.0));
            assert_eq!(items[2], Value::number(2.0));
        } else {
            panic!("Expected pair");
        }
    }

    #[test]
    fn test_parser_nested() {
        let input = "(define square (lambda (x) (* x x)))";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 1);

        let expr = &ast[0];

        if let Value::Pair(outer_cons) = &**expr {
            let outer_items = outer_cons.to_vec().expect("Expected proper list");
            assert_eq!(outer_items.len(), 3);
            assert_eq!(outer_items[0], Value::symbol("define"));
            assert_eq!(outer_items[1], Value::symbol("square"));

            if let Value::Pair(lambda_cons) = &*outer_items[2] {
                let lambda_items = lambda_cons.to_vec().expect("Expected proper list");
                assert_eq!(lambda_items.len(), 3);
                assert_eq!(lambda_items[0], Value::symbol("lambda"));

                if let Value::Pair(params_cons) = &*lambda_items[1] {
                    let params = params_cons.to_vec().expect("Expected proper list");
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0], Value::symbol("x"));
                } else {
                    panic!("Expected params to be a pair");
                }

                if let Value::Pair(body_cons) = &*lambda_items[2] {
                    let body = body_cons.to_vec().expect("Expected proper list");
                    assert_eq!(body.len(), 3);
                    assert_eq!(body[0], Value::symbol("*"));
                    assert_eq!(body[1], Value::symbol("x"));
                    assert_eq!(body[2], Value::symbol("x"));
                } else {
                    panic!("Expected body to be a pair");
                }
            } else {
                panic!("Expected lambda to be a pair");
            }
        } else {
            panic!("Expected outer expression to be a pair");
        }
    }

    #[test]
    fn test_parser_quote() {
        let input = "'(1 2 3)";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 1);

        let expr = &ast[0];

        if let Value::Pair(cons) = &**expr {
            let items = cons.to_vec().expect("Expected proper list");
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::symbol("quote"));

            if let Value::Pair(quoted_cons) = &*items[1] {
                let quoted_items = quoted_cons.to_vec().expect("Expected proper list");
                assert_eq!(quoted_items.len(), 3);
                assert_eq!(quoted_items[0], Value::number(1.0));
                assert_eq!(quoted_items[1], Value::number(2.0));
                assert_eq!(quoted_items[2], Value::number(3.0));
            } else {
                panic!("Expected quoted expression to be a pair");
            }
        } else {
            panic!("Expected quote expression to be a pair");
        }
    }

    #[test]
    fn test_parser_multiple_expressions() {
        let input = "(define x 1) (define y 2) (+ x y)";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 3);

        if let Value::Pair(cons1) = &*ast[0] {
            let items1 = cons1.to_vec().expect("Expected proper list");
            assert_eq!(items1.len(), 3);
            assert_eq!(items1[0], Value::symbol("define"));
            assert_eq!(items1[1], Value::symbol("x"));
            assert_eq!(items1[2], Value::number(1.0));
        }

        if let Value::Pair(cons2) = &*ast[1] {
            let items2 = cons2.to_vec().expect("Expected proper list");
            assert_eq!(items2.len(), 3);
            assert_eq!(items2[0], Value::symbol("define"));
            assert_eq!(items2[1], Value::symbol("y"));
            assert_eq!(items2[2], Value::number(2.0));
        }

        if let Value::Pair(cons3) = &*ast[2] {
            let items3 = cons3.to_vec().expect("Expected proper list");
            assert_eq!(items3.len(), 3);
            assert_eq!(items3[0], Value::symbol("+"));
            assert_eq!(items3[1], Value::symbol("x"));
            assert_eq!(items3[2], Value::symbol("y"));
        }
    }

    #[test]
    fn test_parser_strings() {
        let input = r#"(concat "Hello, " "world!")"#;
        let ast = read_str(input).unwrap();

        let expr = &ast[0];

        if let Value::Pair(cons) = &**expr {
            let items = cons.to_vec().expect("Expected proper list");
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::symbol("concat"));
            assert_eq!(items[1], Value::string("Hello, "));
            assert_eq!(items[2], Value::string("world!"));
        } else {
            panic!("Expected pair");
        }
    }

    #[test]
    fn test_parser_negative_numbers() {
        let input = "(- 10 -5)";
        let ast = read_str(input).unwrap();

        let expr = &ast[0];

        if let Value::Pair(cons) = &**expr {
            let items = cons.to_vec().expect("Expected proper list");
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::symbol("-"));
            assert_eq!(items[1], Value::number(10.0));
            assert_eq!(items[2], Value::number(-5.0));
        } else {
            panic!("Expected pair");
        }
    }

    #[test]
    fn test_parser_floats() {
        let input = "(+ 3.1 2.71)";
        let ast = read_str(input).unwrap();

        let expr = &ast[0];

        if let Value::Pair(cons) = &**expr {
            let items = cons.to_vec().expect("Expected proper list");
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::symbol("+"));

            if let Value::Number(n1) = &*items[1] {
                assert!((*n1 - 3.1).abs() < 0.0001, "Expected ~3.1, got {}", n1);
            } else {
                panic!("Expected number");
            }

            if let Value::Number(n2) = &*items[2] {
                assert!((*n2 - 2.71).abs() < 0.0001, "Expected ~2.71, got {}", n2);
            } else {
                panic!("Expected number");
            }
        } else {
            panic!("Expected pair");
        }
    }

    #[test]
    fn test_parser_empty_list() {
        let input = "()";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(ast[0].is_nil());
    }

    #[test]
    fn test_parser_dotted_pair() {
        let input = "(1 . 2)";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 1);

        let expr = &ast[0];
        assert!(expr.is_pair());
        assert!(!expr.is_list());

        if let Value::Pair(cons) = &**expr {
            assert!(cons.to_vec().is_none());

            assert_eq!(cons.car, Value::number(1.0));
            assert_eq!(cons.cdr, Value::number(2.0));
        }
    }

    #[test]
    fn test_parser_improper_list() {
        let input = "(1 2 . 3)";
        let ast = read_str(input).unwrap();

        assert_eq!(ast.len(), 1);

        let expr = &ast[0];
        assert!(expr.is_pair());
        assert!(!expr.is_list());

        if let Value::Pair(outer) = &**expr {
            assert_eq!(outer.car, Value::number(1.0));

            if let Value::Pair(inner) = &*outer.cdr {
                assert_eq!(inner.car, Value::number(2.0));
                assert_eq!(inner.cdr, Value::number(3.0));
            } else {
                panic!("Expected inner pair");
            }
        }
    }

    #[test]
    fn test_tokenizer_escaped_strings() {
        let input = r#""Line 1\nLine 2""#;
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(tokens, vec![Token::String("Line 1\nLine 2".to_string()),]);
    }

    #[test]
    fn test_parser_nill() {
        let input = "nil";
        let ast = read_str(input).unwrap();
        println!("{:?}", ast);
        assert_eq!(ast.len(), 1);
        assert!(ast[0].is_nil());
    }
}
