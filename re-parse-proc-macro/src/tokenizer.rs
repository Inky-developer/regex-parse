use std::fmt::{Display, Write};
use std::iter::Peekable;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Token {
    Char(char),
    LeftBrace,
    RightBrace,
    Postfix(PostfixToken),
    Pipe,
    Eof,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PostfixToken {
    QuestionMark,
    Star,
    Plus,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Token::Char(c) => f.write_char(c),
            Token::LeftBrace => f.write_char('{'),
            Token::RightBrace => f.write_char('}'),
            Token::Postfix(postfix_token) => match postfix_token {
                PostfixToken::QuestionMark => f.write_char('?'),
                PostfixToken::Star => f.write_char('*'),
                PostfixToken::Plus => f.write_char('+'),
            },
            Token::Pipe => f.write_char('|'),
            Token::Eof => f.write_str("<EOF>"),
        }
    }
}

pub fn tokenize(input: &str) -> impl Iterator<Item = Token> + use<'_> {
    Tokenizer {
        chars: input.chars().peekable(),
    }
}

struct Tokenizer<I: Iterator> {
    chars: Peekable<I>,
}

impl<I> Iterator for Tokenizer<I>
where
    I: Iterator<Item = char>,
{
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let char = self.chars.next()?;

        match char {
            '\\' => {
                let next = self.chars.next().expect("Unterminated escape sequence");
                Some(Token::Char(next))
            }
            '{' => Some(Token::LeftBrace),
            '}' => Some(Token::RightBrace),
            '?' => Some(Token::Postfix(PostfixToken::QuestionMark)),
            '*' => Some(Token::Postfix(PostfixToken::Star)),
            '+' => Some(Token::Postfix(PostfixToken::Plus)),
            '|' => Some(Token::Pipe),
            _ => Some(Token::Char(char)),
        }
    }
}