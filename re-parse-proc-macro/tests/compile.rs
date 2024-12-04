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