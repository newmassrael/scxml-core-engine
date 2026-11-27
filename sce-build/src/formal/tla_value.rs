// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The values of TLA+ as TLC prints them, read into one canonical form, so
//! that a state TLC wrote and a state another encoding of the same model
//! decodes to are compared by value (docs/adr/0119, D4): a set whatever order
//! TLC printed its elements in, a record whatever order its fields, and a
//! function whose domain is `1..n` as the sequence it is, which is how TLC
//! prints it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A value of TLA+.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Str(String),
    /// A model value, or another name TLC prints bare.
    Name(String),
    Set(BTreeSet<Value>),
    /// A function whose domain is `1..n`, `n` from 0: a sequence, and the
    /// function of empty domain.
    Seq(Vec<Value>),
    /// Any other function; a record is a function whose domain is strings.
    Fun(BTreeMap<Value, Value>),
}

impl Value {
    /// The function `map` describes, in its canonical form: a sequence where
    /// its domain is `1..n`, the empty one included.
    pub fn function(map: BTreeMap<Value, Value>) -> Value {
        let is_sequence = map
            .keys()
            .zip(1i64..)
            .all(|(key, at)| *key == Value::Int(at));
        if is_sequence {
            Value::Seq(map.into_values().collect())
        } else {
            Value::Fun(map)
        }
    }

    /// The record of `fields`.
    pub fn record<'a>(fields: impl IntoIterator<Item = (&'a str, Value)>) -> Value {
        Value::function(
            fields
                .into_iter()
                .map(|(name, value)| (Value::Str(name.to_string()), value))
                .collect(),
        )
    }

    pub fn string(text: &str) -> Value {
        Value::Str(text.to_string())
    }

    /// The sequence of `texts`, each a string.
    pub fn strings<'a>(texts: impl IntoIterator<Item = &'a str>) -> Value {
        Value::Seq(texts.into_iter().map(Value::string).collect())
    }

    /// The field `name` of a record, if this is one that has it.
    pub fn field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Fun(map) => map.get(&Value::string(name)),
            _ => None,
        }
    }
}

/// A list of values written between `open` and `close`, separated by commas.
fn write_list<'a>(
    f: &mut fmt::Formatter<'_>,
    open: &str,
    items: impl Iterator<Item = &'a Value>,
    close: &str,
) -> fmt::Result {
    f.write_str(open)?;
    for (at, item) in items.enumerate() {
        if at > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{item}")?;
    }
    f.write_str(close)
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(value) => f.write_str(if *value { "TRUE" } else { "FALSE" }),
            Value::Int(value) => write!(f, "{value}"),
            Value::Str(text) => {
                f.write_str("\"")?;
                for c in text.chars() {
                    if matches!(c, '"' | '\\') {
                        f.write_str("\\")?;
                    }
                    write!(f, "{c}")?;
                }
                f.write_str("\"")
            }
            Value::Name(name) => f.write_str(name),
            Value::Set(items) => write_list(f, "{", items.iter(), "}"),
            Value::Seq(items) => write_list(f, "<<", items.iter(), ">>"),
            Value::Fun(map) => {
                let record = map.keys().all(|key| matches!(key, Value::Str(_)));
                f.write_str(if record { "[" } else { "(" })?;
                for (at, (key, value)) in map.iter().enumerate() {
                    match (record, key) {
                        (true, Value::Str(name)) => {
                            if at > 0 {
                                f.write_str(", ")?;
                            }
                            write!(f, "{name} |-> {value}")?;
                        }
                        _ => {
                            if at > 0 {
                                f.write_str(" @@ ")?;
                            }
                            write!(f, "{key} :> {value}")?;
                        }
                    }
                }
                f.write_str(if record { "]" } else { ")" })
            }
        }
    }
}

/// A state: the value of each variable, by its name.
pub type State = BTreeMap<String, Value>;

/// A state as one line: each variable and its value, in the order of their
/// names.
pub fn shown(state: &State) -> String {
    state
        .iter()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect::<Vec<_>>()
        .join(" /\\ ")
}

