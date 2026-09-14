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
//! # Every value a template writes into a comment
//!
//! The annotation macro was the first consumer, and it was not the only
//! place author text reaches a comment. Measured 2026-09-13, 963
//! interpolations across the template tree sat inside a comment of the
//! language their template emits, and none was encoded: the C11 block
//! comment that echoes a `<log>` element's `expr` closed at a value's `*/`,
//! and the Go line comment that echoes a `<data>` element's `expr` put the
//! second line of a value into code.
//!
//! Writing the filter at each of those sites would be manual escaping — the
//! arrangement that forgets one, and the one this tree had. So the
//! generator encodes by CONTEXT instead. [`encode_template_comments`] runs
//! when a template is registered (`generator::register_template`), finds
//! every `{{ … }}` that [`crate::template_lexing`] places inside a comment of
//! the emitted language, and routes the whole value through [`filter`]. A
//! template says `// {{ action.cond }}` and no value can break that comment,
//! the way contextual autoescaping keeps a value inside the HTML attribute it
//! was written into.
//!
//! Three consequences are deliberate:
//!
//! - **A string-literal escaper inside a comment would run twice.** `escape_c`
//!   and then this encoder is neither encoding, so templates do not write one
//!   there, and `a_value_written_into_a_comment_is_encoded` refuses it.
//! - **A macro that spells its comment delimiter from a variable** writes a
//!   comment no reading of its text can see, so the annotation macro still
//!   writes `| comment_text` itself. The rewrite leaves a tag that already
//!   applies the filter last, to its whole value, as it is.
//! - **A value written into a string literal is not this module's.** Its
//!   encoder is the literal's escaper, which differs per language, and it is
//!   applied at the same door by [`crate::literal_text`]. The two passes
//!   compose because the classes do not overlap: a character is in a comment
//!   or in a literal, never both.

use std::borrow::Cow;

use crate::template_lexing::{
    applies_last_to_the_whole_value, interpolations, Class, Interpolation, Syntax,
};

/// The name the filter is registered under, and the one the rewrite writes.
pub const FILTER: &str = "comment_text";

/// `template` with every value it writes into a comment of `syntax` routed
/// through [`filter`]. Borrowed and unchanged when there is none.
///
/// No newline is added and each tag keeps its whitespace control, so the
/// rendered layout is the template's and a template error still names the
/// line its author wrote.
pub fn encode_template_comments(template: &str, syntax: Syntax) -> Cow<'_, str> {
    let sites: Vec<Interpolation> = interpolations(template, syntax)
        .into_iter()
        .filter(|site| {
            site.context == Class::Comment && !applies_last_to_the_whole_value(&site.tag, FILTER)
        })
        .collect();
    if sites.is_empty() {
        return Cow::Borrowed(template);
    }
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::with_capacity(template.len() + sites.len() * (FILTER.len() + 6));
    let mut at = 0usize;
    for site in &sites {
        out.extend(&chars[at..site.start]);
        out.push_str(&crate::template_lexing::routed_through(&site.tag, FILTER));
        at = site.end;
    }
    out.extend(&chars[at..]);
    Cow::Owned(out)
}

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

    fn rewrite(template: &str, syntax: Syntax) -> String {
        encode_template_comments(template, syntax).into_owned()
    }

    #[test]
    fn a_value_inside_a_comment_is_routed_through_the_filter() {
        assert_eq!(
            rewrite("/* {{ x }} */\n", Syntax::CFamily),
            "/* {{ (x) | comment_text }} */\n"
        );
        // Whitespace control survives on both sides, and an operator is kept
        // inside the parentheses so the filter covers the whole value.
        assert_eq!(
            rewrite("// {{- a ~ b -}}\n", Syntax::Go),
            "// {{- (a ~ b) | comment_text -}}\n"
        );
        assert_eq!(
            rewrite("# {{ x | default('-') }}\n", Syntax::Python),
            "# {{ (x | default('-')) | comment_text }}\n"
        );
    }

    #[test]
    fn a_value_anywhere_else_is_left_as_written() {
        for (template, syntax) in [
            ("int a = {{ x }};\n", Syntax::CFamily),
            ("const char *s = \"{{ x }}\";\n", Syntax::CFamily),
            ("{# // {{ x }} #}\n", Syntax::CFamily),
            ("{% raw %}// {{ x }}{% endraw %}\n", Syntax::Rust),
            ("// {{ x | comment_text }}\n", Syntax::Kotlin),
        ] {
            assert!(
                matches!(encode_template_comments(template, syntax), Cow::Borrowed(_)),
                "{template:?} was rewritten",
            );
        }
    }

    /// The whole door, rendered: registration rewrites, the environment knows
    /// the filter, and a hostile value stays inside the comment it was written
    /// into. The second line is a conditional whose value is undefined, which
    /// must render as nothing rather than fail.
    #[test]
    fn a_registered_template_renders_a_hostile_value_inside_its_comment() {
        let mut env = minijinja::Environment::new();
        crate::generator::register_template(
            &mut env,
            "probe.c.jinja2".to_string(),
            "/* expr=\"{{ e }}\" */\n/* [{{ 'x' if flag }}] */\n",
            crate::generator::Language::C11,
        )
        .expect("the rewritten template parses");
        let out = env
            .get_template("probe.c.jinja2")
            .expect("registered")
            .render(minijinja::context! { e => "a*/int b;/*", flag => false })
            .expect("renders");
        assert_eq!(out, "/* expr=\"a*\\x2Fint b;/\\x2A\" */\n/* [] */");
    }
}
