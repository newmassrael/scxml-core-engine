// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tokens of the TTCN-3 the export writes (ES 201 873-1 Annex A.1): names,
//! numbers, character strings, octetstrings and the punctuation of its
//! constructs. A comment, the header of a generated source among them, is
//! no token. A character outside these is a [`ParseError`].

use super::ParseError;

/// One token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A name or a keyword: an ASCII letter, then letters, digits and
    /// underscores.
    Word(String),
    /// Decimal digits.
    Integer(String),
    /// Decimal digits, a point and decimal digits.
    Float(String),
    /// A character string, its doubled quotes read as one.
    Text(String),
    /// An octetstring's hexadecimal digits, `'…'O`.
    Octets(String),
    /// Punctuation.
    Punct(&'static str),
}

impl Token {
    /// The token as TTCN-3 writes it.
    pub fn spelling(&self) -> String {
        match self {
            Token::Word(text) | Token::Integer(text) | Token::Float(text) => text.clone(),
            Token::Text(text) => format!("\"{}\"", text.replace('"', "\"\"")),
            Token::Octets(hex) => format!("'{hex}'O"),
            Token::Punct(punct) => (*punct).to_string(),
        }
    }
}

/// A token and the row it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexed {
    pub token: Token,
    pub line: u32,
}

/// The punctuation, each spelling of two characters before the one of its
/// first character, so the longer is read first.
const PUNCTUATION: [&str; 23] = [
    ":=", "==", "!=", "<=", ">=", "->", "..", "{", "}", "(", ")", "[", "]", ";", ",", ":", ".",
    "<", ">", "?", "*", "-", "&",
];

/// Whether `chars` begins with `text`.
fn begins_with(chars: &[char], text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(position, c)| chars.get(position) == Some(&c))
}

/// The tokens of `text`.
pub fn lex(text: &str) -> Result<Vec<Lexed>, ParseError> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    let mut line = 1;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        if c == '\n' {
            line += 1;
            at += 1;
            continue;
        }
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            let opened = line;
            at += 2;
            loop {
                match (chars.get(at), chars.get(at + 1)) {
                    (Some('*'), Some('/')) => {
                        at += 2;
                        break;
                    }
                    (Some(inside), _) => {
                        if *inside == '\n' {
                            line += 1;
                        }
                        at += 1;
                    }
                    (None, _) => return Err(ParseError::new(opened, "a comment never closed")),
                }
            }
            continue;
        }
        let row = line;
        let token = if c.is_ascii_alphabetic() {
            let start = at;
            while at < chars.len() && (chars[at].is_ascii_alphanumeric() || chars[at] == '_') {
                at += 1;
            }
            Token::Word(chars[start..at].iter().collect())
        } else if c.is_ascii_digit() {
            let start = at;
            while at < chars.len() && chars[at].is_ascii_digit() {
                at += 1;
            }
            let fraction = chars.get(at) == Some(&'.')
                && chars
                    .get(at + 1)
                    .is_some_and(|digit| digit.is_ascii_digit());
            if fraction {
                at += 1;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
                Token::Float(chars[start..at].iter().collect())
            } else {
                Token::Integer(chars[start..at].iter().collect())
            }
        } else if c == '"' {
            at += 1;
            let mut value = String::new();
            loop {
                match chars.get(at) {
                    Some('"') if chars.get(at + 1) == Some(&'"') => {
                        value.push('"');
                        at += 2;
                    }
                    Some('"') => {
                        at += 1;
                        break;
                    }
                    Some(inside) => {
                        if *inside == '\n' {
                            line += 1;
                        }
                        value.push(*inside);
                        at += 1;
                    }
                    None => return Err(ParseError::new(row, "a character string never closed")),
                }
            }
            Token::Text(value)
        } else if c == '\'' {
            at += 1;
            let start = at;
            while at < chars.len() && chars[at].is_ascii_hexdigit() {
                at += 1;
            }
            let hex: String = chars[start..at].iter().collect();
            if chars.get(at) != Some(&'\'') || chars.get(at + 1) != Some(&'O') {
                return Err(ParseError::new(
                    row,
                    "a quoted string that is no octetstring ('…'O)",
                ));
            }
            at += 2;
            Token::Octets(hex)
        } else if let Some(&punct) = PUNCTUATION.iter().find(|p| begins_with(&chars[at..], p)) {
            at += punct.len();
            Token::Punct(punct)
        } else {
            return Err(ParseError::new(
                row,
                format!("{c:?} is no character of the TTCN-3 the export writes"),
            ));
        };
        tokens.push(Lexed { token, line: row });
    }
    Ok(tokens)
}

