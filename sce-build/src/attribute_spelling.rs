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
//!
//! # An element's character data
//!
//! A `<script>` or `<finalize>` body is an expression read from an
//! element's character data rather than from an attribute, and its
//! refusals were placed on the element's row whatever row the refused
//! token sat on. [`AttributeSpelling::of_character_data`] spells that text
//! the same way: the element's content as written, decoded by the rules of
//! character data rather than of an attribute value — line breaks are
//! kept, a CDATA section is literal, and a comment or processing
//! instruction contributes nothing. Every question above is then answered
//! for a body exactly as for an attribute.

use std::ops::Range;

/// An attribute value — or an element's character data — as written, and
/// the row and column it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSpelling {
    row: u32,
    col: u32,
    written: String,
    decoding: Decoding,
}

/// The rules a spelling's text decodes by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decoding {
    /// An attribute value (XML 1.0 §3.3.3).
    Attribute,
    /// An element's character data (XML 1.0 §2.4, §2.7, §2.11).
    CharacterData,
}

/// One decoded character and the written bytes that produced it.
#[derive(Debug, Clone, Copy)]
struct Unit {
    at: usize,
    len: usize,
    produced: char,
}

/// A spelling decoded as far as its decoding reaches: every unit up to the
/// end, or up to the first text that has no answer here — a DTD entity,
/// whose replacement text is not written in the spelling.
struct Decoded {
    units: Vec<Unit>,
    complete: bool,
}

/// The attributes of one element that carry no namespace, each as written
/// and where.
///
/// For a model struct that stands for several kinds of element — an
/// `Action` is a `<send>`, an `<assign>`, a `<foreach>` — so one field
/// serves whichever attributes its kind carries, rather than a field per
/// attribute of every kind. A struct that stands for one element kind names
/// its attribute instead, as `Transition::cond_spelling` does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributeSpellings(Vec<(String, AttributeSpelling)>);

impl AttributeSpellings {
    /// Every attribute of `node` that carries no namespace.
    pub fn of(node: &roxmltree::Node) -> Self {
        Self(
            node.attributes()
                .filter(|attribute| attribute.namespace().is_none())
                .filter_map(|attribute| {
                    AttributeSpelling::of(node, None, attribute.name())
                        .map(|spelling| (attribute.name().to_string(), spelling))
                })
                .collect(),
        )
    }

