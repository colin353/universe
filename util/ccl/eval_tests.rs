use crate::{exec_or_panic, Value};

#[test]
fn test_operator_precedence() {
    assert_eq!(exec_or_panic("1+2", ""), Value::Number(3.0));
    assert_eq!(exec_or_panic("1+2*3", ""), Value::Number(7.0));
    assert_eq!(exec_or_panic("(1+2) * 3", ""), Value::Number(9.0));
    assert_eq!(exec_or_panic("(1/2) * 4", ""), Value::Number(2.0));
}

#[test]
fn test_boolean_operators() {
    assert_eq!(exec_or_panic("true", ""), Value::Bool(true));
    assert_eq!(exec_or_panic("false", ""), Value::Bool(false));
    assert_eq!(exec_or_panic("true || false", ""), Value::Bool(true));
    assert_eq!(exec_or_panic("false || true", ""), Value::Bool(true));
    assert_eq!(exec_or_panic("true && false", ""), Value::Bool(false));
    assert_eq!(exec_or_panic("false && true", ""), Value::Bool(false));

    // Empty string is falsy
    assert_eq!(
        exec_or_panic("\"\" || \"colin\"", ""),
        Value::String(String::from("colin"))
    );
    assert_eq!(
        exec_or_panic("\"colin\" || \"\"", ""),
        Value::String(String::from("colin"))
    );

    // Zero is falsy
    assert_eq!(exec_or_panic("0 || 5", ""), Value::Number(5.0));
    assert_eq!(exec_or_panic("5 || 0", ""), Value::Number(5.0));

    // OR has precedence over AND
    assert_eq!(
        exec_or_panic("true && true || false && false", ""),
        Value::Bool(true)
    );
    assert_eq!(
        exec_or_panic("true && (true || false) && false", ""),
        Value::Bool(false)
    );
}

#[test]
fn test_expansion_operator() {
    assert_eq!(
        // TODO: fix this, it doesn't work
        exec_or_panic(
            r#"
x = {
    name = "sir"
    message = "hello, " + name
}
out = x { 
    name = "colin" 
}
out.message
"#,
            ""
        ),
        Value::String(String::from("hello, colin")),
    );
}

#[test]
fn test_arrays() {
    assert_eq!(
        exec_or_panic("[1,2,3]", ""),
        Value::Array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ])
    );

    assert_eq!(
        exec_or_panic("[1+1,2+2,3+3]", ""),
        Value::Array(vec![
            Value::Number(2.0),
            Value::Number(4.0),
            Value::Number(6.0),
        ])
    );

    assert_eq!(
        exec_or_panic(
            "
            x = 5.0
            y = [x, x*x, x*x*x]
            y
            ",
            ""
        ),
        Value::Array(vec![
            Value::Number(5.0),
            Value::Number(25.0),
            Value::Number(125.0),
        ])
    );
}

#[test]
fn test_resolution() {
    assert_eq!(
        exec_or_panic(
            r#"
a = {
    _zzz = "test"
}
b = {
    c = a {
        t = 3.0
    }
}
b.c
            "#,
            ""
        ),
        Value::Dictionary(crate::Dictionary {
            kv_pairs: vec![
                (String::from("_zzz"), Value::String(String::from("test"))),
                (String::from("t"), Value::Number(3.0)),
            ]
        })
    );
}
