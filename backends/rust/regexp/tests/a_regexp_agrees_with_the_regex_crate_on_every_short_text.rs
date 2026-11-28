// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The language of an I-Regexp is the one RFC 9485 means (docs/adr/0149, D7): every expression of
//! a corpus is tested against every text over a small alphabet up to a length, and `matches` and
//! `search` agree with the `regex` crate given the expression's mapping (RFC 9485, section 5.4:
//! the dots outside classes as `[^\n\r]`, the whole in `\A(?:` and `)\z` for `matches`).
//!
//! The mapping writes every literal character as `\x{..}`. Section 5's mapping is not normative
//! (section 4 makes the meaning XSD-2's), and copied as written it would read `^` and `$` as
//! anchors and `&&`, `--` and `~~` in a class as set operations, where XSD-2 and RFC 9485's
//! grammar make each a character.
//!
//! The alphabets hold characters whose General_Category no Unicode version since 6.0 has moved,
//! so the oracle's own Unicode tables and the pin agree on them.

use std::fmt::Write as _;

use regex::Regex;
use sce_rust_regexp::Space;

struct Case {
    regexp: &'static str,
    alphabet: &'static [char],
    longest: usize,
}

const AB: &[char] = &['a', 'b'];
const SPACES: &[char] = &[' ', '\t', '\n', '\r', 'a'];
const DOTS: &[char] = &['a', 'b', '\n', '\r', '\u{2028}', '\u{10101}'];
/// The characters of the JSONPath compliance test suite's `match()` and `search()` cases.
const SUITE: &[char] = &['a', 'b', 'c', 'x', '.', '\\', '[', ']'];
const ANCHORS: &[char] = &['^', '$', 'a', 'b', 'c'];
const DIGITS: &[char] = &['0', '5', '9', '/', ':', 'a'];
/// One character of each General Category the corpus names, and a few more.
const CATEGORIES: &[char] = &[
    'A', 'a', '1', '_', '-', ' ', '\n', '\u{416}', '\u{436}', '\u{663}', '\u{a0}', '\u{20ac}',
    '\u{301}', '\u{2028}',
];

const fn case(regexp: &'static str, alphabet: &'static [char], longest: usize) -> Case {
    Case {
        regexp,
        alphabet,
        longest,
    }
}

const CORPUS: &[Case] = &[
    // RFC 9485's examples (sections 5.1 and 8).
    case(r"[^ \t\n\r]", SPACES, 3),
    case(r"[^\t\n\r]", SPACES, 3),
    case("[0-9]", DIGITS, 2),
    case(r"\p{Nd}", CATEGORIES, 2),
    case("a{2,4}", AB, 6),
    case("(a{2,4}){2,4}", AB, 9),
    case("(a{2,4}){2,4}", &['a'], 18),
    // The regular expressions of the JSONPath compliance test suite's match() and search() cases.
    case("a.*", SUITE, 4),
    case("b.?b", SUITE, 4),
    case(r"\p{Lu}", CATEGORIES, 2),
    case(r"\P{Lu}", CATEGORIES, 2),
    case("a.b", DOTS, 4),
    case(".", DOTS, 3),
    case("a[.b]c", SUITE, 4),
    case(r"a\.c", SUITE, 4),
    case(r"a\\.c", SUITE, 4),
    case(r"a\[.c", SUITE, 4),
    case(r"a[\].]c", SUITE, 4),
    case("^ab.*", ANCHORS, 4),
    case(".*bc$", ANCHORS, 4),
    case("[a-z]+", CATEGORIES, 2),
    // The families that drive a backtracking matcher to exponential time (D7).
    case("(a|a)*b", AB, 10),
    case("(a*)*b", AB, 10),
    case("(a|b)*a(a|b){10}", AB, 13),
    case(".*.*.*x", &['a', 'x', '\n'], 6),
    // The edges of the grammar and of the construction.
    case("", AB, 3),
    case("|", AB, 3),
    case("a|", AB, 3),
    case("()", AB, 3),
    case("(|)*", AB, 3),
    case("a{0}", AB, 3),
    case("a{0,0}b", AB, 3),
    case("(a{0})*b", AB, 3),
    case("a{2,}", AB, 5),
    case("a{0,}b", AB, 5),
    case("(ab){1,3}", AB, 7),
    case("(a|b){2}(ab|ba)?", AB, 6),
    case("((a|)b?){2,3}", AB, 6),
    case("x|y|z|", &['x', 'y', 'z', 'w'], 3),
    case("[-a]", &['-', 'a', 'b'], 3),
    case("[a-]", &['-', 'a', 'b'], 3),
    case("[^-]", &['-', 'a', 'b'], 3),
    case("[^^]", ANCHORS, 3),
    case(r"[\-a]", &['-', 'a', 'b'], 3),
    case("[!-/]", &['!', '-', '/', '0', ' '], 3),
    case(r"[\n-\r]", SPACES, 3),
    case("[a&&b]", &['a', '&', 'b', 'c'], 3),
    case(r"[a\-\-b]", &['a', '-', 'b', 'c'], 3),
    case("[a~~b]", &['a', '~', 'b', 'c'], 3),
    case(r"\.", SUITE, 3),
    case(r"\\", SUITE, 3),
    case("^$", ANCHORS, 3),
    case(r"[\p{L}\p{Nd}]+", CATEGORIES, 2),
    case(r"\P{L}", CATEGORIES, 2),
    case(r"[^\P{L}]", CATEGORIES, 2),
    case(r"[\P{L}\P{N}]", CATEGORIES, 2),
    case(r"[\p{Lu}-]", CATEGORIES, 2),
    case(r"\p{Zs}", CATEGORIES, 2),
    case(r"\p{Zl}", CATEGORIES, 2),
    case(r"\p{Sc}", CATEGORIES, 2),
    case(r"\p{Mn}", CATEGORIES, 2),
    case(r"\p{Pc}|\p{Pd}", CATEGORIES, 2),
    case(r"\p{C}", CATEGORIES, 2),
    case(r"\p{Cc}", CATEGORIES, 2),
    case(r"[^\p{L}\p{N}]*", CATEGORIES, 2),
];