    /// The spelling of the attribute `local`, when the element carries it.
    pub fn get(&self, local: &str) -> Option<&AttributeSpelling> {
        self.0
            .iter()
            .find(|(name, _)| name == local)
            .map(|(_, spelling)| spelling)
    }
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
            decoding: Decoding::Attribute,
        })
    }

    /// The character data of `node` — every text and CDATA child, in
    /// document order, and nothing of a comment or processing instruction —
    /// as written, with the row and column it starts on.
    ///
    /// `None` for an element with no content, and for one with an element
    /// child, whose text around the child this does not follow. It is also
    /// `None` unless the decoding reads back to exactly the character data
    /// the tree holds, so a body the check reads and the spelling that
    /// places its refusals cannot disagree.
    pub fn of_character_data(node: &roxmltree::Node) -> Option<Self> {
        if node.children().any(|child| child.is_element()) {
            return None;
        }
        let document = node.document();
        let input = document.input_text();
        let start = node.first_child()?.range().start;
        let element = &input[node.range()];
        let end = node.range().start + element.rfind("</")?;
        let at = document.text_pos_at(start);
        let spelling = Self {
            row: at.row,
            col: at.col,
            written: input.get(start..end)?.to_string(),
            decoding: Decoding::CharacterData,
        };
        let held: String = node
            .children()
            .filter(|child| child.is_text())
            .filter_map(|child| child.text())
            .collect();
        (spelling.decoded()? == held).then_some(spelling)
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
        // Only the decoded prefix is placed: a range reaching past a DTD
        // entity runs out of units before its end and has no answer.
        let Decoded { units, .. } = self.units();
        // The written offset a decoded offset starts at, and the one it ends
        // at: the same place, except where a unit produced nothing — a
        // comment between two characters of a body — which belongs to
        // neither side of the boundary.
        let (mut decoded, mut from, mut to) = (0usize, None, None);
        let mut previous_end = 0usize;
        for unit in &units {
            if decoded == range.start && from.is_none() {
                from = Some(unit.at);
            }
            if decoded == range.end && from.is_some() {
                to = Some(previous_end);
                break;
            }
            if decoded > range.end {
                return None;
            }
            decoded += unit.produced.len_utf8();
            previous_end = unit.at + unit.len;
        }
        if decoded == range.start && from.is_none() {
            from = Some(previous_end);
        }
        if decoded == range.end && to.is_none() {
            to = Some(previous_end);
        }
        let (from, to) = (from?, to?);
        (from <= to).then(|| self.written(from, to))
    }

    /// This spelling decoded, as the characters it produces and the written
    /// bytes each came from — up to the first text with no answer here, a
    /// DTD entity or a markup construct character data does not hold.
    fn units(&self) -> Decoded {
        let mut units = Vec::with_capacity(self.written.len());
        let mut at = 0usize;
        while at < self.written.len() {
            let next = match self.decoding {
                Decoding::Attribute => step(&self.written[at..]).map(|(len, produced)| {
                    units.push(Unit { at, len, produced });
                    at + len
                }),
                Decoding::CharacterData => character_data_step(&self.written, at, &mut units),
            };
            match next {
                Some(next) => at = next,
                None => {
                    return Decoded {
                        units,
                        complete: false,
                    }
                }
            }
        }
        Decoded {
            units,
            complete: true,
        }
    }

    /// Whether this attribute, decoded, is `text` — up to the whitespace
    /// around it. A range of `text` reads back onto this spelling only then:
    /// a model that rewrote the value after reading it holds text nobody
    /// wrote here, and placing a range of it against this spelling would
    /// name a place that holds something else.
    pub fn spells(&self, text: &str) -> bool {
        self.decoded()
            .is_some_and(|decoded| decoded.trim() == text.trim())
    }

    /// The whole value as written, without the whitespace around it, and the
    /// row and column it starts on — what a refusal of the attribute's whole
    /// value names. `None` when the value reaches a DTD entity.
    pub fn value(&self) -> Option<Written<'_>> {
        let decoded = self.decoded()?;
        self.locate_trimmed(0..decoded.trim().len())
    }

    /// This value as the reader decoded it, or `None` when it reaches a DTD
    /// entity, whose replacement text is not written here.
    fn decoded(&self) -> Option<String> {
        let Decoded { units, complete } = self.units();
        complete.then(|| units.iter().map(|unit| unit.produced).collect())
    }

    /// [`locate`](Self::locate) for a range of this value decoded and then
    /// trimmed — the text an expression parser is handed. The leading
    /// whitespace `str::trim` drops is counted on the decoding itself, so
    /// a model that stored its value already trimmed is placed as exactly
    /// as one that did not.
    pub fn locate_trimmed(&self, range: Range<usize>) -> Option<Written<'_>> {
        let Decoded { units, complete } = self.units();
        let leading = units
            .iter()
            .take_while(|unit| unit.produced.is_whitespace())
            .count();
        // Whitespace up to a DTD entity may go on into its replacement text,
        // so how much the trim dropped is not known here.
        if leading == units.len() && !complete {
            return None;
        }
        let lead: usize = units[..leading]
            .iter()
            .map(|unit| unit.produced.len_utf8())
            .sum();
        self.locate(range.start + lead..range.end + lead)
    }

    /// What `range` of this value, decoded and then trimmed, was written as,
    /// as a spelling of its own — for a value that holds several expressions
    /// (`<sce:call args>`), so each is placed exactly as an attribute holding
    /// only that one would be. `None` where
    /// [`locate_trimmed`](Self::locate_trimmed) has no answer.
    pub fn piece_trimmed(&self, range: Range<usize>) -> Option<Self> {
        self.locate_trimmed(range).map(|written| Self {
            row: written.row,
            col: written.col,
            written: written.text.to_string(),
            decoding: self.decoding,
        })
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

/// One step of character-data decoding at `written[at..]`: pushes the units
/// it produces and answers where the next step starts, or `None` for text
/// character data does not hold — a DTD entity, or an element's tag. The
/// rules the tree's reader applies:
///
/// - a line break written `\r\n`, or a lone `\r`, decodes to `\n` (XML 1.0
///   §2.11), inside a CDATA section as everywhere else;
/// - a CDATA section's content decodes to itself, its markers to nothing;
/// - a comment or processing instruction decodes to nothing;
/// - a predefined entity or a character reference to the character it names.
fn character_data_step(written: &str, at: usize, units: &mut Vec<Unit>) -> Option<usize> {
    const CDATA_OPEN: &str = "<![CDATA[";
    let rest = &written[at..];
    if let Some(body) = rest.strip_prefix(CDATA_OPEN) {
        let body_at = at + CDATA_OPEN.len();
        let close = body.find("]]>")?;
        let mut i = 0usize;
        while i < close {
            let chunk = &body[i..close];
            let (len, produced) = match line_break(chunk) {
                Some(len) => (len, '\n'),
                None => {
                    let c = chunk.chars().next()?;
                    (c.len_utf8(), c)
                }
            };
            units.push(Unit {
                at: body_at + i,
                len,
                produced,
            });
            i += len;
        }
        return Some(body_at + close + "]]>".len());
    }
    if rest.starts_with("<!--") {
        return Some(at + rest.find("-->")? + "-->".len());
    }
    if rest.starts_with("<?") {
        return Some(at + rest.find("?>")? + "?>".len());
    }
    if rest.starts_with('<') {
        return None;
    }
    let (len, produced) = match line_break(rest) {
        Some(len) => (len, '\n'),
        None if rest.starts_with('&') => {
            let end = rest.find(';')?;
            (end + 1, reference(&rest[1..end])?)
        }
        None => {
            let c = rest.chars().next()?;
            (c.len_utf8(), c)
        }
    };
    units.push(Unit { at, len, produced });
    Some(at + len)
}

/// The written length of the line break at the head of `rest` — `\r\n` or a
/// lone `\r` or `\n` — or `None` when it holds none there.
fn line_break(rest: &str) -> Option<usize> {
    match rest.as_bytes() {
        [b'\r', b'\n', ..] => Some(2),
        [b'\r', ..] | [b'\n', ..] => Some(1),
        _ => None,
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

    /// The whole value as written — its references as written, the
    /// whitespace around it dropped — placed at its first character.
    #[test]
    fn a_whole_value_is_placed_where_its_first_character_is() {
        let (decoded, spelling) = spelling_of("<t\n  v=\"  4&#48;\n\"/>", "v");
        assert_eq!(decoded.trim(), "40");
        let value = spelling.value().expect("the value");
        assert_eq!((value.text, value.row, value.col), ("4&#48;", 2, 8));
    }

    /// A piece of a value is placed as an attribute holding only that piece
    /// would be: on the row it continues onto, and spelled as written.
    #[test]
    fn a_piece_reads_back_as_its_own_attribute() {
        let (decoded, spelling) =
            spelling_of("<t\n  args=\" a &amp;&amp; b,\n    c &lt; 1\"/>", "args");
        let trimmed = decoded.trim();
        let at = trimmed.find('c').expect("the second argument");
        let piece = spelling
            .piece_trimmed(at..trimmed.len())
            .expect("the second argument is written");
        assert!(piece.spells("c < 1"));
        assert_eq!((piece.row(), piece.col()), (3, 5));
        let operator = piece.locate_trimmed(2..3).expect("the operator");
        assert_eq!((operator.text, operator.row, operator.col), ("&lt;", 3, 7));
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

    /// The character data of the root element of `document`, as the tree
    /// holds it, and its spelling.
    fn body_of(document: &str) -> (String, Option<AttributeSpelling>) {
        let document = roxmltree::Document::parse(document).expect("the fixture parses");
        let node = document.root_element();
        let held = node
            .children()
            .filter(|child| child.is_text())
            .filter_map(|child| child.text())
            .collect();
        (held, AttributeSpelling::of_character_data(&node))
    }

    /// A statement on the third row of a body is placed on that row, at its
    /// column — not at the `<script>` tag.
    #[test]
    fn a_token_in_a_body_is_placed_on_its_row() {
        let (held, spelling) = body_of("<script>\n  a = 1;\n  b = conut;\n</script>");
        let spelling = spelling.expect("a body");
        let at = held.find("conut").unwrap();
        let written = spelling.locate(at..at + 5).expect("the name");
        assert_eq!((written.text, written.row, written.col), ("conut", 3, 7));
        let trimmed = held.trim();
        let at = trimmed.find("conut").unwrap();
        let written = spelling.locate_trimmed(at..at + 5).expect("the name");
        assert_eq!((written.row, written.col), (3, 7));
    }

    /// Every way a body is written decodes to the text the tree holds, and a
    /// range of that text is placed at the spelling that produced it: an
    /// entity as the entity, a CDATA section's content as itself, a `\r\n` as
    /// the one line break it decodes to, a comment as nothing.
    #[test]
    fn a_body_decodes_to_the_text_the_tree_holds() {
        let (held, spelling) = body_of(
            "<script>x = a &lt; b;\r\n<!-- note --><![CDATA[y = c < d;\r\n]]>z = e;</script>",
        );
        assert_eq!(held, "x = a < b;\ny = c < d;\nz = e;");
        let spelling = spelling.expect("a body");
        let entity = held.find('<').unwrap();
        let written = spelling.locate(entity..entity + 1).expect("the entity");
        assert_eq!((written.text, written.row, written.col), ("&lt;", 1, 15));
        let literal = held.rfind('<').unwrap();
        let written = spelling.locate(literal..literal + 1).expect("the literal");
        assert_eq!((written.text, written.row, written.col), ("<", 2, 29));
        let last = held.find('z').unwrap();
        let written = spelling.locate(last..last + 1).expect("the last row");
        assert_eq!((written.text, written.row, written.col), ("z", 3, 4));
    }

    /// An element child is text around a child this does not follow, and an
    /// empty element has nothing to spell.
    #[test]
    fn a_body_with_an_element_or_nothing_has_no_spelling() {
        assert!(body_of("<script>a<b/>c</script>").1.is_none());
        assert!(body_of("<script></script>").1.is_none());
        assert!(body_of("<script/>").1.is_none());
    }
}
