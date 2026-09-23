// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An attribute value as the document spells it, and where it sits.
//!
//! # What was wrong
//!
//! A reader hands a check an attribute's value DECODED — `&lt;` arrives
//! as `<`, and a value continued onto a second row arrives on one — and
//! the check decides on that value, which is right. A refusal then
//! REPORTS a piece of it: it names a token as its `actual` and places the
//! record on a row, and SCE_ERROR_CONTRACT.md §3.1.1 has a consumer find
//! that token on that row and edit it. Both halves came from the wrong
//! place. Measured 2026-09-22:
//!
//! - `negative_statechart_bytes_ordering.scxml` was refused with
//!   `actual: "<"` on the row of its `<transition` element. That row
//!   spells the operator `&lt;`; the only `<` on it is the element's own
//!   opening bracket. The §3.1.1 guard passed for exactly that reason, and
//!   a consumer applying the edit there would have broken the tag.
//! - a `cond` continued onto the rows below its `<transition` was refused
//!   with `actual: "_event.data.missing"` on the `<transition` row, two
//!   rows above the token, so its `replace_one_of` fix had nothing to
//!   replace.
//!
//! # The shape
//!
//! [`AttributeSpelling`] holds the value between its quotes exactly as
//! written, with the row and column its first character sits on. A model
//! keeps it beside the decoded value, `#[serde(skip)]` — the shape
//! [`crate::source_literal`] gives a number's spelling, and for the same
//! reason: the decoded value is what the machine computes with, the
//! spelling is what the author typed, and only the source knows the
//! second.
//!
//! [`AttributeSpelling::locate`] takes a byte range of the decoded value —
//! the span a parsed expression node carries — and answers the text that
//! produced it and the row and column that text starts on.
//!
//! ⚠ It answers `None` rather than guess. A range that does not fall on a
//! boundary of the decoding has no written counterpart, and a value that
//! reaches a DTD entity holds text the attribute does not spell — the
//! tree's readers refuse a DTD, so that second case is a defence rather
//! than a path. A refusal that cannot be placed exactly says less rather
//! than something false.

use std::ops::Range;

/// An attribute value as written, and the row and column it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSpelling {
    row: u32,
    col: u32,
    written: String,
}

/// A piece of an attribute value as written, and the row and column it
/// starts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written<'a> {
    pub text: &'a str,
    pub row: u32,
    pub col: u32,
}

impl<'a> Written<'a> {
    /// The text, when it lies on one row — the only shape a record may
    /// carry as `actual`, since §3.1.1 has a consumer find it on one line.
    pub fn on_one_row(&self) -> Option<&'a str> {
        (!self.text.contains(['\n', '\r'])).then_some(self.text)
    }
}

impl AttributeSpelling {
    /// The spelling of `node`'s attribute `local` in `namespace`, or `None`
    /// when the element carries no such attribute.
    pub fn of(node: &roxmltree::Node, namespace: Option<&str>, local: &str) -> Option<Self> {
        let attribute = node
            .attributes()
            .find(|a| a.name() == local && a.namespace() == namespace)?;
        let document = node.document();
        let range = attribute.range_value();
        let at = document.text_pos_at(range.start);
        Some(Self {
            row: at.row,
            col: at.col,
            written: document.input_text()[range].to_string(),
        })
    }

    /// The 1-based row the value's first character sits on.
    pub fn row(&self) -> u32 {
        self.row
    }

    /// The 1-based column the value's first character sits on.
    pub fn col(&self) -> u32 {
        self.col
    }

