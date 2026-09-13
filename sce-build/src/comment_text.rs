// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Text placed inside a comment in generated source.
//!
//! Generated code carries author text in comments — the `sce:req`,
//! `sce:provenance` and `sce:unresolved` annotations of
//! `docs/SCE_ACCEPTED_SUBSET.md` §2.10 — and that text is opaque by
//! contract: a requirement id may be any run of non-whitespace, and an
//! unresolved-marker reason any string at all, including a newline an
//! author writes as `&#10;`. Nothing encoded it for the comment it lands
//! in. Measured 2026-09-13 with one hostile document through all six
//! backends:
//!
//! - an id `A*/B` closed the C11 block comment and left `B` as code;
//! - a reason carrying a newline put its second line in code in the five
//!   line-comment backends — Python then failed to compile, and Go parsed
//!   the injected line as a statement, which is the worse of the two;
//! - an id ending in `\` spliced the next C++ line into the comment.
//!
//! # The grammar — one spelling for every backend
//!
//! [`encode`] writes exactly these as `\xHH` and passes every other
//! character through unchanged:
//!
//! | character | written as | why |
//! |---|---|---|
//! | `\` | `\x5C` | the escape introducer, so it must be encoded itself — which also means encoded text never ends in `\`, the C/C++ line splice |
//! | LF | `\x0A` | ends a `//` or `#` comment |
//! | CR | `\x0D` | ends a `//` or `#` comment |
//! | the `/` of `*/` | `\x2F` | closes a block comment in every C-family language |
//! | the `*` of `/*` | `\x2A` | opens a NESTED block comment in Rust and Kotlin, which nest |
//!
//! Text containing none of those is returned borrowed and byte-identical,
//! so an ordinary id costs nothing and no committed output moves.
//!
//! One grammar rather than one per backend because the payload is a
//! traceability link: a reader recovering `sce:req` ids from generated C
//! and from generated Python must get the same ids back with one decoder,
//! and [`decode`] is that decoder. It refuses a backslash that does not
//! introduce one of the five escapes, so a payload that was not produced
//! by [`encode`] reads as foreign rather than as a different id.
//!
//! ⚠ What the grammar guarantees is about the VALUE: an encoded value
//! contains no line terminator, no `*/`, no `/*`, and does not end in
//! `\`. A template still owns what sits next to it — a value written
//! flush against a closing `*/` with no separating space is the
//! template's defect, not the encoder's.
//!
//! # Scope, stated rather than hidden
//!
//! The annotation macro (`tools/codegen/templates/_macros/
//! sce_annotation_marker.jinja2`) is the first consumer. It is not the
//! only place author text reaches a comment: other templates echo author
//! attributes verbatim, and the C11 `/* W3C SCXML 4.4: <log expr="…"> */`
//! echo was measured breaking on `*/` the same day, the same way. Those
//! sites are not yet routed through this encoder. Re-derive their extent
//! by lexing `tools/codegen/templates/` for interpolations that fall inside
//! a target-language comment, rather than trusting a count written here.

use std::borrow::Cow;

/// Encode `text` for a comment body. See the module docs for the grammar.
pub fn encode(text: &str) -> Cow<'_, str> {
    if !needs_encoding(text) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut previous: Option<char> = None;
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\x5C"),
            '\n' => out.push_str("\\x0A"),
            '\r' => out.push_str("\\x0D"),
            '/' if previous == Some('*') => out.push_str("\\x2F"),
            '*' if previous == Some('/') => out.push_str("\\x2A"),
            _ => out.push(c),
        }
        previous = Some(c);
    }
    Cow::Owned(out)
}

/// Decode a comment body written by [`encode`].
///
/// `None` when a backslash introduces anything but the five escapes the
/// grammar defines — the payload was not produced by [`encode`].
pub fn decode(encoded: &str) -> Option<String> {
    let mut out = String::with_capacity(encoded.len());
    let mut chars = encoded.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        if chars.next()? != 'x' {
            return None;
        }
        let decoded = match (chars.next()?, chars.next()?) {
            ('5', 'C') => '\\',
            ('0', 'A') => '\n',
            ('0', 'D') => '\r',
            ('2', 'F') => '/',
            ('2', 'A') => '*',
            _ => return None,
        };
        out.push(decoded);
    }
    Some(out)
}

/// The template filter: `{{ value | comment_text }}`.
pub fn filter(value: String) -> String {
    encode(&value).into_owned()
}

fn needs_encoding(text: &str) -> bool {
    text.contains(['\\', '\n', '\r']) || text.contains("*/") || text.contains("/*")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What an encoded value must never contain, whatever went in.
    fn assert_inert(original: &str, encoded: &str) {
        assert!(
            !encoded.contains(['\n', '\r']),
            "{original:?} encoded to {encoded:?}, which still breaks a line comment",
        );
        assert!(
            !encoded.contains("*/"),
            "{original:?} encoded to {encoded:?}, which still closes a block comment",
        );
        assert!(
            !encoded.contains("/*"),
            "{original:?} encoded to {encoded:?}, which still opens a nested block comment",
        );
        assert!(
            !encoded.ends_with('\\'),
            "{original:?} encoded to {encoded:?}, which still splices the next C/C++ line",
        );
    }

    #[test]
    fn ordinary_ids_pass_through_borrowed_and_unchanged() {
        for id in [
            "3.DoIP-152",
            "REQ_AB_12345",
            "ns:req.1",
            "REQ/1",
            "REQ#9",
            "a*b",
            "a/b",
            "OEM-DIAG-SPEC@D#3.4.2:page=112",
        ] {
            let encoded = encode(id);
            assert!(
                matches!(encoded, Cow::Borrowed(_)),
                "{id:?} needed no encoding but was copied",
            );
            assert_eq!(encoded, id);
        }
    }

    #[test]
    fn the_measured_hostile_values_are_inert_and_round_trip() {
        for hostile in [
            "A*/INJ_BLOCK",
            "one\nINJ_NL = 1",
            "one\r\nINJ_CRLF",
            "SPLICE\\",
            "/*INJ_NEST",
            "*/*/",
            "/*/",
            "\\x5C",
        ] {
            let encoded = encode(hostile);
            assert_inert(hostile, &encoded);
            assert_eq!(decode(&encoded).as_deref(), Some(hostile));
        }
    }

    /// Every string over the characters the grammar is about, up to length
    /// five. The hostile list above is what was measured; this is what
    /// makes the guarantee hold for inputs nobody thought to write down,
    /// including the overlapping `/*/` and `*/*` shapes.
    #[test]
    fn every_short_string_over_the_hazardous_alphabet_is_inert_and_round_trips() {
        const ALPHABET: [char; 6] = ['*', '/', '\\', '\n', '\r', 'a'];
        let mut checked = 0usize;
        let mut frontier = vec![String::new()];
        for _ in 0..5 {
            let mut next = Vec::with_capacity(frontier.len() * ALPHABET.len());
            for prefix in &frontier {
                for c in ALPHABET {
                    let mut s = prefix.clone();
                    s.push(c);
                    let encoded = encode(&s);
                    assert_inert(&s, &encoded);
                    assert_eq!(decode(&encoded).as_deref(), Some(s.as_str()));
                    checked += 1;
                    next.push(s);
                }
            }
            frontier = next;
        }
        assert_eq!(checked, 6 + 36 + 216 + 1296 + 7776);
    }

    #[test]
    fn decode_refuses_a_payload_encode_did_not_write() {
        for foreign in ["\\", "\\q", "\\x", "\\x5", "\\x41", "tail\\"] {
            assert_eq!(decode(foreign), None, "{foreign:?} must be refused");
        }
    }
}
