// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The runtime's one JSON reader and writer.
//!
//! Two things read JSON here: an event's `_event.data` lifted to its typed
//! payload ([`crate::event_payload`]) and a machine's saved state read back
//! ([`crate::saved_state`]). One reader serves both, so a text one accepts and
//! the other refuses cannot exist; one writer serves the saved state and the
//! inject seam's `data`.
//!
//! A number keeps the spelling it was written in ([`crate::json::Value::Number`]), so a
//! whole number stays exact until the one who reads it chooses its width — a
//! 64-bit value never passes through an `f64`.
//!
//! ⚠ `std` only, like both of its readers: a `no_std` build has neither an
//! event data wire nor a saved state.

#![cfg(not(feature = "no_std"))]

use core::fmt;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, in the spelling it was written in.
    Number(String),
    /// A text, its escapes already read.
    Text(String),
    /// An array's items in order.
    Array(Vec<Value>),
    /// An object's members in the order they were written.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member `name` of an object, or `None` for a missing member or a
    /// value that is not an object.
    pub fn member(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Why a text is not one JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonError {
    /// The text breaks the grammar; the sentence says where.
    Malformed(String),
    /// One value is followed by more text.
    Trailing,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::Malformed(what) => write!(f, "not JSON ({what})"),
            JsonError::Trailing => f.write_str("more than one JSON value"),
        }
    }
}

/// Read `text` as exactly one JSON value, surrounding whitespace aside.
pub fn parse(text: &str) -> Result<Value, JsonError> {
    let mut reader = Reader { text, at: 0 };
    let value = reader.read_value()?;
    reader.skip_whitespace();
    if reader.at < text.len() {
        return Err(JsonError::Trailing);
    }
    Ok(value)
}

/// `value` as compact JSON text. A number is written in the spelling it
/// carries, so a value read by [`parse`] is written back unchanged.
pub fn write(value: &Value) -> String {
    let mut out = String::new();
    write_into(value, &mut out);
    out
}

fn write_into(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(n),
        Value::Text(s) => out.push_str(&quote(s)),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (i, (key, item)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&quote(key));
                out.push(':');
                write_into(item, out);
            }
            out.push('}');
        }
    }
}

/// The JSON spelling of one text.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.at += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn malformed(what: &str) -> JsonError {
        JsonError::Malformed(what.to_string())
    }

    fn read_value(&mut self) -> Result<Value, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(Self::malformed("it ends where a value was expected")),
            Some('{') => self.read_object(),
            Some('[') => self.read_array(),
            Some('"') => Ok(Value::Text(self.read_string()?)),
            Some('t') => self.read_keyword("true").map(|()| Value::Bool(true)),
            Some('f') => self.read_keyword("false").map(|()| Value::Bool(false)),
            Some('n') => self.read_keyword("null").map(|()| Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => Ok(Value::Number(self.read_number()?)),
            Some(c) => Err(Self::malformed(&format!("unexpected '{c}'"))),
        }
    }

    fn read_object(&mut self) -> Result<Value, JsonError> {
        self.bump(); // '{'
        let mut members = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(Self::malformed("a field name was expected"));
            }
            let key = self.read_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(Self::malformed("a ':' was expected"));
            }
            self.bump();
            let value = self.read_value()?;
            members.push((key, value));
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some('}') => return Ok(Value::Object(members)),
                _ => return Err(Self::malformed("a ',' or '}' was expected")),
            }
        }
    }

    fn read_array(&mut self) -> Result<Value, JsonError> {
        self.bump(); // '['
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.read_value()?);
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some(']') => return Ok(Value::Array(items)),
                _ => return Err(Self::malformed("a ',' or ']' was expected")),
            }
        }
    }

    fn read_string(&mut self) -> Result<String, JsonError> {
        self.bump(); // '"'
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(Self::malformed("a text ends unclosed")),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    None => return Err(Self::malformed("an escape ends the text")),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        if self.at + 4 > self.text.len() {
                            return Err(Self::malformed("a \\u escape is short"));
                        }
                        let hex = &self.text[self.at..self.at + 4];
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| Self::malformed("a \\u escape is not hex"))?;
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        self.at += 4;
                    }
                    Some(e) => {
                        return Err(Self::malformed(&format!("unknown escape '\\{e}'")));
                    }
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn read_number(&mut self) -> Result<String, JsonError> {
        let start = self.at;
        if self.peek() == Some('-') {
            self.bump();
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.') {
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.bump();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.bump();
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        let slice = &self.text[start..self.at];
        if slice.is_empty() || slice == "-" {
            return Err(Self::malformed("a number has no digits"));
        }
        Ok(slice.to_string())
    }

    fn read_keyword(&mut self, word: &str) -> Result<(), JsonError> {
        if !self.text[self.at..].starts_with(word) {
            return Err(Self::malformed(&format!("expected '{word}'")));
        }
        self.at += word.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_written_is_read_back_unchanged() {
        let text = r#"{"a":[1,-2.5e3,"x\ny"],"b":{"c":true,"d":null},"e":18446744073709551615}"#;
        let value = parse(text).expect("valid");
        assert_eq!(write(&value), text);
    }

    #[test]
    fn a_whole_number_keeps_its_spelling() {
        // No `f64` stands between the text and the reader who picks a width.
        assert_eq!(
            parse("18446744073709551615"),
            Ok(Value::Number("18446744073709551615".to_string()))
        );
    }

    #[test]
    fn trailing_text_and_broken_grammar_are_refused() {
        assert_eq!(parse("1 2"), Err(JsonError::Trailing));
        assert!(matches!(parse("{\"a\" 1}"), Err(JsonError::Malformed(_))));
        assert!(matches!(parse("[1,"), Err(JsonError::Malformed(_))));
    }
}
