use re_parse_proc_macro::re_parse;

#[test]
fn test_compile_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}

#[test]
fn test_parse_simple() {
    let var: u32;
    re_parse!("{var}", "42");
    assert_eq!(var, 42);
}

#[test]
fn test_empty() {
    re_parse!("", "");
}

#[test]
#[should_panic(expected = "Invalid character: 1")]
fn test_empty_fail() {
    re_parse!("", "1");
}

#[test]
fn test_parse_text() {
    let var: u32;
    let var2: u32;

    re_parse!("A{var}B{var2}", "A1B2");
    assert_eq!(var, 1);
    assert_eq!(var2, 2);
}
#[test]
fn test_parse_regex() {
    let foo: u32;
    let bar: f32;

    re_parse!("A*{foo}B+{bar}", "AAA740BB5.0");
    assert_eq!(foo, 740);
    assert_eq!(bar, 5.0);
}

#[test]
fn test_parse_regex_2() {
    re_parse!("((Hello|World) )*", "Hello World World Hello Hello World ");
}

// FIXME: This test should probably be an error, at least when parsing into `Vec`s is supported
#[test]
fn test_parse_var_in_loop() {
    let var: u32;
    re_parse!("({var})*", "1234");
    assert_eq!(var, 1234);
}

// FIXME: This should parse into a `Vec<u32>`
#[test]
fn test_parse_var_in_loop2() {
    let var: u32;
    re_parse!("({var},)*", "1,2,3,4,");
    assert_eq!(var, 4);
}

#[test]
fn test_group() {
    for input in ["A", "B", "C", "D", "E", "F"] {
        re_parse!("[ABC]|[DEF]", input)
    }
}

#[test]
fn test_group_range() {
    for input in ["A", "B", "C", "D", "E", "F"] {
        re_parse!("[A-F]", input)
    }
}

#[test]
fn test_dot() {
    let var: u32;
    re_parse!(".{var}.", "123");
    assert_eq!(var, 2);
}

#[test]
fn test_precedence() {
    re_parse!("(abc|.)", "a");
}