    /// Where `decoded[range]` was written, for `decoded` this value as the
    /// reader decoded it. `None` when the range does not fall on a boundary
    /// of the decoding, or the value reaches a DTD entity before its end.
    pub fn locate(&self, range: Range<usize>) -> Option<Written<'_>> {
        if range.start > range.end {
            return None;
        }
        let (mut decoded, mut at) = (0usize, 0usize);
        let mut start = None;
        loop {
            if start.is_none() && decoded == range.start {
                start = Some(at);
            }
            if let Some(from) = start {
                if decoded == range.end {
                    return Some(self.written(from, at));
                }
            }
            if decoded > range.end || (start.is_none() && decoded > range.start) {
                return None;
            }
            let (written, produced) = step(self.written.get(at..)?)?;
            at += written;
            decoded += produced.len_utf8();
        }
    }

    /// Whether this attribute, decoded, is `text` — up to the whitespace
    /// around it. A range of `text` reads back onto this spelling only then:
    /// a model that rewrote the value after reading it holds text nobody
    /// wrote here, and placing a range of it against this spelling would
    /// name a place that holds something else.
    pub fn spells(&self, text: &str) -> bool {
        let mut decoded = String::with_capacity(self.written.len());
        let mut at = 0usize;
        while at < self.written.len() {
            let Some((written, produced)) = self.written.get(at..).and_then(step) else {
                return false;
            };
            decoded.push(produced);
            at += written;
        }
        decoded.trim() == text.trim()
    }

    /// [`locate`](Self::locate) for a range of this value decoded and then
    /// trimmed — the text an expression parser is handed. The leading
    /// whitespace `str::trim` drops is counted on the decoding itself, so
    /// a model that stored its value already trimmed is placed as exactly
    /// as one that did not.
    pub fn locate_trimmed(&self, range: Range<usize>) -> Option<Written<'_>> {
        let (mut lead, mut at) = (0usize, 0usize);
        while at < self.written.len() {
            let (written, produced) = step(&self.written[at..])?;
            if !produced.is_whitespace() {
                break;
            }
            at += written;
            lead += produced.len_utf8();
        }
        self.locate(range.start + lead..range.end + lead)
    }

    /// `written[from..to]` with the position it starts at, counted the way
    /// `roxmltree::Document::text_pos_at` counts: a row per `\n`, a column
    /// per character.
    fn written(&self, from: usize, to: usize) -> Written<'_> {
        let before = &self.written[..from];
        let (row, col) = match before.rfind('\n') {
            Some(last) => (
                self.row + before.matches('\n').count() as u32,
                before[last + 1..].chars().count() as u32 + 1,
            ),
            None => (self.row, self.col + before.chars().count() as u32),
        };
        Written {
            text: &self.written[from..to],
            row,
            col,
        }
    }
}

/// One step of XML's attribute-value decoding (XML 1.0 §2.11, §3.3.3) at
/// the head of `rest`, as the bytes it reads and the character it decodes
/// to. These are the rules `roxmltree` applies, so a range of the value it
/// handed out maps back onto the text it read:
///
/// - a line break written `\r\n` decodes to one space, and a lone `\r`,
///   `\n` or `\t` to one space;
/// - a predefined entity or a character reference decodes to the
///   character it names, which is NOT normalized — `&#10;` stays a line
///   break;
/// - any other reference names a DTD entity, whose replacement text is not
///   written here, so the step has no answer.
fn step(rest: &str) -> Option<(usize, char)> {
    let bytes = rest.as_bytes();
    match bytes.first()? {
        b'\r' if bytes.get(1) == Some(&b'\n') => Some((2, ' ')),
        b'\r' | b'\n' | b'\t' => Some((1, ' ')),
        b'&' => {
            let end = rest.find(';')?;
            Some((end + 1, reference(&rest[1..end])?))
        }
        _ => {
            let c = rest.chars().next()?;
            Some((c.len_utf8(), c))
        }
    }
}

