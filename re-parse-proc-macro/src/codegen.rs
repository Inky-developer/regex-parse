use crate::dfa::{Dfa, DfaIndex};
use crate::regex::VariableKind;
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
            .map(|(var, ident)| {
                (
                    var.ident.to_string(),
                    Variable {
                        ident: ident.clone(),
                        kind: var.kind,
                    },
                )
            })
            .collect::<Map<_, _>>();

        let variable_setups = variable_map
            .values()
            .map(|var| self.quote_variable_setup(var));
        let variable_finalizers = variable_map
            .iter()
            .map(|(k, v)| self.quote_variable_finalizer(v, k));

        let states = self.collect_states();
        let internal_states = states.values();
        let initial_state = &states[&self.dfa.root];

        let state_branches = self.collect_state_branches(&states, &variable_map);
        let state_terminations = self.collect_state_terminations(&states, &variable_map);

        let expr = &self.expression;

        quote! {
            {
                #(#variable_setups)*

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

                #(#variable_finalizers)*
            }
        }
    }

    fn quote_variable_finalizer(&self, var: &Variable, name: &str) -> TokenStream {
        let ident = &var.ident;
        let original_ident = Ident::new(name, Span::call_site());
        match var.kind {
            VariableKind::Singular => {
                quote! { #original_ident = __initial_input[#ident].parse().unwrap();}
            }
            VariableKind::Multiple => {
                quote! { #original_ident = #ident.into_iter().map(|span| __initial_input[span].parse().unwrap()).collect(); }
            }
        }
    }

    fn quote_variable_setup(&self, var: &Variable) -> TokenStream {
        let ident = &var.ident;
        match var.kind {
            VariableKind::Singular => quote! { let mut #ident = 0_usize..0; },
            VariableKind::Multiple => quote! { let mut #ident = ::std::vec::Vec::new(); },
        }
    }

    fn collect_state_terminations(
        &self,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Variable>,
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
        variables: &Map<String, Variable>,
    ) -> TokenStream {
        let state = &self.dfa.nodes[dfa_idx];

        let panic_message = format!("Unexpected end of input ({internal_name})");

        let termination = match (state.is_accepting, &state.variable) {
            (true, Some(var)) => {
                let internal_var = &variables[&var.name];
                let update =
                    self.quote_update_variable(internal_var, quote! {__initial_input.len()});
                quote! {
                    {
                        #update;
                        break;
                    }
                }
            }
            (true, None) => quote! { break },
            (false, _) => quote! {panic!(#panic_message)},
        };

        quote! {
            __State::#internal_name => #termination
        }
    }

    fn quote_update_variable(&self, variable: &Variable, variable_end: TokenStream) -> TokenStream {
        let ident = &variable.ident;
        match variable.kind {
            VariableKind::Singular => quote! { #ident = __variable_start..#variable_end; },
            VariableKind::Multiple => {
                quote! { #ident.push(__variable_start..#variable_end); }
            }
        }
    }

    fn collect_state_branches(
        &self,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Variable>,
    ) -> Vec<TokenStream> {
        // Let's sort the states first to make it easier to read the macro expansion
        let mut sorted_states = states.iter().collect::<Vec<_>>();
        sorted_states.sort_unstable_by_key(|(_, name)| *name);

        sorted_states
            .iter()
            .map(|(dfa_idx, internal_name)| {
                self.collect_state_branch(**dfa_idx, internal_name, states, variables)
            })
            .collect()
    }

    fn collect_state_branch(
        &self,
        dfa_idx: DfaIndex,
        internal_name: &Ident,
        states: &Map<DfaIndex, Ident>,
        variables: &Map<String, Variable>,
    ) -> TokenStream {
        let state = &self.dfa.nodes[dfa_idx];

        let default_edge = match state.edges.default {
            Some(target) => (
                None,
                StateTransition::Valid {
                    target: states[&target].clone(),
                    variable_update: self.make_variable_update(dfa_idx, target, variables),
                },
            ),
            None => {
                let expected = if state.edges.edges.is_empty() {
                    vec!["End of input".to_string()]
                } else {
                    state.edges.edges.keys().copied().map(Into::into).collect()
                };
                (None, StateTransition::Invalid { expected })
            }
        };
        let initial_patterns = state
            .edges
            .edges
            .iter()
            .map(|(char, idx)| {
                (
                    Some(*char),
                    StateTransition::Valid {
                        target: states[idx].clone(),
                        variable_update: self.make_variable_update(dfa_idx, *idx, variables),
                    },
                )
            })
            .chain(std::iter::once(default_edge));

        let simplified_patterns = self.simplify_match(initial_patterns);

        quote! {
            __State::#internal_name => {
                match __next_char {
                    #(#simplified_patterns)*
                }
            }
        }
    }

    fn simplify_match(
        &self,
        patterns_and_transitions: impl Iterator<Item = (Option<char>, StateTransition)>,
    ) -> Vec<TokenStream> {
        let mut simplified: Map<StateTransition, Vec<Option<char>>> = Map::default();

        for (pattern, transition) in patterns_and_transitions {
            simplified
                .entry(transition.clone())
                .or_default()
                .push(pattern);
        }

        // Sort the patterns and transitions, so that the default pattern is always at the end
        let mut simplified: Vec<_> = simplified.into_iter().collect();
        simplified.sort_unstable_by_key(|(_, patterns)| patterns.iter().any(|it| it.is_none()));

        simplified
            .into_iter()
            .map(|(transition, patterns)| {
                let transition = transition.quote();
                if patterns.iter().any(|it| it.is_none()) {
                    quote! {_ => {
                        #transition
                    }}
                } else {
                    let chars = patterns.iter().map(|it| it.unwrap());
                    quote! {#(#chars)|* => #transition,}
                }
            })
            .collect()
    }

    fn make_variable_update(
        &self,
        current_idx: DfaIndex,
        target_idx: DfaIndex,
        variables: &Map<String, Variable>,
    ) -> VariableUpdate {
        let current_state = &self.dfa.nodes[current_idx];
        let target_state = &self.dfa.nodes[target_idx];

        match (&current_state.variable, &target_state.variable) {
            (None, Some(_)) => VariableUpdate::StartVariable,
            (Some(var), None) => VariableUpdate::EndVariable(variables[&var.name].clone()),
            _ => VariableUpdate::NoVariable,
        }
    }

    fn collect_variables(&self) -> Vec<Variable> {
        let mut variables = Set::default();
        for node_idx in self.dfa.iter() {
            let node = &self.dfa.nodes[node_idx];
            if let Some(variable) = &node.variable {
                let ident = Ident::new(&variable.name, Span::call_site());
                variables.insert(Variable {
                    ident,
                    kind: variable.kind,
                });
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

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
struct Variable {
    kind: VariableKind,
    ident: Ident,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum StateTransition {
    Invalid {
        expected: Vec<String>,
    },
    Valid {
        target: Ident,
        variable_update: VariableUpdate,
    },
}

impl StateTransition {
    fn quote(&self) -> TokenStream {
        match self {
            StateTransition::Invalid { expected } => {
                let message = match expected.as_slice() {
                    [single] => {
                        format!("Unexpected character {{__next_char}}. Expected '{single}'")
                    }
                    _ => format!(
                        "Unexpected character: {{__next_char}}. Expected one of: '{}'",
                        expected.join(", ")
                    ),
                };
                quote! {panic!(#message)}
            }
            StateTransition::Valid {
                target,
                variable_update,
            } => {
                let variable_update = variable_update.quote();
                quote! {{
                    #variable_update
                    __state = __State::#target;
                }}
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum VariableUpdate {
    NoVariable,
    StartVariable,
    EndVariable(Variable),
}

impl VariableUpdate {
    fn quote(&self) -> TokenStream {
        match self {
            VariableUpdate::NoVariable => quote! {},
            VariableUpdate::StartVariable => quote! {__variable_start = __byte_index;},
            VariableUpdate::EndVariable(Variable {
                kind: VariableKind::Singular,
                ident,
            }) => quote! {#ident = __variable_start..__byte_index;},
            VariableUpdate::EndVariable(Variable {
                kind: VariableKind::Multiple,
                ident,
            }) => quote! {#ident.push(__variable_start..__byte_index);},
        }
    }
}
