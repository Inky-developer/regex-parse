use crate::arena::{Arena, ArenaIndex};
use crate::regex::{Regex, RegexArena, RegexNode, RegexNodeIndex, RegexPattern, RegexVariable};
use crate::util::FloodFill;
use crate::Set;

pub type NfaArena = Arena<NfaNode>;
pub type NfaIndex = ArenaIndex<NfaNode>;

#[derive(Debug)]
pub struct Nfa {
    pub root: NfaIndex,
    pub nodes: NfaArena,
}

impl From<Regex> for Nfa {
    fn from(value: Regex) -> Self {
        let Regex { arena, root } = value;
        let mut nodes = NfaArena::default();
        let root_node = nodes.add(NfaNode::EPSILON);
        let target_node = convert_regex_node(&mut nodes, &arena, root, root_node);
        nodes[target_node].is_accepting = true;

        check_variables(&nodes);

        Nfa {
            nodes,
            root: root_node,
        }
    }
}

fn check_variables(nodes: &NfaArena) {
    let mut visited_variables = Set::default();
    for node in nodes.iter() {
        if let NfaNodeKind::Variable(RegexVariable { name, .. }) = &nodes[node].kind {
            if visited_variables.contains(name) {
                panic!(
                    "(TODO MAKE ERROR) Variable \"{}\" is already declared",
                    name
                );
            }
            visited_variables.insert(name.clone());
        }
    }
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
    use crate::parser::ParseError;
    use crate::regex::Regex;

    fn parse(source: &str) -> Result<Nfa, ParseError> {
        let regex = Regex::from_str(source)?;
        Ok(Nfa::from(regex))
    }

    #[test]
    fn test_regex_to_nfa() {
        insta::assert_debug_snapshot!(parse("A"));
        insta::assert_debug_snapshot!(parse("A|B|C"));
        insta::assert_debug_snapshot!(parse("A?b*c"));
        insta::assert_debug_snapshot!(parse(".{var}."));
        insta::assert_debug_snapshot!(parse(".+;"));
    }
}