/// One token of what TLC prints.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Open(&'static str),
    Close(&'static str),
    Comma,
    MapsTo,
    Colon,
    AtAt,
    Equals,
    Conjunction,
    Str(String),
    Int(i64),
    Name(String),
}

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut at = 0;
    let starts = |at: usize, word: &str| {
        word.chars()
            .enumerate()
            .all(|(offset, c)| chars.get(at + offset) == Some(&c))
    };
    while at < chars.len() {
        let c = chars[at];
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        let fixed: [(&str, Token); 13] = [
            ("<<", Token::Open("<<")),
            (">>", Token::Close(">>")),
            ("|->", Token::MapsTo),
            (":>", Token::Colon),
            ("@@", Token::AtAt),
            ("/\\", Token::Conjunction),
            ("{", Token::Open("{")),
            ("}", Token::Close("}")),
            ("[", Token::Open("[")),
            ("]", Token::Close("]")),
            ("(", Token::Open("(")),
            (")", Token::Close(")")),
            (",", Token::Comma),
        ];
        if let Some((word, token)) = fixed.iter().find(|(word, _)| starts(at, *word)) {
            found.push(token.clone());
            at += word.chars().count();
            continue;
        }
        if c == '=' {
            found.push(Token::Equals);
            at += 1;
            continue;
        }
        if c == '"' {
            let mut text = String::new();
            at += 1;
            loop {
                match chars.get(at) {
                    None => return Err("a string that is not closed".to_string()),
                    Some('"') => {
                        at += 1;
                        break;
                    }
                    Some('\\') => {
                        match chars.get(at + 1) {
                            Some('n') => text.push('\n'),
                            Some('t') => text.push('\t'),
                            Some(other) => text.push(*other),
                            None => return Err("a string that is not closed".to_string()),
                        }
                        at += 2;
                    }
                    Some(other) => {
                        text.push(*other);
                        at += 1;
                    }
                }
            }
            found.push(Token::Str(text));
            continue;
        }
        if c.is_ascii_digit() || (c == '-' && chars.get(at + 1).is_some_and(char::is_ascii_digit)) {
            let start = at;
            at += 1;
            while chars.get(at).is_some_and(char::is_ascii_digit) {
                at += 1;
            }
            let spelled: String = chars[start..at].iter().collect();
            let value = spelled
                .parse::<i64>()
                .map_err(|e| format!("`{spelled}` is no integer TLC prints: {e}"))?;
            found.push(Token::Int(value));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = at;
            while chars
                .get(at)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
            {
                at += 1;
            }
            found.push(Token::Name(chars[start..at].iter().collect()));
            continue;
        }
        return Err(format!("`{c}` is no part of a value TLC prints"));
    }
    Ok(found)
}

/// Reads values from tokens.
struct Reader {
    tokens: Vec<Token>,
    at: usize,
}

impl Reader {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn expect(&mut self, wanted: &Token) -> Result<(), String> {
        match self.take() {
            Some(token) if token == *wanted => Ok(()),
            other => Err(format!("expected {wanted:?}, found {other:?}")),
        }
    }

