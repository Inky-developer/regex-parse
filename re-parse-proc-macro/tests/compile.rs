use re_parse_proc_macro::re_parse;

#[test]
fn test_compile_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/**/*.rs");
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
#[should_panic(expected = "Unexpected character 1. Expected 'End of input'")]
fn test_empty_fail() {
    re_parse!("", "1");
}

#[test]
#[should_panic(expected = "Unexpected character: D. Expected one of: 'A', 'B', 'C'")]
fn test_unexpected_character() {
    re_parse!("[ABC]*", "ABCD");
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
    re_parse!("A.*B.*;", "AAAAABBBB;");
}

#[test]
fn test_parse_vec_var() {
    let result: Vec<u32>;
    re_parse!(r"\[({result*},?)*\]", "[1,2,3]");
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_parse_var_in_loop() {
    let var: u32;
    re_parse!("({var})*", "1234");
    assert_eq!(var, 1234);
}

#[test]
fn test_parse_var_in_loop2() {
    let var: Vec<u32>;
    re_parse!("({var*},)*", "1,2,3,4,");
    assert_eq!(var, vec![1, 2, 3, 4]);
}

#[test]
fn test_parse_var_in_loop3() {
    let result: u32;
    let operands: Vec<u32>;
    re_parse!("{result}: ({operands*} ?)+", "3267: 81 40 27");
    assert_eq!(result, 3267);
    assert_eq!(operands, vec![81, 40, 27]);
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

#[test]
fn test_character_class() {
    let a: String;
    re_parse!("\\w+ {a}\\s?", "Hello World ");
    assert_eq!(a, "World");
}
