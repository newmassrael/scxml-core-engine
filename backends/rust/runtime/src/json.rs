// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#![cfg(feature = "std")]

use core::fmt;

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Whether `c` is whitespace RFC 8259 has between tokens (section 2): space,
/// tab, line feed or carriage return.
pub fn is_json_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
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

/// Why a value has no form in the JSON Canonicalization Scheme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotCanonical(String);

impl NotCanonical {
    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NotCanonical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `value` in the JSON Canonicalization Scheme (RFC 8785): no whitespace, the
/// members of each object sorted by their names as arrays of UTF-16 code units
/// (section 3.2.3), each text in the one form every engine writes
/// ([`push_escaped`], which is ECMAScript's `JSON.stringify` of a text: section
/// 3.2.2.2), and each number as ECMAScript spells the double it reads as
/// ([`number_text`], section 3.2.2.3). Two values equal under the scheme are
/// then written as the same bytes, whatever order their members were built in.
///
/// Refused, naming why: a number past the range of a double, which the scheme
/// has no spelling for, and an object that names one member twice, which has
/// no single meaning (section 3.1).
pub fn write_canonical(value: &Value) -> Result<String, NotCanonical> {
    let mut out = String::new();
    write_canonical_into(value, &mut out)?;
    Ok(out)
}

fn write_canonical_into(value: &Value, out: &mut String) -> Result<(), NotCanonical> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(spelling) => {
            let number: f64 = spelling
                .parse()
                .map_err(|_| NotCanonical(format!("'{spelling}' is not a number")))?;
            if !number.is_finite() {
                return Err(NotCanonical(format!(
                    "the number {spelling} is past the range of a double, which RFC 8785 \
                     writes every number as"
                )));
            }
            out.push_str(&number_text(number));
        }
        Value::Text(s) => {
            out.push('"');
            push_escaped(out, s);
            out.push('"');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical_into(item, out)?;
            }
            out.push(']');
        }
        Value::Object(members) => {
            let mut sorted: Vec<&(String, Value)> = members.iter().collect();
            sorted.sort_by(|left, right| left.0.encode_utf16().cmp(right.0.encode_utf16()));
            if let Some(pair) = sorted.windows(2).find(|pair| pair[0].0 == pair[1].0) {
                return Err(NotCanonical(format!(
                    "the member \"{}\" appears twice",
                    pair[0].0
                )));
            }
            out.push('{');
            for (i, (key, item)) in sorted.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                push_escaped(out, key);
                out.push_str("\":");
                write_canonical_into(item, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// The JSON spelling of one text: escaped and quoted.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    push_escaped(&mut out, text);
    out.push('"');
    out
}

/// Append `text` to `out` as the body of a JSON string: escaped, unquoted.
///
/// The one form SCE writes on every engine (ARCHITECTURE.md, "JSON Text
/// (Single Source of Truth)"): `"` and `\` escaped, the five short forms
/// `\b \f \n \r \t`, every other U+0000-U+001F as `\u00xx` in lowercase hex,
/// everything else as it is. tests/json_text/string_escape.json holds the
/// cases.
pub fn push_escaped(out: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let byte = c as u32 as usize;
                out.push_str("\\u00");
                out.push(char::from(HEX[byte >> 4]));
                out.push(char::from(HEX[byte & 0x0F]));
            }
            c => out.push(c),
        }
    }
}

