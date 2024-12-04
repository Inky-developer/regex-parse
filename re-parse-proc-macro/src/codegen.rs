use crate::dfa::{Dfa, DfaIndex};
use crate::{Map, Set};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::Expr;

pub struct Codegen {
    pub dfa: Dfa,
    pub expression: Expr,
}

impl Codegen {
    pub fn generate(self) -> TokenStream {
        let variables = self.collect_variables();
        let variable_idents = variables
            .iter()
            .enumerate()
            .map(|(index, _)| Ident::new(&format!("__var_{index}"), Span::mixed_site()))
            .collect::<Vec<_>>();
        let variable_map = variables
            .iter()
            .zip(variable_idents.iter())
            .map(|(var, ident)| (var.to_string(), ident.clone()))
            .collect::<Map<_, _>>();

        let states = self.collect_states();
        let internal_states = states.values();
        let initial_state = &states[&self.dfa.root];

        let state_branches = self.collect_state_branches(&states, &variable_map);
        let state_terminations = self.collect_state_terminations(&states, &variable_map);

        let expr = &self.expression;

        quote! {
            {
                #(let mut #variable_idents = 0_usize..0;)*

                enum __State {
                    #(#internal_states),*
                }

                let __initial_input = #expr;
                let mut __input = __initial_input.char_indices();
                let mut __variable_start = 0_usize;

                let mut __state = __State::#initial_state;
                loop {
                    let Some((__byte_index, __next_char)) = __input.next() else {
                        match __state {
                            #(#state_terminations),*
                        }
                    };
                    match __state {
                        #(#state_branches),*
                    }
                }

                #(#variables = __initial_input[#variable_idents].parse().unwrap();)*
            }
        }
    }

    fn collect_state_terminations(
        &self,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Ident>,
    ) -> Vec<TokenStream> {
        states
            .iter()
            .map(|(dfa_idx, internal_name)| {
                self.collect_state_termination(*dfa_idx, internal_name, variables)
            })
            .collect()
    }

    fn collect_state_termination(
        &self,
        dfa_idx: DfaIndex,
        internal_name: &Ident,
        variables: &Map<String, Ident>,
    ) -> TokenStream {
        let state = &self.dfa.nodes[dfa_idx];

        let panic_message = format!("Unexpected end of input ({internal_name})");

        let termination = match (state.is_accepting, &state.variable) {
            (true, Some(var)) => {
                let internal_var = &variables[var];
                quote! {
                    {
                        #internal_var = __variable_start..__initial_input.len();
                        break;
                    }
                }
            }
            (true, None) => quote! { break, },
            (false, _) => quote! {panic!(#panic_message)},
        };

        quote! {
            __State::#internal_name => #termination
        }
    }

    fn collect_state_branches(
        &self,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Ident>,
    ) -> Vec<TokenStream> {
        states
            .iter()
            .map(|(dfa_idx, internal_name)| {
                self.collect_state_branch(*dfa_idx, internal_name, states, variables)
            })
            .collect()
    }

    fn collect_state_branch(
        &self,
        dfa_idx: DfaIndex,
        internal_name: &Ident,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Ident>,
    ) -> TokenStream {
        let state = &self.dfa.nodes[dfa_idx];

        let default_edge = match state.edges.default {
            Some(target) => (
                quote! {_},
                self.make_state_transition(dfa_idx, target, states, variables),
            ),
            None => (quote! {char}, quote! {panic!("Invalid character: {char}")}),
        };
        let (patterns, branches): (Vec<TokenStream>, Vec<TokenStream>) = state
            .edges
            .edges
            .iter()
            .map(|(char, idx)| (quote! {#char}, *idx))
            .map(|(pattern, target)| {
                (
                    pattern,
                    self.make_state_transition(dfa_idx, target, states, variables),
                )
            })
            .chain(std::iter::once(default_edge))
            .unzip();

        quote! {
            __State::#internal_name => {
                match __next_char {
                    #(#patterns => #branches),*
                }
            }
        }
    }

    fn make_state_transition(
        &self,
        current_idx: DfaIndex,
        target_idx: DfaIndex,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Ident>,
    ) -> TokenStream {
        let current_state = &self.dfa.nodes[current_idx];
        let target_state = &self.dfa.nodes[target_idx];

        let variable_update = match (&current_state.variable, &target_state.variable) {
            (None, Some(_)) => Some(quote! {
                __variable_start = __byte_index;
            }),
            (Some(var), None) => {
                let internal_var = &variables[var];
                Some(quote! {
                    #internal_var = __variable_start..__byte_index;
                })
            }
            _ => None,
        }
        .into_iter();

        let next_state = &states[&target_idx];

        quote! {
            {
                #(#variable_update)*
                __state = __State::#next_state;
            }
        }
    }

    fn collect_variables(&self) -> Vec<Ident> {
        let mut variables = Set::default();
        for node_idx in self.dfa.iter() {
            let node = &self.dfa.nodes[node_idx];
            if let Some(variable) = &node.variable {
                variables.insert(Ident::new(&variable, Span::call_site()));
            }
        }

        variables.into_iter().collect()
    }

    fn collect_states(&self) -> Map<DfaIndex, Ident> {
        self.dfa
            .iter()
            .enumerate()
            .map(|(index, dfa_idx)| {
                (
                    dfa_idx,
                    Ident::new(&format!("State_{index}"), Span::mixed_site()),
                )
            })
            .collect()
    }
}
