use crate::arena::{Arena, ArenaIndex};
use crate::nfa::{Nfa, NfaEdge, NfaIndex, NfaNodeKind};
use crate::regex::{RegexPattern, RegexVariable};
use crate::util::FloodFill;
use crate::{Map, Set};
use std::collections::HashSet;
use thiserror::Error;

pub type DfaArena = Arena<DfaNode>;
pub type DfaIndex = ArenaIndex<DfaNode>;

#[derive(Debug, Error)]
pub enum DfaError {
    #[error("Ambiguous variables: {} collides with {}. Make sure that variables are always separated by a character, so it is possible to tell them apart.", first, second)]
    AmbiguousVariables { first: String, second: String },
}

#[derive(Debug)]
pub struct Dfa {
    pub root: DfaIndex,
    pub nodes: DfaArena,
}

impl Dfa {
    pub fn iter(&self) -> impl Iterator<Item = DfaIndex> + use<'_> {
        <Self as FloodFill>::iter(self, self.root)
    }
}

impl TryFrom<Nfa> for Dfa {
    type Error = DfaError;
    fn try_from(nfa: Nfa) -> Result<Self, DfaError> {
        let mut builder = DfaBuilder::default();
        let root_group = expand_group(&nfa, &[nfa.root]);
        builder.pending_nodes.insert(root_group.clone());

        while let Some(group) = builder.pending_nodes.iter().next() {
            let group = group.clone();
            builder.pending_nodes.remove(&group);

            builder.compute_group(&nfa, group)?;
        }

        builder.dedup();

        let root = builder.nfa_to_dfa[&root_group];
        Ok(Dfa {
            root,
            nodes: builder.nodes,
        })
    }
}

impl FloodFill for Dfa {
    type Item = DfaIndex;

    fn get_neighbors(&self, item: &Self::Item) -> impl Iterator<Item = Self::Item> {
        let edges = &self.nodes[*item].edges;
        edges
            .default
            .iter()
            .copied()
            .chain(edges.edges.values().copied())
    }
}

#[derive(Debug, Default)]
pub struct DfaBuilder {
    nodes: DfaArena,
    nfa_to_dfa: Map<Vec<NfaIndex>, DfaIndex>,
    pending_nodes: Set<Vec<NfaIndex>>,
}

impl DfaBuilder {
    fn dedup(&mut self) {
        let mut fixed_duplicates = HashSet::new();
        loop {
            let mut visited_nodes: Vec<DfaIndex> = Vec::new();
            let mut duplicates: Vec<(DfaIndex, DfaIndex)> = Vec::new();

            let valid_nodes = self
                .nodes
                .iter()
                .filter(|idx| !fixed_duplicates.contains(idx));
            for node_idx in valid_nodes {
                let node = &self.nodes[node_idx];
                let other_node_index = visited_nodes
                    .iter()
                    .find(|other| &self.nodes[**other] == node);
                if let Some(idx) = other_node_index {
                    duplicates.push((node_idx, *idx));
                } else {
                    visited_nodes.push(node_idx);
                }
            }

            if duplicates.is_empty() {
                break;
            }

            for (previous, new) in duplicates {
                fixed_duplicates.insert(previous);
                for (_, node) in self.nodes.iter_mut() {
                    node.edges.replace(previous, new);
                }
            }
        }
    }

    fn insert(&mut self, key: Vec<NfaIndex>, node: DfaNode) -> DfaIndex {
        if let Some(idx) = self.nfa_to_dfa.get(&key) {
            self.nodes[*idx] = node;
            return *idx;
        }

        let idx = self.nodes.add(node);
        self.nfa_to_dfa.insert(key, idx);
        idx
    }

    fn entry(&mut self, key: Vec<NfaIndex>) -> DfaIndex {
        if let Some(idx) = self.nfa_to_dfa.get(&key) {
            return *idx;
        }

        let node = DfaNode::default();
        self.pending_nodes.insert(key.clone());
        self.insert(key, node)
    }

    fn compute_group(&mut self, nfa: &Nfa, group: Vec<NfaIndex>) -> Result<(), DfaError> {
        let edges = DfaEdges::from_nfa_group(self, nfa, &group);
        let is_accepting = group
            .iter()
            .copied()
            .any(|nfa_idx| nfa.nodes[nfa_idx].is_accepting);
        let variable = self.compute_group_variable(nfa, &group)?;

        self.insert(
            group,
            DfaNode {
                is_accepting,
                variable,
                edges,
            },
        );
        Ok(())
    }

    fn compute_group_variable(
        &self,
        nfa: &Nfa,
        group: &[NfaIndex],
    ) -> Result<Option<RegexVariable>, DfaError> {
        let mut variable = None;

        for nfa_idx in group.iter().copied() {
            let NfaNodeKind::Variable(var) = &nfa.nodes[nfa_idx].kind else {
                continue;
            };

            match variable {
                None => variable = Some(var.clone()),
                Some(RegexVariable {
                    name: other_var, ..
                }) => {
                    return Err(DfaError::AmbiguousVariables {
                        first: other_var,
                        second: var.name.clone(),
                    })
                }
            }
        }

        Ok(variable)
    }
}