    /// Values separated by commas, up to `close`.
    fn list(&mut self, close: &'static str) -> Result<Vec<Value>, String> {
        let mut items = Vec::new();
        if self.peek() == Some(&Token::Close(close)) {
            self.at += 1;
            return Ok(items);
        }
        loop {
            items.push(self.value()?);
            match self.take() {
                Some(Token::Comma) => {}
                Some(Token::Close(found)) if found == close => return Ok(items),
                other => return Err(format!("expected , or {close}, found {other:?}")),
            }
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.take() {
            Some(Token::Int(value)) => Ok(Value::Int(value)),
            Some(Token::Str(text)) => Ok(Value::Str(text)),
            Some(Token::Name(name)) => Ok(match name.as_str() {
                "TRUE" => Value::Bool(true),
                "FALSE" => Value::Bool(false),
                _ => Value::Name(name),
            }),
            Some(Token::Open("{")) => Ok(Value::Set(self.list("}")?.into_iter().collect())),
            Some(Token::Open("<<")) => Ok(Value::Seq(self.list(">>")?)),
            Some(Token::Open("[")) => {
                let mut fields = BTreeMap::new();
                if self.peek() == Some(&Token::Close("]")) {
                    self.at += 1;
                    return Ok(Value::function(fields));
                }
                loop {
                    let name = match self.take() {
                        Some(Token::Name(name)) => name,
                        other => return Err(format!("expected a field's name, found {other:?}")),
                    };
                    self.expect(&Token::MapsTo)?;
                    fields.insert(Value::Str(name), self.value()?);
                    match self.take() {
                        Some(Token::Comma) => {}
                        Some(Token::Close("]")) => return Ok(Value::function(fields)),
                        other => return Err(format!("expected , or ], found {other:?}")),
                    }
                }
            }
            Some(Token::Open("(")) => {
                let mut map = BTreeMap::new();
                loop {
                    let key = self.value()?;
                    self.expect(&Token::Colon)?;
                    map.insert(key, self.value()?);
                    match self.take() {
                        Some(Token::AtAt) => {}
                        Some(Token::Close(")")) => return Ok(Value::function(map)),
                        other => return Err(format!("expected @@ or ), found {other:?}")),
                    }
                }
            }
            other => Err(format!("expected a value, found {other:?}")),
        }
    }
}

/// The value `text` spells, as TLC prints values.
pub fn parse_value(text: &str) -> Result<Value, String> {
    let mut reader = Reader {
        tokens: tokens(text)?,
        at: 0,
    };
    let value = reader.value()?;
    match reader.peek() {
        None => Ok(value),
        Some(token) => Err(format!("{token:?} after a whole value")),
    }
}

/// The state `text` spells, as TLC prints a state: `/\\ name = value` for each
/// variable, or `name = value` for a state of one variable.
pub fn parse_state(text: &str) -> Result<State, String> {
    let mut reader = Reader {
        tokens: tokens(text)?,
        at: 0,
    };
    let mut state = State::new();
    while reader.peek().is_some() {
        if reader.peek() == Some(&Token::Conjunction) {
            reader.at += 1;
        }
        let name = match reader.take() {
            Some(Token::Name(name)) => name,
            other => return Err(format!("expected a variable's name, found {other:?}")),
        };
        reader.expect(&Token::Equals)?;
        let value = reader.value()?;
        if state.insert(name.clone(), value).is_some() {
            return Err(format!("the variable {name} is given twice"));
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turns red if a set is read in the order printed, a record in the order
    /// of its fields, or a function of domain 1..n otherwise than as the
    /// sequence TLC prints it as.
    #[test]
    fn values_are_read_into_one_form_whatever_order_tlc_prints() {
        assert_eq!(parse_value(r#"{"b", "a"}"#), parse_value(r#"{"a", "b"}"#));
        assert_eq!(
            parse_value("[y |-> 2, x |-> TRUE]"),
            Ok(Value::record([
                ("x", Value::Bool(true)),
                ("y", Value::Int(2))
            ]))
        );
        assert_eq!(
            parse_value("(1 :> \"a\" @@ 2 :> \"b\")"),
            parse_value(r#"<<"a", "b">>"#)
        );
        assert_eq!(
            parse_value(r#"("h" :> {"s1"})"#),
            parse_value(r#"[h |-> {"s1"}]"#)
        );
        assert_eq!(parse_value("<<>>"), Ok(Value::Seq(Vec::new())));
        assert_eq!(Value::function(BTreeMap::new()), Value::Seq(Vec::new()));
        assert_eq!(
            parse_value(r#"<<<<"done", "state", "p0">>>>"#),
            Ok(Value::Seq(vec![Value::strings(["done", "state", "p0"])]))
        );
        assert_eq!(parse_value("-3"), Ok(Value::Int(-3)));
        assert_eq!(parse_value(r#""a\"b""#), Ok(Value::string("a\"b")));
        assert!(parse_value("{1, 2").is_err());
        assert!(parse_value("1 2").is_err());
    }

    /// Turns red if a state of several variables, or of one, is not read whole.
    #[test]
    fn a_state_is_read_as_tlc_prints_it() {
        let state = parse_state("/\\ x = 1\n/\\ told = [fired |-> <<12>>, exited |-> <<>>]\n")
            .expect("a state");
        assert_eq!(state["x"], Value::Int(1));
        assert_eq!(
            state["told"].field("fired"),
            Some(&Value::Seq(vec![Value::Int(12)]))
        );
        assert_eq!(
            parse_state("phase = \"loop\"").expect("a state")["phase"],
            Value::string("loop")
        );
        assert!(parse_state("/\\ x = 1\n/\\ x = 2").is_err());
    }

    /// Turns red if a value is written back otherwise than TLC spells it.
    #[test]
    fn a_value_is_shown_as_tla() {
        let value = Value::record([
            ("entered", Value::strings(["s0"])),
            ("fired", Value::Seq(vec![Value::Int(3)])),
        ]);
        assert_eq!(
            value.to_string(),
            r#"[entered |-> <<"s0">>, fired |-> <<3>>]"#
        );
        assert_eq!(parse_value(&value.to_string()), Ok(value));
    }
}
