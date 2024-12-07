use crate::arena::{Arena, ArenaIndex};
use crate::nfa::{Nfa, NfaEdge, NfaIndex, NfaNodeKind};
use crate::regex::{RegexPattern, RegexVariable};
use crate::util::FloodFill;
use crate::{Map, Set};

pub type DfaArena = Arena<DfaNode>;
pub type DfaIndex = ArenaIndex<DfaNode>;

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

impl From<Nfa> for Dfa {
    fn from(nfa: Nfa) -> Self {
        let mut builder = DfaBuilder::default();
        let hull = calculate_epsilon_hull(&nfa);
        let root_group = hull[&nfa.root].clone();
        builder.pending_nodes.extend(hull.into_values());

        while let Some(group) = builder.pending_nodes.iter().next() {
            let group = group.clone();
            builder.pending_nodes.remove(&group);

            builder.compute_group(&nfa, group);
        }

        let root = builder.nfa_to_dfa[&root_group];
        Dfa {
            root,
            nodes: builder.nodes,
        }
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

    fn compute_group(&mut self, nfa: &Nfa, group: Vec<NfaIndex>) {
        let edges = DfaEdges::from_nfa_group(self, nfa, &group);
        let is_accepting = group
            .iter()
            .copied()
            .any(|nfa_idx| nfa.nodes[nfa_idx].is_accepting);
        let variable = self.compute_group_variable(nfa, &group);

        self.insert(
            group,
            DfaNode {
                is_accepting,
                variable,
                edges,
            },
        );
    }

    fn compute_group_variable(&self, nfa: &Nfa, group: &[NfaIndex]) -> Option<RegexVariable> {
        let mut variable = None;

        for nfa_idx in group.iter().copied() {
            let NfaNodeKind::Variable(var) = &nfa.nodes[nfa_idx].kind else {
                continue;
            };

            match variable {
                None => variable = Some(var.clone()),
                Some(RegexVariable {
                    name: other_var, ..
                }) => panic!(
                    "(TODO THROW ERROR) ambiguous variables: {other_var} collides with {}",
                    &var.name
                ),
            }
        }

        variable
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

fn calculate_epsilon_hull(nfa: &Nfa) -> Map<NfaIndex, Vec<NfaIndex>> {
    let mut hull = Map::default();

    for idx in nfa.iter() {
        hull.insert(idx, get_connected_nodes(nfa, idx));
    }

    hull
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

#[derive(Debug, Default)]
pub struct DfaNode {
    pub is_accepting: bool,
    pub variable: Option<RegexVariable>,
    pub edges: DfaEdges,
}

#[derive(Debug, Default)]
pub struct DfaEdges {
    pub default: Option<DfaIndex>,
    pub edges: Map<char, DfaIndex>,
}

impl DfaEdges {
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
    use crate::parser::ParseError;
    use crate::regex::Regex;

    fn parse(input: &str) -> Result<Dfa, ParseError> {
        let regex = Regex::from_str(input)?;
        let nfa = Nfa::from(regex);
        Ok(Dfa::from(nfa))
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
    #[should_panic(expected = "(TODO THROW ERROR) ambiguous variables: foo collides with bar")]
    fn test_nfa_to_dfa_ambiguous_variable() {
        parse("A{foo}B?{bar}").unwrap();
    }
}
