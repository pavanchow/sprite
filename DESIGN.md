# Sprite design notes

## Pipeline

```
source text -> Lexer -> Vec<Token> -> Parser -> AST (Block) -> Interpreter -> Value
```

Each stage is a plain module in `src/`:

- `lexer.rs`: turns source text into a flat token stream. Single pass over a
  `Vec<char>`, no lookahead beyond one character. Skips whitespace and `#`
  line comments. Reports `SpriteError::Lex` on invalid characters or an
  unterminated string, it never panics.
- `ast.rs`: the expression and statement types the parser produces.
- `parser.rs`: recursive-descent parser over the token stream, producing a
  `Block` (`Vec<Stmt>`). Tracks a recursion depth counter and returns
  `SpriteError::Parse` once nesting passes a bound, instead of overflowing
  the Rust call stack on hostile input such as ten thousand nested parens.
- `interpreter.rs`: walks the AST directly, no bytecode or intermediate
  representation. Tracks a call-depth counter for the same reason as the
  parser, this time to catch runaway recursion in Sprite code and return
  `SpriteError::Runtime("stack overflow")` instead of crashing the host
  process.
- `value.rs`: the `Value` enum and the environment (`Scope`) used for
  variable lookup and closures.

## Grammar

Roughly EBNF, semicolons are optional and statements are also separated
just by starting a new statement:

```
program    := statement* EOF

statement  := letStmt | fnDecl | ifStmt | whileStmt | returnStmt
            | block | exprStmt

letStmt    := "let" IDENT "=" expression
fnDecl     := "fn" IDENT "(" params? ")" block
ifStmt     := "if" expression block ("else" (ifStmt | block))?
whileStmt  := "while" expression block
returnStmt := "return" expression?
block      := "{" statement* "}"
exprStmt   := expression

params     := IDENT ("," IDENT)*

expression := assignment
assignment := logic_or ("=" assignment)?
logic_or   := logic_and ("or" logic_and)*
logic_and  := equality ("and" equality)*
equality   := comparison (("==" | "!=") comparison)*
comparison := term (("<" | ">" | "<=" | ">=") term)*
term       := factor (("+" | "-") factor)*
factor     := unary (("*" | "/" | "%") unary)*
unary      := ("-" | "not") unary | call
call       := primary ("(" args? ")")*
args       := expression ("," expression)*
primary    := NUMBER | STRING | "true" | "false" | "nil"
            | IDENT | "(" expression ")"
```

`print(...)` is not a keyword, it is resolved as an ordinary call to a
builtin during interpretation, so the grammar for calls stays uniform.

## Value model

```rust
enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Nil,
    Function(Rc<FunctionDef>),
}
```

`FunctionDef` holds the parameter names, the body block, and a captured
`Env`, an `Rc<RefCell<Scope>>`. `Scope` is a linked chain of hash maps with
an optional parent, so a nested function's scope points back at the scope
where it was defined. Because `Env` is reference-counted and interior
mutable, two closures that capture the same variable observe each other's
mutations, which is what makes the counter-closure example in the README
work: `increment` mutates `count` in its parent's scope on every call.

Truthiness: `false` and `nil` are falsy, everything else, including `0` and
`""`, is truthy. Equality (`==`, `!=`) compares by value across
number/string/bool/nil, cross-type comparisons are always unequal rather
than an error. Ordering (`<`, `>`, `<=`, `>=`) is defined for numbers and
strings only, comparing anything else is a runtime type error.

A block statement (including the winning branch of an `if/else`) evaluates
to the value of its last expression statement, similar to how expression
oriented languages behave, which lets a REPL print something useful for
`if cond { "a" } else { "b" }` without an explicit `return`.

## Depth bounds against hostile input

Two independent counters guard the two places a maliciously deep input can
blow the native call stack:

1. **Parser recursion depth** (`MAX_DEPTH = 200` in `parser.rs`). Every
   recursive descent into an expression, statement, or block increments a
   counter and decrements it on the way back out. Once it passes the bound
   the parser returns `SpriteError::Parse("expression nested too deeply")`
   instead of recursing further. This catches pathological input like
   `((((((...1...))))))` with thousands of parens.
2. **Interpreter call depth** (`MAX_CALL_DEPTH = 512` in `interpreter.rs`).
   Every Sprite function call increments a counter for the lifetime of the
   call and returns `SpriteError::Runtime("stack overflow")` once it passes
   the bound, so unbounded recursion in a Sprite script is a catchable
   error, not a crashed host process.

Both are exercised by tests: `deeply_nested_parens_returns_parse_error_not_crash`
in `tests/integration.rs` builds a 10,000-deep paren expression and asserts
a `Parse` error comes back rather than a panic.

## Error handling

There is exactly one error type, `SpriteError`, with three variants (`Lex`,
`Parse`, `Runtime`), each wrapping a `String` message. `eval` and
`Session::run` return `Result<Value, SpriteError>` end to end. No stage
calls `panic!`, `unwrap`, or indexes out of bounds on attacker-controlled
input, arithmetic on numbers uses `f64` semantics (division by zero is a
checked runtime error, not IEEE infinity, to keep behavior predictable for
embedders).
