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
/// # Example
/// ```rust
/// # use re_parse_proc_macro::re_parse;
/// let name: String;
/// let score: f32;
/// re_parse!("The score of {name} is {score}", "The score of user is 55.8");
/// assert_eq!(name, "user");
/// assert_eq!(score, 55.8);
/// ```
///
/// # Efficiency
/// The macro compiles the pattern into a state-machine which executes in linear time, so it should be very efficient.
#[proc_macro]
pub fn re_parse(input: TokenStream) -> TokenStream {
    let ReParseInput { regex, expression } = parse_macro_input!(input as ReParseInput);

    let regex = Regex::from_str(&regex.value()).unwrap();
    let nfa = Nfa::from(regex);
    let dfa = Dfa::from(nfa);
    let codegen = Codegen { dfa, expression };
    let result = codegen.generate();
    result.into()
}