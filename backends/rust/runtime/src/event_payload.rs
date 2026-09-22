// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Lifting an event's typed `_event.data` view out of the data it carries.
//!
//! NL→IR Item C1 Path A gives a schema'd event a typed payload that the
//! natively lowered guards read. One producer fills it: the generated
//! `raise_<event>` inject seam. Every other producer — `<send>` with `<param>`,
//! namelist or `<content>`, an invoke forwarding an event either way,
//! autoforward, BasicHTTP, mesh — fills [`EventMetadata::data`](crate::EventMetadata),
//! the wire §scxml-5.10 describes and §scxml-B-2-8-1 reads.
//!
//! Until this module the two never met, so the same guard answered differently
//! depending on where its event came from, and a typed payload could not cross
//! an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
//! already refuses for the same guard, measured on the same document: no data,
//! a missing field, or a value of another type each give `error.execution` and
//! a guard that does not fire. A native lowering that answered differently
//! would make the optimisation observable, which is the one thing it may not
//! be.
//!
//! ⚠ `std` only, because the wire is: a `no_std` build has no
//! `EventMetadata::data` and no script engine to disagree with, so the typed
//! carrier is the whole channel there. That is why this module is `cfg`'d out
//! rather than given a no-alloc twin — there is nothing for one to read.
//!
//! Cross-language siblings: `sce_runtime.event_payload` (Python),
//! `sce.LiftPayload` (Go), `SCE::Common::EventPayloadFields` (C++).

#![cfg(not(feature = "no_std"))]

use core::fmt;

/// Why an event's data cannot be read as its schema's fields.
///
/// A sentence, not a code: it rides in the `error.execution` event's data,
/// where the document's own handler can read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadRefusal(String);

impl PayloadRefusal {
    /// Refuse a payload, saying why in the words the document's handler reads.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// Why the payload was refused.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PayloadRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One field's value, in the spelling it was written in, so a whole number
/// stays exact until it is read at the width its schema declares.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(String),
    Text(String),
    Truth(bool),
    /// A nested value this document's schema does not name. It is kept so a
    /// payload carrying more than the schema does still reads.
    Other,
}

/// The fields an event's data names.
#[derive(Clone, Debug, Default)]
pub struct PayloadFields {
    fields: Vec<(String, Value)>,
}

impl PayloadFields {
    /// Read an event's data as the fields it names.
    ///
    /// The JSON read is §scxml-B-2-8-1's second rung, the same one the
    /// script engine takes for the same string.
    pub fn decode(data: &str) -> Result<Self, PayloadRefusal> {
        let trimmed = data.trim();
        if trimmed.is_empty() {
            return Err(PayloadRefusal::new("the event carries no data"));
        }
        let mut reader = Reader::new(trimmed);
        let fields = reader.read_object()?;
        reader.skip_whitespace();
        if !reader.at_end() {
            return Err(PayloadRefusal::new(
                "the event's data carries more than one JSON value",
            ));
        }
        Ok(Self { fields })
    }

    fn find(&self, name: &str) -> Result<&Value, PayloadRefusal> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .ok_or_else(|| PayloadRefusal::new(format!("the event's data has no '{name}'")))
    }

    /// A whole-number field's written spelling.
    ///
    /// ⚠ A truth value is not a number here. JSON spells both, and a schema
    /// that declared `uint32` and received `true` has been handed something its
    /// own type says cannot occur.
    fn number(&self, name: &str) -> Result<&str, PayloadRefusal> {
        match self.find(name)? {
            Value::Number(text) => Ok(text),
            _ => Err(PayloadRefusal::new(format!("'{name}' is not a number"))),
        }
    }

    /// A signed whole number at the width its schema declares.
    ///
    /// A value the width cannot hold is refused rather than wrapped — silently
    /// truncating is how a guard on a boundary answers for a value the document
    /// never received.
    pub fn signed<T>(&self, name: &str) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<i64>,
    {
        let text = self.number(name)?;
        let whole: i64 = text
            .parse()
            .map_err(|_| PayloadRefusal::new(format!("'{name}' is not a whole number ({text})")))?;
        T::try_from(whole).map_err(|_| {
            PayloadRefusal::new(format!(
                "'{name}' does not fit the width its schema declares ({text})"
            ))
        })
    }

    /// An unsigned whole number at the width its schema declares.
    pub fn unsigned<T>(&self, name: &str) -> Result<T, PayloadRefusal>
    where
        T: TryFrom<u64>,
    {
        let text = self.number(name)?;
        let whole: u64 = text.parse().map_err(|_| {
            PayloadRefusal::new(format!(
                "'{name}' is not a whole number at or above zero ({text})"
            ))
        })?;
        T::try_from(whole).map_err(|_| {
            PayloadRefusal::new(format!(
                "'{name}' does not fit the width its schema declares ({text})"
            ))
        })
    }

    /// A 32-bit fractional field.
    pub fn float32(&self, name: &str) -> Result<f32, PayloadRefusal> {
        let text = self.number(name)?;
        text.parse()
            .map_err(|_| PayloadRefusal::new(format!("'{name}' is not a number ({text})")))
    }

    /// A 64-bit fractional field.
    pub fn float64(&self, name: &str) -> Result<f64, PayloadRefusal> {
        let text = self.number(name)?;
        text.parse()
            .map_err(|_| PayloadRefusal::new(format!("'{name}' is not a number ({text})")))
    }

    /// A truth-value field.
    pub fn truth(&self, name: &str) -> Result<bool, PayloadRefusal> {
        match self.find(name)? {
            Value::Truth(b) => Ok(*b),
            _ => Err(PayloadRefusal::new(format!(
                "'{name}' is not a truth value"
            ))),
        }
    }

    /// A text field.
    pub fn text(&self, name: &str) -> Result<crate::SceString, PayloadRefusal> {
        match self.find(name)? {
            Value::Text(s) => Ok(crate::sce_string_from_str(s)),
            _ => Err(PayloadRefusal::new(format!("'{name}' is not a text"))),
        }
    }

    /// A byte-string field at the capacity its schema declares.
    ///
    /// JSON has no byte string, so the wire carries the byte-exact Latin-1 text
    /// the inject seam writes, and this reads it back the same way: every one
    /// of the 256 values is one character and back. Printable ASCII — what a
    /// bytes guard compares — is the same bytes under either reading.
    pub fn bytes<const CAP: usize>(
        &self,
        name: &str,
    ) -> Result<crate::SceBytes<CAP>, PayloadRefusal> {
        let text = match self.find(name)? {
            Value::Text(s) => s,
            _ => {
                return Err(PayloadRefusal::new(format!(
                    "'{name}' is not a byte string"
                )))
            }
        };
        let mut out: Vec<u8> = Vec::with_capacity(text.len());
        for c in text.chars() {
            if (c as u32) > 0xFF {
                return Err(PayloadRefusal::new(format!(
                    "'{name}' carries a character above U+00FF, which no single byte spells"
                )));
            }
            out.push(c as u8);
        }
        crate::SceBytes::<CAP>::from_slice(&out).map_err(|_| {
            PayloadRefusal::new(format!(
                "'{name}' is {} bytes, past the {CAP} its schema declares",
                out.len()
            ))
        })
    }
}

