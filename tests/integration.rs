use sprite::eval;

fn num(src: &str) -> f64 {
    match eval(src).unwrap() {
        sprite::Value::Number(n) => n,
        other => panic!("expected number, got {}", other),
    }
}

#[test]
fn arithmetic_precedence() {
    assert_eq!(num("2 + 3 * 4"), 14.0);
    assert_eq!(num("(2 + 3) * 4"), 20.0);
    assert_eq!(num("10 - 2 - 3"), 5.0);
}

#[test]
fn recursive_factorial() {
    let src = r#"
        fn fact(n) {
            if n <= 1 {
                return 1
            }
            return n * fact(n - 1)
        }
        fact(5)
    "#;
    assert_eq!(num(src), 120.0);
}

#[test]
fn while_loop_sum() {
    let src = r#"
        let i = 1
        let sum = 0
        while i <= 10 {
            sum = sum + i
            i = i + 1
        }
        sum
    "#;
    assert_eq!(num(src), 55.0);
}

#[test]
fn string_concatenation() {
    let v = eval(r#""hello" + " " + "world""#).unwrap();
    assert_eq!(v.to_string(), "hello world");
}

#[test]
fn if_else_branches() {
    let v = eval(r#"
        let x = 5
        if x > 10 {
            "big"
        } else {
            "small"
        }
    "#)
    .unwrap();
    assert_eq!(v.to_string(), "small");
}

#[test]
fn closures_capture_environment() {
    let src = r#"
        fn make_adder(n) {
            fn adder(x) {
                return x + n
            }
            return adder
        }
        let add5 = make_adder(5)
        add5(10)
    "#;
    assert_eq!(num(src), 15.0);
}

#[test]
fn undefined_variable_is_runtime_error() {
    let err = eval("x + 1").unwrap_err();
    assert!(matches!(err, sprite::SpriteError::Runtime(_)));
}

#[test]
fn type_error_on_bad_arithmetic() {
    let err = eval(r#""abc" - 1"#).unwrap_err();
    assert!(matches!(err, sprite::SpriteError::Runtime(_)));
}

#[test]
fn unterminated_string_is_lex_error() {
    let err = eval("\"abc").unwrap_err();
    assert!(matches!(err, sprite::SpriteError::Lex(_)));
}

#[test]
fn deeply_nested_parens_returns_parse_error_not_crash() {
    let mut src = String::new();
    for _ in 0..10_000 {
        src.push('(');
    }
    src.push('1');
    for _ in 0..10_000 {
        src.push(')');
    }
    let err = eval(&src).unwrap_err();
    assert!(matches!(err, sprite::SpriteError::Parse(_)));
}
