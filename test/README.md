# Optimization examples

These files contain Lisp programs copied from the examples that were in
`src/main.rs`.

- `constant_folding.lisp`: arithmetic constants inside `get-seconds`
- `closures.lisp`: closure capture through `make-adder`
- `tail_calls.lisp`: tail-recursive list summation
- `memoization.lisp`: repeated pure function call
- `fib_memoization.lisp`: recursive pure function memoization
- `nested_lambdas.lisp`: deep lambda nesting and local variables

Run all examples with:

```sh
cargo run
```

The runner reads every `.lisp` file in this directory, prints each expression
result, shows rewritten expressions when optimization changes them, and prints
parse/optimization/evaluation timings plus profiler counters.