/// The wire spelling of one byte string, for the inject seam's `data`.
///
/// The writing half of [`PayloadFields::bytes`] — see there for why Latin-1.
pub fn bytes_as_payload_text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| *b as char).collect()
}

/// The JSON spelling of one text, for the inject seam's `data`.
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

/// A JSON reader over one payload.
///
/// It reads the whole grammar rather than only an object of scalars: a payload
/// may carry fields this document's schema does not name, and a nested one
/// still has to be walked past to reach the fields that follow.
struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, at: 0 }
    }

    fn at_end(&self) -> bool {
        self.at >= self.text.len()
    }

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

    fn refuse(what: &str) -> PayloadRefusal {
        PayloadRefusal::new(format!("the event's data is not JSON ({what})"))
    }

    fn read_object(&mut self) -> Result<Vec<(String, Value)>, PayloadRefusal> {
        self.skip_whitespace();
        if self.peek() != Some('{') {
            return Err(PayloadRefusal::new(
                "the event's data is a bare value, and this event's schema declares named fields",
            ));
        }
        self.bump();
        let mut out = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(out);
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(Self::refuse("a field name was expected"));
            }
            let key = self.read_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(Self::refuse("a ':' was expected"));
            }
            self.bump();
            let value = self.read_value()?;
            out.push((key, value));
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some('}') => return Ok(out),
                _ => return Err(Self::refuse("a ',' or '}' was expected")),
            }
        }
    }

    fn read_value(&mut self) -> Result<Value, PayloadRefusal> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(Self::refuse("it ends where a value was expected")),
            Some('{') => {
                self.read_object()?;
                Ok(Value::Other)
            }
            Some('[') => {
                self.read_array()?;
                Ok(Value::Other)
            }
            Some('"') => Ok(Value::Text(self.read_string()?)),
            Some('t') => self.read_keyword("true").map(|()| Value::Truth(true)),
            Some('f') => self.read_keyword("false").map(|()| Value::Truth(false)),
            Some('n') => self.read_keyword("null").map(|()| Value::Other),
            Some(c) if c == '-' || c.is_ascii_digit() => Ok(Value::Number(self.read_number()?)),
            Some(c) => Err(Self::refuse(&format!("unexpected '{c}'"))),
        }
    }

    fn read_array(&mut self) -> Result<(), PayloadRefusal> {
        self.bump(); // '['
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(());
        }
        loop {
            self.read_value()?;
            self.skip_whitespace();
            match self.bump() {
                Some(',') => continue,
                Some(']') => return Ok(()),
                _ => return Err(Self::refuse("a ',' or ']' was expected")),
            }
        }
    }

    fn read_string(&mut self) -> Result<String, PayloadRefusal> {
        self.bump(); // '"'
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(Self::refuse("a text ends unclosed")),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    None => return Err(Self::refuse("an escape ends the text")),
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
                            return Err(Self::refuse("a \\u escape is short"));
                        }
                        let hex = &self.text[self.at..self.at + 4];
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| Self::refuse("a \\u escape is not hex"))?;
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        self.at += 4;
                    }
                    Some(e) => {
                        return Err(Self::refuse(&format!("unknown escape '\\{e}'")));
                    }
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn read_number(&mut self) -> Result<String, PayloadRefusal> {
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
            return Err(Self::refuse("a number has no digits"));
        }
        Ok(slice.to_string())
    }

    fn read_keyword(&mut self, word: &str) -> Result<(), PayloadRefusal> {
        if !self.text[self.at..].starts_with(word) {
            return Err(Self::refuse(&format!("expected '{word}'")));
        }
        self.at += word.len();
        Ok(())
    }
}