fn get_non_epsilon_edges(nfa: &Nfa, group: &[NfaIndex]) -> Vec<(RegexPattern, NfaIndex)> {
    let mut edges: Vec<(RegexPattern, NfaIndex)> = Vec::new();
    for node_idx in group {
        let node = &nfa.nodes[*node_idx];
        for edge_idx in &node.edges {
            let edge = &nfa.nodes[*edge_idx];
            if let NfaEdge::Pattern(pattern) = &edge.edge_kind {
                edges.push((*pattern, *edge_idx))
            }
        }
    }
    edges
}

fn expand_group(nfa: &Nfa, group: &[NfaIndex]) -> Vec<NfaIndex> {
    let mut nodes = Set::default();
    for idx in group.iter().copied() {
        nodes.extend(get_connected_nodes(nfa, idx));
    }

    let mut result = nodes.into_iter().collect::<Vec<_>>();
    result.sort();
    result
}

fn get_connected_nodes(nfa: &Nfa, idx: NfaIndex) -> Vec<NfaIndex> {
    let mut nodes: Set<NfaIndex> = Set::default();
    let mut pending_nodes: Set<NfaIndex> = Set::default();

    pending_nodes.insert(idx);
    while let Some(node) = pending_nodes.iter().copied().next() {
        pending_nodes.remove(&node);
        nodes.insert(node);

        pending_nodes.extend(
            nfa.nodes[node]
                .edges
                .iter()
                .copied()
                .filter(|edge| nfa.nodes[*edge].edge_kind.is_epsilon()),
        )
    }

    let mut result: Vec<NfaIndex> = nodes.into_iter().collect();
    result.sort();
    result
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct DfaNode {
    pub is_accepting: bool,
    pub variable: Option<RegexVariable>,
    pub edges: DfaEdges,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct DfaEdges {
    pub default: Option<DfaIndex>,
    pub edges: Map<char, DfaIndex>,
}

impl DfaEdges {
    fn replace(&mut self, old_target: DfaIndex, new_target: DfaIndex) {
        let DfaEdges { default, edges } = self;
        if *default == Some(old_target) {
            *default = Some(new_target);
        }

        for edge in edges.values_mut() {
            if *edge == old_target {
                *edge = new_target;
            }
        }
    }

    fn from_nfa_group(dfa: &mut DfaBuilder, nfa: &Nfa, group: &[NfaIndex]) -> Self {
        let edges = get_non_epsilon_edges(nfa, group);

        let mut default_edges: Vec<NfaIndex> = Vec::new();
        let mut lazy_default_edges: Vec<NfaIndex> = Vec::new();

        let mut edge_map: Map<char, Vec<NfaIndex>> = Map::default();
        for (edge_pattern, target_idx) in edges.iter().copied() {
            match edge_pattern {
                RegexPattern::Char(char) => edge_map.entry(char).or_default().push(target_idx),
                RegexPattern::Range(start, end) => {
                    for char in start..=end {
                        edge_map.entry(char).or_default().push(target_idx);
                    }
                }
                RegexPattern::AnyChar => default_edges.push(target_idx),
                RegexPattern::AnyCharLazy => lazy_default_edges.push(target_idx),
            }
        }

        // Since a default edge can be any char, it also has to be added to each value in the edge map now.
        for targets in edge_map.values_mut() {
            targets.extend(default_edges.iter().copied());
            targets.sort_unstable();
            targets.dedup();
        }

        // If there is a default_edge, it will overwrite the lazy-default edge completely.
        if default_edges.is_empty() {
            default_edges = lazy_default_edges;
        }
        default_edges.sort_unstable();
        default_edges.dedup();

        let default_edge_idx = if default_edges.is_empty() {
            None
        } else {
            Some(dfa.entry(expand_group(nfa, &default_edges)))
        };
        let edge_indices = edge_map
            .into_iter()
            .map(|(key, value)| (key, dfa.entry(expand_group(nfa, &value))))
            .collect();
        DfaEdges {
            default: default_edge_idx,
            edges: edge_indices,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dfa::Dfa;
    use crate::nfa::Nfa;
    use crate::regex::Regex;
    use crate::ProcMacroErrorKind;

    fn parse(input: &str) -> Result<Dfa, ProcMacroErrorKind> {
        let regex = Regex::from_str(input)?;
        let nfa = Nfa::try_from(regex)?;
        let dfa = Dfa::try_from(nfa)?;
        Ok(dfa)
    }

    #[test]
    fn test_nfa_to_dfa() {
        insta::assert_debug_snapshot!(parse("A"));
        insta::assert_debug_snapshot!(parse("AB"));
        insta::assert_debug_snapshot!(parse("A?B"));
        insta::assert_debug_snapshot!(parse("A?A"));
        insta::assert_debug_snapshot!(parse("A?b*c"));
        insta::assert_debug_snapshot!(parse("{foo}"));
        insta::assert_debug_snapshot!(parse("A{foo}B+{bar}"));
        insta::assert_debug_snapshot!(parse("[a-e]"));
        insta::assert_debug_snapshot!(parse(".{var}."));
    }

    #[test]
    fn test_simplify() {
        insta::assert_debug_snapshot!(parse(".+;"));
    }

    #[test]
    fn test_simplify_dfa() {
        // Without simplification, this is a relatively big state machine
        // With simplification, only two states are used.
        insta::assert_debug_snapshot!(parse("([abc]\\s*)*"));
    }

    #[test]
    fn test_nfa_to_dfa_ambiguous_variable() {
        insta::assert_debug_snapshot!(parse("A{foo}B?{bar}"));
    }
}
