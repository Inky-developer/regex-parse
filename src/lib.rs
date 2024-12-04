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

    #[test]
    fn test_readme_example_1() {
        let input = "2024-12-15";
        let year: u32;
        let month: u32;
        let day: u32;
        re_parse!("{year}-{month}-{day}", input);
        assert_eq!(year, 2024);
        assert_eq!(month, 12);
        assert_eq!(day, 15);
    }

    #[test]
    fn test_readme_example_2() {
        let inputs = ["1 2", "3      4"];
        let parsed_inputs = inputs.map(|input| {
            let first_number: u32;
            let second_number: u32;
            re_parse!("{first_number} +{second_number}", input);
            (first_number, second_number)
        });
        assert_eq!(parsed_inputs, [(1, 2), (3, 4)]);
    }
}
