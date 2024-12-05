use crate::regex::{Regex, RegexArena, RegexNode, RegexNodeIndex, RegexPattern};
use crate::tokenizer::{PostfixToken, Token};
use std::iter::Peekable;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token '}}'. Did you forget a '{{'?")]
    UnexpectedRightBrace,
    #[error("Unexpected token ')'. Did you forget a '('?")]
    UnexpectedRightParenthesis,
    #[error("Unexpected token ']'. Did you forget a '['?")]
    UnexpectedRightBracket,
    #[error("Unexpected token '-'. It is currently only supported in a group: `[a-z]`")]
    UnexpectedMinus,
    #[error("Unexpected postfix token: '{}'", got)]
    UnexpectedPostfixToken { got: Token },
    #[error("Unexpected token '|'")]
    UnexpectedBar,
    #[error("Unexpected token '{}'. Expected '{}'", got, expected)]
    UnexpectedToken { got: Token, expected: Token },
    #[error("Expected an identifier, got '{}'", got)]
    ExpectedIdent { got: Token },
    #[error("Expected a character, got '{}'", got)]
    ExpectedChar { got: Token },
    #[error("Expected a postfix operator, got '{}'", got)]
    ExpectedPostfixOperator { got: Token },
    #[error("Expected end of input, got '{}'", got)]
    ExpectedEof { got: Token },
}

type Result<T> = std::result::Result<T, ParseError>;

pub struct RegexParser<I: Iterator> {
    source: Peekable<I>,
    nodes: RegexArena,
    stack: Vec<Vec<RegexNodeIndex>>,
}

