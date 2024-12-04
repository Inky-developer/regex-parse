use re_parse_proc_macro::re_parse;

fn main() {
    let var: u32;
    re_parse!("{var}B{var}", "111B222");
    let _ = var;
}