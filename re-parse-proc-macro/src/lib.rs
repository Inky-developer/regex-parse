mod arena;
mod codegen;
mod dfa;
mod nfa;
mod parser;
mod regex;
mod tokenizer;
mod util;

use crate::codegen::Codegen;
use crate::dfa::Dfa;
use crate::nfa::Nfa;
use crate::regex::Regex;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Expr, LitStr};

// Use non-std map and set implementations to make snapshot testing possible.
// std map and set implementations are not deterministic, which is required for that.
pub(crate) type Map<K, V> = fxhash::FxHashMap<K, V>;
pub(crate) type Set<K> = fxhash::FxHashSet<K>;

struct ReParseInput {
    regex: LitStr,
    expression: Expr,
}

impl Parse for ReParseInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let regex = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let expression = input.parse()?;
        Ok(Self { regex, expression })
    }
}

/// The main macro of this crate, which parses strings using regular expressions and can extract variables.
///
/// # Usage
/// `re_parse!(pattern: StrLiteral, value: &str);`
///
/// Any variables contained in `pattern` will be set after the macro has run.
/// For now, the macro will panic if the input cannot be parsed (TODO: Return error)
///
/// The pattern is a regular expression which can contain variable captures.
///
/// ## Variable Captures
/// - `{var_name}`: Captures a single variable of at least one character
/// - `{var_name*}`: Captures multiple (or zero) variables
///
/// ## Character Classes
/// `re_parse!` currently supports these character classes:
/// - `\s`: Any Whitespace (equivalent to `[\n\t\r ]`)
/// - `\d`: Any Digit (equivalent to `[0-9]`)
/// - `\w`: Any Word (equivalent to `[a-zA-Z0-0_]`)
///
/// # Example
///
/// ```rust
/// # use re_parse_proc_macro::re_parse;
/// let name: String;
/// let score: f32;
/// re_parse!("The score of {name} is {score}", "The score of user is 55.8");
/// assert_eq!(name, "user");
/// assert_eq!(score, 55.8);
/// ```
///
/// ## Multiple variables
/// ```rust
/// # use re_parse_proc_macro::re_parse;
/// let temperatures: Vec<f32>;
/// re_parse!(r"Temperatures: \[({temperatures*}\s*,?\s*)*\]", "Temperatures: [10.0, 9.0, 8.5, 8.0]");
/// assert_eq!(temperatures, vec![10.0, 9.0, 8.5, 8.0]);
/// ```
///
/// # Efficiency
/// The macro compiles the pattern into a state-machine which executes in linear time, so it should be very efficient.
#[proc_macro]
pub fn re_parse(input: TokenStream) -> TokenStream {
    let ReParseInput { regex, expression } = parse_macro_input!(input as ReParseInput);

    let result = re_parse_impl(regex, expression);
    result.into()
}

fn re_parse_impl(regex: LitStr, expression: Expr) -> proc_macro2::TokenStream {
    let regex = Regex::from_str(&regex.value()).unwrap();
    let nfa = Nfa::from(regex);
    let dfa = Dfa::from(nfa);
    let codegen = Codegen { dfa, expression };
    codegen.generate()
}

#[cfg(test)]
mod tests {
    use super::{re_parse_impl, ReParseInput};
    use quote::quote;

    fn test_re_parse(input: proc_macro2::TokenStream) -> String {
        let ReParseInput { regex, expression } = syn::parse2::<ReParseInput>(input).unwrap();
        let stream = re_parse_impl(regex, expression);
        let file_content = format!("fn main() {{ {stream} }}");
        let file = syn::parse_file(&file_content).unwrap();
        prettyplease::unparse(&file)
    }

    macro_rules! dbg_re_parse {
        ($($input:tt)*) => {test_re_parse(quote! {$($input)*})};
    }

    #[test]
    fn test_macro_expansion() {
        insta::assert_snapshot!(dbg_re_parse!("A", "A"));
        insta::assert_snapshot!(dbg_re_parse!("A+", "A"));
        insta::assert_snapshot!(dbg_re_parse!("({var*},)*", "1,2,3,4,"));
        insta::assert_snapshot!(dbg_re_parse!("([abc]\\s*)*", "A"));
        insta::assert_snapshot!(dbg_re_parse!("A.*B.*;", "AAABBB;"));
    }
}