impl<I> RegexParser<I>
where
    I: Iterator<Item = Token>,
{
    pub fn parse(source: I) -> Result<Regex> {
        let mut parser = RegexParser {
            source: source.peekable(),
            nodes: RegexArena::default(),
            stack: vec![Vec::new()],
        };

        parser.parse_regex()?;
        if parser.peek() != Token::Eof {
            return Err(ParseError::ExpectedEof { got: parser.peek() });
        }
        let root_node = *parser
            .stack
            .last()
            .expect("Stack should contain one row")
            .last()
            .expect("Stack should contain one element");
        assert!(
            parser.stack.len() == 1 && parser.stack[0].len() == 1,
            "Stack should be empty now, but is: {:?}",
            parser.stack
        );

        Ok(Regex {
            arena: parser.nodes,
            root: root_node,
        })
    }

    fn consume(&mut self) -> Token {
        self.source.next().unwrap_or(Token::Eof)
    }

    fn expect(&mut self, token: Token) -> Result<()> {
        let next = self.consume();
        if next != token {
            Err(ParseError::UnexpectedToken {
                got: next,
                expected: token,
            })
        } else {
            Ok(())
        }
    }

    fn peek(&mut self) -> Token {
        self.source.peek().copied().unwrap_or(Token::Eof)
    }

    /// Interprets the next token as a character in a `\[...\]` group
    fn consume_as_char(&mut self) -> Result<char> {
        fn single_char(input: &str) -> char {
            let mut chars = input.chars();
            let char = chars.next().expect("Input should not be empty");
            assert!(
                chars.next().is_none(),
                "Input should have a single character only"
            );
            char
        }
        if matches!(self.peek(), Token::Eof | Token::RightBracket) {
            return Err(ParseError::UnexpectedRightBracket);
        };

        Ok(single_char(&self.consume().to_string()))
    }

    fn push_node(&mut self, node: RegexNode) -> RegexNodeIndex {
        let node_idx = self.nodes.add(node);
        self.push_node_idx(node_idx);
        node_idx
    }

    fn push_node_idx(&mut self, idx: RegexNodeIndex) {
        self.stack.last_mut().expect("Stack not empty").push(idx);
    }

    fn pop_row(&mut self) -> Vec<RegexNodeIndex> {
        self.stack.pop().expect("Stack not empty")
    }

    fn pop_single(&mut self) -> RegexNodeIndex {
        self.stack
            .last_mut()
            .expect("Stack not empty")
            .pop()
            .expect("Stack not empty")
    }

    fn push_row(&mut self) {
        self.stack.push(Vec::new());
    }

    fn parse_regex(&mut self) -> Result<()> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<()> {
        self.push_row();

        loop {
            self.parse_and()?;
            if self.peek() == Token::Pipe {
                self.consume();
            } else {
                break;
            }
        }

        let nodes = self.pop_row();
        match nodes.as_slice() {
            [single] => self.push_node_idx(*single),
            _ => {
                self.push_node(RegexNode::Or(nodes));
            }
        };

        Ok(())
    }

    fn parse_and(&mut self) -> Result<()> {
        self.push_row();

        loop {
            self.parse_value()?;
            if !self.peek().is_valid_after_value() {
                break;
            }
        }

        let nodes = self.pop_row();
        match nodes.as_slice() {
            [single] => self.push_node_idx(*single),
            _ => {
                self.push_node(RegexNode::And(nodes));
            }
        }

        Ok(())
    }

    fn parse_value(&mut self) -> Result<()> {
        match self.peek() {
            Token::Eof => Ok(()),
            Token::Char(_) | Token::Dot => self.parse_char(),
            Token::RightBrace => Err(ParseError::UnexpectedRightBrace),
            Token::LeftBrace => self.parse_variable(),
            Token::LeftParenthesis => self.parse_parenthesis(),
            Token::RightParenthesis => Err(ParseError::UnexpectedRightParenthesis),
            Token::LeftBracket => self.parse_group(),
            Token::RightBracket => Err(ParseError::UnexpectedRightBracket),
            Token::Minus => Err(ParseError::UnexpectedMinus),
            Token::Pipe => Err(ParseError::UnexpectedBar),
            token @ Token::Postfix(_) => Err(ParseError::UnexpectedPostfixToken { got: token }),
        }
    }

    fn parse_group(&mut self) -> Result<()> {
        self.expect(Token::LeftBracket)?;
        self.parse_group_inner()?;
        self.expect(Token::RightBracket)?;

        if matches!(self.peek(), Token::Postfix(_)) {
            self.parse_postfix()?;
        }

        Ok(())
    }

    fn parse_group_inner(&mut self) -> Result<()> {
        let mut chars = Vec::new();
        while let Ok(char) = self.consume_as_char() {
            if self.peek() == Token::Minus {
                self.consume();
                let final_char = self.consume_as_char()?;
                chars.push(
                    self.nodes
                        .add(RegexNode::Literal(RegexPattern::Range(char, final_char))),
                )
            } else {
                chars.push(self.nodes.add(RegexNode::Literal(RegexPattern::Char(char))));
            }
        }

        match chars.as_slice() {
            [single] => self.push_node_idx(*single),
            _ => {
                self.push_node(RegexNode::Or(chars));
            }
        }

        Ok(())
    }

    fn parse_parenthesis(&mut self) -> Result<()> {
        self.expect(Token::LeftParenthesis)?;
        self.parse_regex()?;
        self.expect(Token::RightParenthesis)?;

        if matches!(self.peek(), Token::Postfix(_)) {
            self.parse_postfix()?;
        }

        Ok(())
    }

    fn parse_postfix(&mut self) -> Result<()> {
        let token = self.consume();
        let Token::Postfix(postfix_token) = token else {
            return Err(ParseError::ExpectedPostfixOperator { got: token });
        };

        let node = match postfix_token {
            PostfixToken::QuestionMark => RegexNode::ZeroOrOne,
            PostfixToken::Star => RegexNode::Many,
            PostfixToken::Plus => RegexNode::OneOrMore,
        };

        let child = self.pop_single();
        self.push_node(node(child));

        Ok(())
    }

    fn parse_char(&mut self) -> Result<()> {
        let token = self.consume();
        match token {
            Token::Char(char) => {
                self.push_node(RegexNode::Literal(RegexPattern::Char(char)));
            }
            Token::Dot => {
                self.push_node(RegexNode::Literal(RegexPattern::AnyChar));
            }
            _ => return Err(ParseError::ExpectedChar { got: token }),
        }

        if matches!(self.peek(), Token::Postfix(_)) {
            self.parse_postfix()?;
        }

        Ok(())
    }

    fn parse_variable(&mut self) -> Result<()> {
        self.expect(Token::LeftBrace)?;
        let ident = self.parse_ident()?;
        self.push_node(RegexNode::Variable(ident));
        self.expect(Token::RightBrace)?;
        Ok(())
    }

    fn parse_ident(&mut self) -> Result<String> {
        let mut ident = String::new();
        while let Token::Char(char) = self.peek() {
            ident.push(char);
            self.consume();
        }
        if ident.is_empty() {
            return Err(ParseError::ExpectedIdent { got: self.peek() });
        }
        Ok(ident)
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::ParseError;
    use crate::regex::Regex;

    fn parse(source: &str) -> Result<Regex, ParseError> {
        Regex::from_str(source)
    }

    #[test]
    fn test_char() {
        insta::assert_debug_snapshot!(parse("a"));
        insta::assert_debug_snapshot!(parse("abc"));
    }

    #[test]
    fn test_variable() {
        insta::assert_debug_snapshot!(parse("{a}"));
        insta::assert_debug_snapshot!(parse("a{a}b{b}c"));
    }

    #[test]
    fn test_postfix_operator() {
        insta::assert_debug_snapshot!(parse("a?"));
        insta::assert_debug_snapshot!(parse("a+"));
        insta::assert_debug_snapshot!(parse("a*"));
    }

    #[test]
    fn test_postfix_error() {
        insta::assert_debug_snapshot!(parse("a?+"));
        insta::assert_debug_snapshot!(parse("a**"));
    }

    #[test]
    fn test_or() {
        insta::assert_debug_snapshot!(parse("a|b"));
        insta::assert_debug_snapshot!(parse("a?|b|c+d"));
    }

    #[test]
    fn test_parenthesis() {
        insta::assert_debug_snapshot!(parse("(ab)"));
        insta::assert_debug_snapshot!(parse("(ab)|(cd)+"));
        insta::assert_debug_snapshot!(parse("((a|b)c)*"));
        insta::assert_debug_snapshot!(parse("(ab|cd)*"));
    }

    #[test]
    fn test_empty() {
        insta::assert_debug_snapshot!(parse(""));
    }

    #[test]
    fn test_group() {
        insta::assert_debug_snapshot!(parse("[ABC]"));
        insta::assert_debug_snapshot!(parse("[ABC]|[DEF]"));
        insta::assert_debug_snapshot!(parse("a[ABC]*e"));
    }

    #[test]
    fn test_range() {
        insta::assert_debug_snapshot!(parse("[a-z]"));
        insta::assert_debug_snapshot!(parse("[a-z1234A-Z]"));
        insta::assert_debug_snapshot!(parse("[,.{}()]"));
    }

    #[test]
    fn test_dot() {
        insta::assert_debug_snapshot!(parse("a.c"));
        insta::assert_debug_snapshot!(parse(".*."));
        insta::assert_debug_snapshot!(parse("[.,]"));
    }
}