/// The text `tokens` are, spaced as the export spaces an expression: no
/// space around `.`, inside parentheses and brackets, before `,`, after a
/// sign, or between a name and the parenthesis of its arguments.
pub fn spelled(tokens: &[Lexed]) -> String {
    let mut text = String::new();
    let mut previous: Option<&Token> = None;
    let mut after_sign = false;
    for lexed in tokens {
        let token = &lexed.token;
        let tight = match previous {
            None => true,
            Some(before) => {
                after_sign
                    || matches!(token, Token::Punct("." | ")" | "]" | ","))
                    || matches!(before, Token::Punct("." | "(" | "["))
                    || (matches!(token, Token::Punct("("))
                        && matches!(before, Token::Word(word) if !is_operator_word(word)))
            }
        };
        if !tight {
            text.push(' ');
        }
        text.push_str(&token.spelling());
        after_sign = matches!(token, Token::Punct("-"))
            && match previous {
                None => true,
                Some(before) => opens_operand(before),
            };
        previous = Some(token);
    }
    text
}

/// Whether `word` is an operator TTCN-3 spells as a word.
pub fn is_operator_word(word: &str) -> bool {
    matches!(word, "and" | "or" | "xor" | "not")
}

/// Whether an operand may follow `token`, so a `-` after it is a sign.
fn opens_operand(token: &Token) -> bool {
    match token {
        Token::Punct(punct) => !matches!(*punct, ")" | "]"),
        Token::Word(word) => is_operator_word(word),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<Token> {
        lex(text)
            .expect("tokens")
            .into_iter()
            .map(|lexed| lexed.token)
            .collect()
    }

    /// A comment is no token, the header of a generated source among them,
    /// and a row is counted across it.
    #[test]
    fn a_comment_is_no_token() {
        let lexed = lex("// source-hash: abc\n/* two\nrows */ module m {}").expect("tokens");
        assert_eq!(lexed[0].token, Token::Word("module".to_string()));
        assert_eq!(lexed[0].line, 3);
        assert_eq!(lexed.len(), 4);
    }

    /// A range of integers is two integers and `..`; a float has digits on
    /// both sides of its point.
    #[test]
    fn a_number_is_an_integer_or_a_float() {
        assert_eq!(
            tokens("(0 .. 255) 0..9 2.0 -128"),
            [
                Token::Punct("("),
                Token::Integer("0".to_string()),
                Token::Punct(".."),
                Token::Integer("255".to_string()),
                Token::Punct(")"),
                Token::Integer("0".to_string()),
                Token::Punct(".."),
                Token::Integer("9".to_string()),
                Token::Float("2.0".to_string()),
                Token::Punct("-"),
                Token::Integer("128".to_string()),
            ]
        );
    }

    #[test]
    fn a_string_reads_its_doubled_quote_as_one() {
        assert_eq!(
            tokens(r#""say ""hi""" '0AFF'O"#),
            [
                Token::Text("say \"hi\"".to_string()),
                Token::Octets("0AFF".to_string())
            ]
        );
        assert!(lex("\"open").is_err());
        assert!(lex("'0A'B").is_err());
        assert!(lex("a # b").is_err());
    }

    /// An expression is spelled as the export spells one.
    #[test]
    fn an_expression_is_spelled_as_the_export_writes_it() {
        let spell = |text: &str| spelled(&lex(text).expect("tokens"));
        assert_eq!(spell("v . job == ( - 5 )"), "v.job == (-5)");
        assert_eq!(spell("not ( a and b )"), "not (a and b)");
        assert_eq!(
            spell("\"a\" & char ( 0 , 0 , 0 , 233 )"),
            "\"a\" & char(0, 0, 0, 233)"
        );
        assert_eq!(spell("x - 1"), "x - 1");
    }
}
