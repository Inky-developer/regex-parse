use crate::regex::RegexPattern;
use std::fmt::{Display, Write};
use std::iter::Peekable;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Token {
    Char(char),
    Dot,
    CharacterClass(CharacterClass),
    LeftBrace,
    RightBrace,
    LeftParenthesis,
    RightParenthesis,
    LeftBracket,
    RightBracket,
    Minus,
    Postfix(PostfixToken),
    Pipe,
    Eof,
}

impl Token {
    /// Indicates whether this token may follow after a value to combine into an and-node
    pub fn is_valid_after_value(self) -> bool {
        match self {
            Token::RightBrace
            | Token::RightParenthesis
            | Token::RightBracket
            | Token::Postfix(_)
            | Token::Pipe
            | Token::Minus
            | Token::Eof => false,
            Token::Char(_)
            | Token::Dot
            | Token::CharacterClass(_)
            | Token::LeftBrace
            | Token::LeftParenthesis
            | Token::LeftBracket => true,
        }
    }
}

/// Perl character classes (e.g. `\d`, `\w`)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CharacterClass {
    Whitespace,
    Digit,
    Word,
}

impl CharacterClass {
    /// Returns a list of patterns that correspond to this character class if or-ed together
    pub fn as_patterns(self) -> &'static [RegexPattern] {
        match self {
            CharacterClass::Whitespace => &[
                RegexPattern::Char('\r'),
                RegexPattern::Char('\n'),
                RegexPattern::Char('\t'),
                RegexPattern::Char(' '),
            ],
            CharacterClass::Digit => &[RegexPattern::Range('0', '9')],
            CharacterClass::Word => &[
                RegexPattern::Range('a', 'z'),
                RegexPattern::Range('A', 'Z'),
                RegexPattern::Range('0', '9'),
                RegexPattern::Char('_'),
            ],
        }
    }
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
            Token::Dot => f.write_str("."),
            Token::CharacterClass(class) => match class {
                CharacterClass::Whitespace => f.write_str("\\s"),
                CharacterClass::Digit => f.write_str("\\d"),
                CharacterClass::Word => f.write_str("\\w"),
            },
            Token::LeftBrace => f.write_char('{'),
            Token::RightBrace => f.write_char('}'),
            Token::LeftParenthesis => f.write_char('('),
            Token::RightParenthesis => f.write_char(')'),
            Token::LeftBracket => f.write_char('['),
            Token::RightBracket => f.write_char(']'),
            Token::Minus => f.write_char('-'),
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
                let Some(next) = self.chars.next() else {
                    // TODO: This should probably return an error
                    return None;
                };
                let token = match next {
                    's' => Token::CharacterClass(CharacterClass::Whitespace),
                    'd' => Token::CharacterClass(CharacterClass::Digit),
                    'w' => Token::CharacterClass(CharacterClass::Word),
                    _ => Token::Char(next),
                };
                Some(token)
            }
            '{' => Some(Token::LeftBrace),
            '}' => Some(Token::RightBrace),
            '(' => Some(Token::LeftParenthesis),
            ')' => Some(Token::RightParenthesis),
            '[' => Some(Token::LeftBracket),
            ']' => Some(Token::RightBracket),
            '-' => Some(Token::Minus),
            '?' => Some(Token::Postfix(PostfixToken::QuestionMark)),
            '*' => Some(Token::Postfix(PostfixToken::Star)),
            '+' => Some(Token::Postfix(PostfixToken::Plus)),
            '|' => Some(Token::Pipe),
            '.' => Some(Token::Dot),
            _ => Some(Token::Char(char)),
        }
    }
}
