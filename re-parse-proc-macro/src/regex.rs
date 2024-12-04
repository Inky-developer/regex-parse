use crate::arena::{Arena, ArenaIndex};
use crate::parser::{ParseError, RegexParser};
use crate::tokenizer::tokenize;
use std::fmt::{Debug, Display, Formatter, Write};

pub type RegexArena = Arena<RegexNode>;

pub type RegexNodeIndex = ArenaIndex<RegexNode>;

pub struct Regex {
    pub arena: RegexArena,
    pub root: RegexNodeIndex,
}

impl Regex {
    pub fn from_str(input: &str) -> Result<Self, ParseError> {
        RegexParser::parse(tokenize(input))
    }
}

impl Display for Regex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(
            &RegexDisplay {
                arena: &self.arena,
                node_idx: self.root,
            },
            f,
        )
    }
}

impl Debug for Regex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(
            &RegexDisplay {
                arena: &self.arena,
                node_idx: self.root,
            },
            f,
        )
    }
}

#[derive(Debug)]
pub enum RegexNode {
    And(Vec<RegexNodeIndex>),
    Or(Vec<RegexNodeIndex>),
    Literal(RegexPattern),
    Variable(String),
    ZeroOrOne(RegexNodeIndex),
    Many(RegexNodeIndex),
    OneOrMore(RegexNodeIndex),
}

#[derive(Debug, Clone, Copy)]
pub enum RegexPattern {
    Char(char),
    AnyChar,
}

pub struct RegexDisplay<'arena> {
    arena: &'arena RegexArena,
    node_idx: RegexNodeIndex,
}

impl RegexDisplay<'_> {
    fn node(&self, node_idx: RegexNodeIndex) -> Self {
        Self {
            arena: self.arena,
            node_idx,
        }
    }
}

impl<'arena> Display for RegexDisplay<'arena> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let node = &self.arena[self.node_idx];

        match node {
            RegexNode::And(nodes) => {
                for node in nodes {
                    Display::fmt(&self.node(*node), f)?;
                }
            }
            RegexNode::Or(nodes) => {
                for (index, node) in nodes.iter().enumerate() {
                    Display::fmt(&self.node(*node), f)?;
                    if index + 1 < nodes.len() {
                        f.write_char('|')?;
                    }
                }
            }
            RegexNode::Literal(pat) => match pat {
                RegexPattern::Char(char) => f.write_char(*char)?,
                RegexPattern::AnyChar => f.write_char('.')?,
            },
            RegexNode::Variable(var) => write!(f, "{{{var}}}")?,
            RegexNode::ZeroOrOne(node) => {
                Display::fmt(&self.node(*node), f)?;
                f.write_char('?')?;
            }
            RegexNode::Many(node) => {
                Display::fmt(&self.node(*node), f)?;
                f.write_char('*')?;
            }
            RegexNode::OneOrMore(node) => {
                Display::fmt(&self.node(*node), f)?;
                f.write_char('+')?;
            }
        }

        Ok(())
    }
}

impl<'arena> Debug for RegexDisplay<'arena> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let node = &self.arena[self.node_idx];
        match node {
            RegexNode::And(nodes) => {
                let mut tuple = f.debug_tuple("And");
                for node in nodes {
                    tuple.field(&self.node(*node));
                }
                tuple.finish()?;
            }
            RegexNode::Or(nodes) => {
                let mut tuple = f.debug_tuple("Or");
                for node in nodes {
                    tuple.field(&self.node(*node));
                }
                tuple.finish()?;
            }
            RegexNode::Literal(literal) => f.debug_tuple("Literal").field(literal).finish()?,
            RegexNode::Variable(var) => f.debug_tuple("Variable").field(var).finish()?,
            RegexNode::ZeroOrOne(child) => f
                .debug_tuple("ZeroOrOne")
                .field(&self.node(*child))
                .finish()?,
            RegexNode::Many(child) => f.debug_tuple("Many").field(&self.node(*child)).finish()?,
            RegexNode::OneOrMore(child) => f
                .debug_tuple("OneOrMore")
                .field(&self.node(*child))
                .finish()?,
        }

        Ok(())
    }
}
