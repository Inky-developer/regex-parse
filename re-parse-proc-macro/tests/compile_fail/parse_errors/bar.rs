use re_parse_proc_macro::re_parse;

fn main() {
    re_parse!("a||B", "111B222");
}