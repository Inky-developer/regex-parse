use crate::regex::{Regex, RegexArena, RegexNode, RegexNodeIndex, RegexPattern};
use crate::tokenizer::{PostfixToken, Token};
use std::iter::Peekable;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token '}}'. Did you forget a '{{'?")]
    UnexpectedRightBrace,
    #[error("Unexpected postfix token: '{}'", got)]
    UnexpectedPostfixToken { got: Token },
    #[error("Unexpected token '{}'. Expected '{}'", got, expected)]
    UnexpectedToken { got: Token, expected: Token },
    #[error("Expected an identifier, got '{}'", got)]
    ExpectedIdent { got: Token },
    #[error("Expected a character, got '{}'", got)]
    ExpectedChar { got: Token },
    #[error("Expected a postfix operator, got '{}'", got)]
    ExpectedPostfixOperator { got: Token },
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

        parser.parse_root(&[])?;
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
        self.source.next().unwrap_or(Token::EOF)
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
        self.source.peek().copied().unwrap_or(Token::EOF)
    }

    fn push_node(&mut self, node: RegexNode) -> RegexNodeIndex {
        let node_idx = self.nodes.add(node);
        self.stack
            .last_mut()
            .expect("Stack not empty")
            .push(node_idx);
        node_idx
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

    fn parse_root(&mut self, stop_tokens: &[Token]) -> Result<()> {
        self.push_row();
        loop {
            if self.peek() == Token::EOF || stop_tokens.contains(&self.peek()) {
                break;
            }
            self.parse_next()?;
        }
        let nodes = self.pop_row();
        match nodes.as_slice() {
            [single] => self
                .stack
                .last_mut()
                .expect("Stack not empty")
                .push(*single),
            _ => {
                self.push_node(RegexNode::And(nodes));
            }
        };

        Ok(())
    }

    fn parse_next(&mut self) -> Result<()> {
        match self.peek() {
            Token::EOF => Ok(()),
            Token::Char(_) => self.parse_char(),
            Token::RightBrace => Err(ParseError::UnexpectedRightBrace),
            Token::LeftBrace => self.parse_variable(),
            Token::Pipe => self.parse_or(),
            token @ Token::Postfix(_) => Err(ParseError::UnexpectedPostfixToken { got: token }),
        }
    }

    fn parse_or(&mut self) -> Result<()> {
        while self.peek() == Token::Pipe {
            self.consume();
            self.parse_root(&[Token::Pipe])?;
        }

        let parts = self.pop_row();
        self.push_row();
        self.push_node(RegexNode::Or(parts));

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
        loop {
            match self.peek() {
                Token::Char(char) => ident.push(char),
                _ => break,
            }
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
    fn test_or() {
        insta::assert_debug_snapshot!(parse("a?|b|c+d"));
    }
}
