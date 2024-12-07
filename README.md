# Regex-Parse

A macro to parse simple inputs using regular expressions

[![Documentation](https://img.shields.io/badge/Documentation-blue)](https://inky-developer.github.io/regex-parse/regex_parse/)
[![Build status](https://github.com/Inky-developer/regex-parse/actions/workflows/check.yml/badge.svg)](https://github.com/Inky-developer/regex-parse/actions/workflows/check.yml)

## Installation

```shell
cargo add --git https://github.com/Inky-developer/regex-parse regex-parse
```

## Example

### Parsing a date:
```rust
use regex_parse::re_parse;

fn main() {
    let input = "2024-12-15";
    let year: u32;
    let month: u32;
    let day: u32;
    re_parse!("{year}\\-{month}\\-{day}", input);
    assert_eq!(year, 2024);
    assert_eq!(month, 12);
    assert_eq!(day, 15);
}
```

### Using regular expressions:

```rust
use regex_parse::re_parse;

fn main() {
    let inputs = ["1 2 3 4", "-5      6 -7"];
    let parsed_inputs = inputs.map(|input| {
        let numbers: Vec<i32>;
        re_parse!(r"({numbers*}\s*)*", input);
        numbers
    });
    assert_eq!(parsed_inputs, [vec![1, 2, 3, 4], vec![-5, 6, -7]]);
}
```

## Regex Features
- [x] literal text: `abcdef`
- [x] variables: `abc{var}def`
- [x] or: `a|b`
- [x] parenthesis: `(ab)|(cd)`
- [x] any character in group: `[abc]`
- [x] any character in range: `[a-z]`
- [ ] any character not in group: `[^abc]`
- [x] any character: `.`
- [x] any whitespace: `\s`
- [x] any digit: `\d`
- [x] any word: `\w`
- [x] zero or one: `a?`
- [x] zero or more: `a*`
- [x] one or more: `a+`
- [ ] exactly n: `a{3}`
- [ ] n or more: `a{3,}`
- [ ] between n and m: `a{3,6}`
