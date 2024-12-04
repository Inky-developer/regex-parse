//! For detailed documentation, look at [re_parse]
#![doc=include_str!("../README.md")]

pub use re_parse_proc_macro::re_parse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let a: u32;
        let b: u32;
        re_parse!("{a} {b}", "1 2");
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }
}