/// The character a reference's body (between `&` and `;`) names, or `None`
/// for a DTD entity.
fn reference(body: &str) -> Option<char> {
    match body {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => {
            let number = body.strip_prefix('#')?;
            let (digits, radix) = match number.strip_prefix('x') {
                Some(hex) => (hex, 16),
                None => (number, 10),
            };
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return None;
            }
            char::from_u32(u32::from_str_radix(digits, radix).ok()?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spelling_of(document: &str, local: &str) -> (String, AttributeSpelling) {
        let document = roxmltree::Document::parse(document).expect("the fixture parses");
        let node = document.root_element();
        let decoded = node.attribute(local).expect("the attribute").to_string();
        let spelling = AttributeSpelling::of(&node, None, local).expect("the attribute");
        (decoded, spelling)
    }

    #[test]
    fn an_escaped_operator_is_reported_as_the_entity() {
        let (decoded, spelling) = spelling_of(r#"<t cond="n &lt; 3"/>"#, "cond");
        assert_eq!(decoded, "n < 3");
        let at = decoded.find('<').unwrap();
        let written = spelling.locate(at..at + 1).expect("the operator");
        assert_eq!(written.text, "&lt;");
        assert_eq!((written.row, written.col), (1, 12));
    }

    #[test]
    fn a_token_on_a_continued_row_is_placed_on_that_row() {
        let (decoded, spelling) =
            spelling_of("<t\n   cond=\"a &amp;&amp;\r\n      b.c === 1\"/>", "cond");
        assert_eq!(decoded, "a &&       b.c === 1");
        let at = decoded.find("b.c").unwrap();
        let written = spelling.locate(at..at + 3).expect("the member");
        assert_eq!(written.text, "b.c");
        assert_eq!((written.row, written.col), (3, 7));
        assert_eq!(written.on_one_row(), Some("b.c"));
        let whole = spelling.locate(0..decoded.len()).expect("the whole value");
        assert_eq!(whole.on_one_row(), None, "{whole:?}");
    }

    /// A range of the trimmed value — what a parser is handed — is placed
    /// past the whitespace the trim dropped, however that was written.
    #[test]
    fn a_range_of_the_trimmed_value_skips_what_the_trim_dropped() {
        let (decoded, spelling) = spelling_of("<t v=\"&#10; \r\n\tx &lt; y \"/>", "v");
        let trimmed = decoded.trim();
        assert_eq!(trimmed, "x < y");
        let written = spelling.locate_trimmed(2..3).expect("the operator");
        assert_eq!(written.text, "&lt;");
        assert_eq!((written.row, written.col), (2, 4));
    }

    /// An attribute spells the value the reader decoded from it — escapes
    /// resolved, line breaks normalized, the surrounding whitespace aside —
    /// and nothing else: not its own escaped text, not a rewrite of it.
    #[test]
    fn an_attribute_spells_what_the_reader_decoded_and_nothing_else() {
        let (decoded, spelling) = spelling_of("<t v=\" a &amp;&amp;\r\n b \"/>", "v");
        assert!(spelling.spells(&decoded));
        assert!(spelling.spells(decoded.trim()));
        assert!(
            !spelling.spells("a &amp;&amp; b"),
            "the escaped text is not the value"
        );
        assert!(!spelling.spells("(a) && b"), "a rewrite is not the value");
    }

    #[test]
    fn a_character_reference_is_one_character_wide() {
        let (decoded, spelling) = spelling_of(r#"<t v="&#xE9;&#10;x"/>"#, "v");
        assert_eq!(decoded, "\u{e9}\nx");
        assert_eq!(spelling.locate(0..2).unwrap().text, "&#xE9;");
        assert_eq!(spelling.locate(2..3).unwrap().text, "&#10;");
        // One byte into the two-byte `é` is no boundary of the decoding.
        assert_eq!(spelling.locate(1..2), None);
    }

    /// The tree's readers refuse a DTD (`roxmltree`'s default), so no value
    /// they hand out reaches a declared entity. A reader that allowed one
    /// would get `None` past the entity rather than a wrong position.
    #[test]
    fn a_dtd_entity_is_not_placed() {
        let options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let document = roxmltree::Document::parse_with_options(
            "<!DOCTYPE t [<!ENTITY op \"==\">]><t v=\"a &op; b\"/>",
            options,
        )
        .expect("the fixture parses with a DTD allowed");
        let node = document.root_element();
        let decoded = node.attribute("v").expect("the attribute");
        let spelling = AttributeSpelling::of(&node, None, "v").expect("the attribute");
        assert_eq!(decoded, "a == b");
        assert_eq!(spelling.locate(0..1).unwrap().text, "a");
        assert_eq!(spelling.locate(2..4), None);
    }

    /// Every range the reader's decoding can produce is placed where
    /// `roxmltree` itself says its text starts, and that text decodes back
    /// to the range — for values mixing each rule `step` names.
    #[test]
    fn every_range_maps_back_to_what_the_reader_read() {
        let source = "<t\n a=\"x &lt;= y &amp;&amp;\r\n\tz\"\n b='say \"&#233;t&#xE9;\"&#10;ok'\n c=\"plain\"/>";
        let document = roxmltree::Document::parse(source).expect("the fixture parses");
        let node = document.root_element();
        for attribute in node.attributes() {
            let decoded = attribute.value();
            let spelling = AttributeSpelling::of(&node, None, attribute.name()).unwrap();
            let value_start = attribute.range_value().start;
            let boundaries: Vec<usize> = decoded
                .char_indices()
                .map(|(i, _)| i)
                .chain([decoded.len()])
                .collect();
            for (i, &from) in boundaries.iter().enumerate() {
                for &to in &boundaries[i..] {
                    let written = spelling
                        .locate(from..to)
                        .unwrap_or_else(|| panic!("{}: {from}..{to} unplaced", attribute.name()));
                    let offset =
                        written.text.as_ptr() as usize - spelling.written.as_ptr() as usize;
                    let expected = document.text_pos_at(value_start + offset);
                    assert_eq!(
                        (written.row, written.col),
                        (expected.row, expected.col),
                        "{}: {from}..{to} = {:?}",
                        attribute.name(),
                        written.text,
                    );
                    let quote = if written.text.contains('"') {
                        '\''
                    } else {
                        '"'
                    };
                    let again = format!("<t v={quote}{}{quote}/>", written.text);
                    let again = roxmltree::Document::parse(&again).expect("a piece parses");
                    assert_eq!(
                        again.root_element().attribute("v"),
                        Some(&decoded[from..to]),
                        "{}: {from}..{to} = {:?}",
                        attribute.name(),
                        written.text,
                    );
                }
            }
        }
    }
}