/// A 64-bit float as ECMAScript's `Number::toString` spells it, radix 10.
///
/// The one spelling SCE writes on every engine (ARCHITECTURE.md, "JSON Number
/// Text (Single Source of Truth)"): the fewest digits that read back as the
/// same double, in decimal notation when `1e-6 <= |x| < 1e21` and as
/// `d[.ddd]e[+-]n` otherwise, no fraction on a whole value and `0` for either
/// zero. A value that is not finite is spelled `NaN`, `Infinity` or
/// `-Infinity`, as `String(x)` does; a JSON writer, which has no spelling for
/// those, tests [`f64::is_finite`] before it calls this.
/// tests/json_text/real_text.json holds the cases.
pub fn number_text(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    // `{:e}` of a finite double is the shortest round-trip digits, `d[.ddd]e<n>`.
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("a finite `{:e}` has an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent
        .parse()
        .expect("a `{:e}` exponent is a whole number");
    // The value is `0.<digits> * 10^point`; the spec's `k` and `n`.
    let k = digits.len() as i32;
    let point = exponent + 1;
    let mut out = String::new();
    if value < 0.0 {
        out.push('-');
    }
    if k <= point && point <= 21 {
        out.push_str(&digits);
        out.push_str(&"0".repeat((point - k) as usize));
    } else if 0 < point && point <= 21 {
        out.push_str(&digits[..point as usize]);
        out.push('.');
        out.push_str(&digits[point as usize..]);
    } else if -6 < point && point <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat((-point) as usize));
        out.push_str(&digits);
    } else {
        let power = point - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if power < 0 { '-' } else { '+' });
        out.push_str(&power.abs().to_string());
    }
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

    /// Past the whitespace RFC 8259 has between tokens (section 2): space, tab,
    /// line feed and carriage return, and no other, so a no-break space or a
    /// vertical tab is a text that is not JSON on every runtime
    /// (tests/json_text/payload_refusal.json).
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if is_json_whitespace(c) {
                self.at += 1;
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
            // An object whose names repeat has no single meaning: a reader that
            // takes the first and one that takes the last would read two values
            // from one text, so it is refused (the Kotlin reader refuses it the
            // same way).
            if members
                .iter()
                .any(|(earlier, _): &(String, Value)| *earlier == key)
            {
                return Err(Self::malformed(&format!(
                    "the field \"{key}\" appears twice"
                )));
            }
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
                        let code = self.read_hex4()?;
                        // A character past U+FFFF is written as the two halves of
                        // its UTF-16 pair, each its own escape; a half alone names
                        // no character, so the text is no JSON text any runtime
                        // reads alike (RFC 8259, sections 7 and 8.2).
                        let character = match code {
                            0xD800..=0xDBFF => {
                                if !self.text[self.at..].starts_with("\\u") {
                                    return Err(Self::malformed(
                                        "a \\u escape is half of a character",
                                    ));
                                }
                                self.at += 2;
                                let low = self.read_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(Self::malformed(
                                        "a \\u escape is half of a character",
                                    ));
                                }
                                0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00)
                            }
                            0xDC00..=0xDFFF => {
                                return Err(Self::malformed("a \\u escape is half of a character"));
                            }
                            other => other,
                        };
                        match char::from_u32(character) {
                            Some(c) => out.push(c),
                            None => return Err(Self::malformed("a \\u escape names no character")),
                        }
                    }
                    Some(e) => {
                        return Err(Self::malformed(&format!("unknown escape '\\{e}'")));
                    }
                },
                // RFC 8259, section 7: a character below U+0020 stands in a text
                // only as an escape.
                Some(c) if (c as u32) < 0x20 => {
                    return Err(Self::malformed(
                        "a control character stands in a text unescaped",
                    ));
                }
                Some(c) => out.push(c),
            }
        }
    }

    /// The four hex digits after a `\u`, as the number they spell: exactly four
    /// of `0-9`, `a-f` and `A-F`, and no sign.
    fn read_hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self
            .text
            .as_bytes()
            .get(self.at..self.at + 4)
            .ok_or_else(|| Self::malformed("a \\u escape is short"))?;
        let mut code = 0u32;
        for &digit in digits {
            let value = char::from(digit)
                .to_digit(16)
                .ok_or_else(|| Self::malformed("a \\u escape is not hex"))?;
            code = code * 16 + value;
        }
        self.at += 4;
        Ok(code)
    }

    /// One number in RFC 8259's grammar (section 6): `-`, then `0` or a digit
    /// that is not `0` and more digits, then a fraction and an exponent, each
    /// with at least one digit. A leading zero, a fraction or an exponent with
    /// no digit, and a sign but `-` are no JSON number.
    fn read_number(&mut self) -> Result<String, JsonError> {
        let start = self.at;
        if self.peek() == Some('-') {
            self.bump();
        }
        match self.peek() {
            Some('0') => {
                self.bump();
            }
            Some(c) if c.is_ascii_digit() => self.skip_digits(),
            _ => return Err(Self::malformed("a number has no digits")),
        }
        if self.peek() == Some('.') {
            self.bump();
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err(Self::malformed("a fraction has no digits"));
            }
            self.skip_digits();
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.bump();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err(Self::malformed("an exponent has no digits"));
            }
            self.skip_digits();
        }
        // A digit right after the number is a number with a leading zero
        // (`01`), which no runtime reads alike.
        if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            return Err(Self::malformed("a number has a leading zero"));
        }
        Ok(self.text[start..self.at].to_string())
    }

    fn skip_digits(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
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

    /// An object whose names repeat has no single meaning — one reader takes
    /// the first, another the last — so no reader of this module takes it,
    /// nested or not.
    #[test]
    fn an_object_that_repeats_a_field_name_is_refused() {
        for text in [
            r#"{"a":1,"a":2}"#,
            r#"{"outer":{"a":1,"b":2,"a":3}}"#,
            r#"[{"a":1,"a":1}]"#,
        ] {
            let refusal = parse(text).expect_err(text);
            assert!(refusal.to_string().contains("appears twice"), "{refusal}");
        }
        assert!(
            parse(r#"{"a":1,"b":{"a":2}}"#).is_ok(),
            "the same name at another level is fine"
        );
    }

    /// tests/json_text/string_escape.json: the cases every engine's writer
    /// is measured against, read with this module's own parser.
    #[test]
    fn a_string_is_written_in_the_one_form_every_engine_writes() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/json_text/string_escape.json"
        );
        let table = std::fs::read_to_string(path).expect("the shared escape table");
        let Value::Object(members) = parse(&table).expect("the table is JSON") else {
            panic!("the table is an object");
        };
        let Some((_, Value::Array(cases))) = members.iter().find(|(key, _)| key == "cases") else {
            panic!("the table has cases");
        };
        assert!(cases.len() >= 8, "the table lost cases: {}", cases.len());
        for case in cases {
            let Value::Object(fields) = case else {
                panic!("a case is an object");
            };
            let text = |name: &str| match fields.iter().find(|(key, _)| key == name) {
                Some((_, Value::Text(text))) => text.clone(),
                other => panic!("a case's {name} is a string, got {other:?}"),
            };
            let (name, input, expected) = (text("name"), text("text"), text("escaped"));
            let mut escaped = String::new();
            push_escaped(&mut escaped, &input);
            assert_eq!(escaped, expected, "{name}");
            assert_eq!(quote(&input), format!("\"{expected}\""), "{name}");
        }
    }

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

    /// RFC 8259 alone, as every runtime reads it: a number in its grammar, a
    /// text with no raw control character and no half of a UTF-16 pair, and
    /// whitespace of the four kinds between tokens
    /// (tests/json_text/payload_refusal.json holds the same at every reader).
    #[test]
    fn what_is_not_rfc_8259_is_refused() {
        for text in [
            "01",
            "-01",
            "+1",
            ".5",
            "1.",
            "1e",
            "1e+",
            "-",
            "\"a\u{1}b\"",
            "\"a\tb\"",
            "\"\\ud800\"",
            "\"\\udc00\"",
            "\"\\ud800\\u0041\"",
            "\"\\u+041\"",
            "\"\\u12\u{e9}\"",
            "\u{a0}1",
            "\u{b}1",
            "1\u{a0}",
        ] {
            assert!(
                matches!(
                    parse(text),
                    Err(JsonError::Malformed(_)) | Err(JsonError::Trailing)
                ),
                "{text:?} was read"
            );
        }
        assert_eq!(parse("-0"), Ok(Value::Number("-0".to_string())));
        assert_eq!(parse("0.5e-3"), Ok(Value::Number("0.5e-3".to_string())));
        assert_eq!(
            parse("\"\\ud83d\\ude00\""),
            Ok(Value::Text("\u{1f600}".to_string()))
        );
        assert_eq!(parse(" \t\r\n1 \t\r\n"), Ok(Value::Number("1".to_string())));
    }

    fn text(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    /// RFC 8785 section 3.2.3, the example of the RFC: members are sorted by
    /// their names as UTF-16 code units, so U+1F600, whose first unit is
    /// 0xD83D, comes before U+FB33, which an order of code points would put
    /// after it; U+0080 is written as it is, since only U+0000 to U+001F are
    /// escaped.
    #[test]
    fn canonical_members_are_sorted_by_their_utf16_code_units() {
        let value = Value::Object(vec![
            ("\u{20ac}".to_string(), text("Euro Sign")),
            ("\r".to_string(), text("Carriage Return")),
            (
                "\u{fb33}".to_string(),
                text("Hebrew Letter Dalet With Dagesh"),
            ),
            ("1".to_string(), text("One")),
            ("\u{1f600}".to_string(), text("Emoji: Grinning Face")),
            ("\u{80}".to_string(), text("Control")),
            (
                "\u{f6}".to_string(),
                text("Latin Small Letter O With Diaeresis"),
            ),
        ]);
        assert_eq!(
            write_canonical(&value).expect("canonical"),
            "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{80}\":\"Control\",\
             \"\u{f6}\":\"Latin Small Letter O With Diaeresis\",\"\u{20ac}\":\"Euro Sign\",\
             \"\u{1f600}\":\"Emoji: Grinning Face\",\
             \"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}"
        );
    }

    /// RFC 8785 section 3.2.2.2: a text is escaped as `JSON.stringify` escapes
    /// it, the five short forms, `\u00xx` in lower-case hex for the other
    /// controls, and nothing else: not `/`, not U+007F, not a character past
    /// U+FFFF.
    #[test]
    fn a_canonical_text_escapes_only_what_json_stringify_escapes() {
        let value = text("\u{8}\t\n\u{c}\r\"\\\u{0}\u{1f}\u{7f}/\u{1f600}");
        assert_eq!(
            write_canonical(&value).expect("canonical"),
            "\"\\b\\t\\n\\f\\r\\\"\\\\\\u0000\\u001f\u{7f}/\u{1f600}\""
        );
    }

    /// Nested arrays and objects are written with no whitespace, each object's
    /// members sorted at its own level, an array's items in their order.
    #[test]
    fn a_canonical_value_has_no_whitespace_and_sorts_every_level() {
        let value = Value::Object(vec![
            (
                "b".to_string(),
                Value::Array(vec![
                    Value::Object(vec![
                        ("z".to_string(), Value::Null),
                        ("a".to_string(), Value::Bool(false)),
                    ]),
                    text("y"),
                    Value::Array(Vec::new()),
                ]),
            ),
            ("a".to_string(), Value::Object(Vec::new())),
        ]);
        assert_eq!(
            write_canonical(&value).expect("canonical"),
            "{\"a\":{},\"b\":[{\"a\":false,\"z\":null},\"y\",[]]}"
        );
    }

    /// RFC 8785 section 3.2.2.3: a number is written as ECMAScript writes the
    /// double it reads as; one past the range of a double is refused.
    #[test]
    fn a_canonical_number_is_the_double_it_reads_as() {
        for (spelling, written) in [
            ("1E3", "1000"),
            ("-0", "0"),
            ("0.10", "0.1"),
            ("1e21", "1e+21"),
            ("1e-7", "1e-7"),
        ] {
            assert_eq!(
                write_canonical(&Value::Number(spelling.to_string())),
                Ok(written.to_string()),
                "{spelling}"
            );
        }
        let refused = write_canonical(&Value::Number("1e400".to_string()))
            .expect_err("past the range of a double");
        assert!(refused.reason().contains("1e400"), "{refused}");
    }

    /// RFC 8785 section 3.1: an object that names one member twice has no
    /// single meaning, at whatever depth.
    #[test]
    fn a_canonical_object_names_each_member_once() {
        let twice = Value::Array(vec![Value::Object(vec![
            ("a".to_string(), text("1")),
            ("b".to_string(), text("2")),
            ("a".to_string(), text("3")),
        ])]);
        let refused = write_canonical(&twice).expect_err("a member twice");
        assert!(refused.reason().contains("\"a\""), "{refused}");
    }
}
