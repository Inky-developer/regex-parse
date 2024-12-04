use re_parse_proc_macro::re_parse;

fn main() {
    re_parse!("Foo{variable}B*{other_variable}C", "Foo111B222C")
}