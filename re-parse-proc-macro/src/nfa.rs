use crate::arena::{Arena, ArenaIndex};
use crate::regex::{Regex, RegexArena, RegexNode, RegexNodeIndex, RegexPattern, RegexVariable};
use crate::util::FloodFill;
use crate::Set;
use thiserror::Error;

pub type NfaArena = Arena<NfaNode>;
pub type NfaIndex = ArenaIndex<NfaNode>;

#[derive(Error, Debug)]
pub enum NfaError {
    #[error("The variable {} is already declared. Capturing a variable twice is not supported right now.", name)]
    DuplicateVariable { name: String },
}

#[derive(Debug)]
pub struct Nfa {
    pub root: NfaIndex,
    pub nodes: NfaArena,
}

impl TryFrom<Regex> for Nfa {
    type Error = NfaError;

    fn try_from(value: Regex) -> Result<Self, NfaError> {
        let Regex { arena, root } = value;
        let mut nodes = NfaArena::default();
        let root_node = nodes.add(NfaNode::EPSILON);
        let target_node = convert_regex_node(&mut nodes, &arena, root, root_node);
        nodes[target_node].is_accepting = true;

        check_variables(&nodes)?;

        Ok(Nfa {
            nodes,
            root: root_node,
        })
    }
}

fn check_variables(nodes: &NfaArena) -> Result<(), NfaError> {
    let mut visited_variables = Set::default();
    for node in nodes.iter() {
        if let NfaNodeKind::Variable(RegexVariable { name, .. }) = &nodes[node].kind {
            if visited_variables.contains(name) {
                return Err(NfaError::DuplicateVariable { name: name.clone() });
            }
            visited_variables.insert(name.clone());
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct NfaNode {
    pub edges: Vec<NfaIndex>,
    pub edge_kind: NfaEdge,
    pub kind: NfaNodeKind,
    pub is_accepting: bool,
}

impl NfaNode {
    pub const EPSILON: Self = Self {
        edges: Vec::new(),
        edge_kind: NfaEdge::Epsilon,
        kind: NfaNodeKind::Simple,
        is_accepting: false,
    };
}

#[derive(Debug)]
pub enum NfaNodeKind {
    Simple,
    Variable(RegexVariable),
}

#[derive(Debug)]
pub enum NfaEdge {
    Epsilon,
    Pattern(RegexPattern),
}

impl NfaEdge {
    pub fn is_epsilon(&self) -> bool {
        matches!(self, NfaEdge::Epsilon)
    }
}

fn convert_regex_node(
    arena: &mut NfaArena,
    regex_arena: &RegexArena,
    node: RegexNodeIndex,
    predecessor: NfaIndex,
) -> NfaIndex {
    let node = &regex_arena[node];
    match node {
        RegexNode::And(nodes) => {
            let mut last_node = predecessor;
            for node in nodes {
                let new_node = convert_regex_node(arena, regex_arena, *node, last_node);
                last_node = new_node;
            }
            last_node
        }
        RegexNode::Or(nodes) => {
            let target_node = arena.add(NfaNode::EPSILON);
            for node in nodes {
                let new_node = convert_regex_node(arena, regex_arena, *node, predecessor);
                arena.connect(new_node, target_node);
            }
            target_node
        }
        RegexNode::Literal(pattern) => arena.add_after(
            predecessor,
            NfaNode {
                edges: Vec::new(),
                edge_kind: NfaEdge::Pattern(*pattern),
                kind: NfaNodeKind::Simple,
                is_accepting: false,
            },
        ),
        RegexNode::Variable(var) => {
            let node = arena.add_after(
                predecessor,
                NfaNode {
                    edges: Vec::new(),
                    edge_kind: NfaEdge::Pattern(RegexPattern::AnyCharLazy),
                    kind: NfaNodeKind::Variable(var.clone()),
                    is_accepting: false,
                },
            );
            arena.connect(node, node);
            node
        }
        RegexNode::ZeroOrOne(child) => {
            let target_node = arena.add(NfaNode::EPSILON);
            arena.connect(predecessor, target_node);
            let new_node = convert_regex_node(arena, regex_arena, *child, predecessor);
            arena.connect(new_node, target_node);
            target_node
        }
        RegexNode::Many(child) => {
            let iteration_node = arena.add(NfaNode::EPSILON);
            arena.connect(predecessor, iteration_node);
            let target_node = arena.add(NfaNode::EPSILON);
            arena.connect(predecessor, target_node);
            let new_node = convert_regex_node(arena, regex_arena, *child, iteration_node);
            arena.connect(new_node, iteration_node);
            arena.connect(new_node, target_node);
            target_node
        }
        RegexNode::OneOrMore(child) => {
            let iteration_node = arena.add(NfaNode::EPSILON);
            arena.connect(predecessor, iteration_node);
            let target_node = arena.add(NfaNode::EPSILON);
            let new_node = convert_regex_node(arena, regex_arena, *child, iteration_node);
            arena.connect(new_node, iteration_node);
            arena.connect(new_node, target_node);
            target_node
        }
    }
}

impl NfaArena {
    fn connect(&mut self, source: NfaIndex, target: NfaIndex) {
        self[source].edges.push(target);
    }

    fn add_after(&mut self, source: NfaIndex, new_node: NfaNode) -> NfaIndex {
        let idx = self.add(new_node);
        self.connect(source, idx);
        idx
    }
}

impl FloodFill for Nfa {
    type Item = NfaIndex;

    fn get_neighbors(&self, item: &Self::Item) -> impl Iterator<Item = Self::Item> {
        self.nodes[*item].edges.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::nfa::Nfa;
    use crate::regex::Regex;
    use crate::ProcMacroErrorKind;

    fn parse(source: &str) -> Result<Nfa, ProcMacroErrorKind> {
        let regex = Regex::from_str(source)?;
        let nfa = Nfa::try_from(regex)?;
        Ok(nfa)
    }

    #[test]
    fn test_regex_to_nfa() {
        insta::assert_debug_snapshot!(parse("A"));
        insta::assert_debug_snapshot!(parse("A|B|C"));
        insta::assert_debug_snapshot!(parse("A?b*c"));
        insta::assert_debug_snapshot!(parse(".{var}."));
        insta::assert_debug_snapshot!(parse(".+;"));
    }

    #[test]
    fn test_duplicate_variable() {
        insta::assert_debug_snapshot!(parse("{foo}bar{foo}"));
    }
}