/// A literal character in the `regex` crate's syntax: its code point, escaped.
fn literal(out: &mut String, c: char) {
    write!(out, "\\x{{{:X}}}", u32::from(c)).expect("a String takes every write");
}

/// RFC 9485 section 5.4's mapping into the `regex` crate's syntax, the body without its
/// anchors, every literal character escaped. The expression is an I-Regexp.
fn mapped(iregexp: &str) -> String {
    let mut out = String::new();
    let mut chars = iregexp.chars().peekable();
    // Inside a class: the items read so far; a hyphen after one and before no `]` makes a range.
    let mut class: Option<usize> = None;
    let mut quantifier = false;
    while let Some(c) = chars.next() {
        if quantifier {
            out.push(c);
            quantifier = c != '}';
            continue;
        }
        match (class, c) {
            (_, '\\') => {
                let escaped = chars.next().expect("an escape has its character");
                match escaped {
                    'p' | 'P' => {
                        out.push('\\');
                        out.push(escaped);
                        for named in chars.by_ref() {
                            out.push(named);
                            if named == '}' {
                                break;
                            }
                        }
                    }
                    'n' => literal(&mut out, '\n'),
                    'r' => literal(&mut out, '\r'),
                    't' => literal(&mut out, '\t'),
                    other => literal(&mut out, other),
                }
                if let Some(items) = class.as_mut() {
                    *items += 1;
                }
            }
            (None, '[') => {
                out.push('[');
                if chars.peek() == Some(&'^') {
                    chars.next();
                    out.push('^');
                }
                class = Some(0);
            }
            (Some(_), ']') => {
                out.push(']');
                class = None;
            }
            (Some(items), '-') if items > 0 && chars.peek() != Some(&']') => out.push('-'),
            (None, '.') => out.push_str(r"[^\n\r]"),
            (None, '(') => out.push_str("(?:"),
            (None, '{') => {
                out.push('{');
                quantifier = true;
            }
            (None, ')' | '|' | '*' | '+' | '?') => out.push(c),
            (_, other) => {
                literal(&mut out, other);
                if let Some(items) = class.as_mut() {
                    *items += 1;
                }
            }
        }
    }
    out
}

/// Every text over `alphabet` of at most `longest` characters, the empty one first.
fn texts(alphabet: &[char], longest: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut layer = vec![String::new()];
    for _ in 0..longest {
        layer = layer
            .iter()
            .flat_map(|text| {
                alphabet.iter().map(move |c| {
                    let mut longer = text.clone();
                    longer.push(*c);
                    longer
                })
            })
            .collect();
        all.extend(layer.iter().cloned());
    }
    all
}

#[test]
fn a_regexp_agrees_with_the_regex_crate_on_every_short_text() {
    let mut space = Box::new(Space::<1024, 64>::new());
    let mut compared = 0usize;
    for Case {
        regexp,
        alphabet,
        longest,
    } in CORPUS
    {
        let body = mapped(regexp);
        let whole = Regex::new(&format!(r"\A(?:{body})\z"))
            .unwrap_or_else(|error| panic!("{regexp:?} maps to {body:?}: {error}"));
        let part = Regex::new(&format!("(?:{body})"))
            .unwrap_or_else(|error| panic!("{regexp:?} maps to {body:?}: {error}"));
        let mut automaton = space
            .compile(regexp)
            .unwrap_or_else(|error| panic!("{regexp:?}: {error:?}"));
        for text in texts(alphabet, *longest) {
            assert_eq!(
                automaton.matches(text.as_bytes()),
                whole.is_match(&text),
                "matches({text:?}) under {regexp:?}, mapped to {body:?}"
            );
            assert_eq!(
                automaton.search(text.as_bytes()),
                part.is_match(&text),
                "search({text:?}) under {regexp:?}, mapped to {body:?}"
            );
            compared += 1;
        }
    }
    assert!(compared > 10_000, "{compared} texts compared");
}
