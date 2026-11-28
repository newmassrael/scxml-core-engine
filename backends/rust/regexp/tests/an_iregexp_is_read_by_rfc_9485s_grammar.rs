// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! RFC 9485's grammar (section 3), read whole. Every example the RFC gives is an I-Regexp; every
//! construct it leaves out (section 3's list, and section 1's capture groups, lookahead and
//! backreferences) is refused at the byte where the grammar stops; and section 8's large range
//! quantifier is read and refused for its size, not its form (docs/adr/0149, D1 and D3).

use sce_rust_regexp::{check, Error, Frame, Reason};

const BOUND: usize = 1024;

fn read(text: &str) -> Result<u64, Error> {
    let mut frames = [Frame::VACANT; 64];
    check(text, BOUND, &mut frames).map(|measure| measure.states)
}

/// Every General Category value and group RFC 9485's IsCategory names.
const CATEGORIES: [&str; 36] = [
    "L", "Ll", "Lm", "Lo", "Lt", "Lu", "M", "Mc", "Me", "Mn", "N", "Nd", "Nl", "No", "P", "Pc",
    "Pd", "Pe", "Pf", "Pi", "Po", "Ps", "Z", "Zl", "Zp", "Zs", "S", "Sc", "Sk", "Sm", "So", "C",
    "Cc", "Cf", "Cn", "Co",
];

#[test]
fn an_iregexp_is_read_by_rfc_9485s_grammar() {
    let accepted = [
        // Section 5.1, Table 1: the substitutes for multi-character escapes, and \p{Nd}.
        r"[^ \t\n\r]",
        r"[^\t\n\r]",
        "[0-9]",
        r"\p{Nd}",
        // Section 8: range quantifiers, nested.
        "a{2,4}",
        "(a{2,4}){2,4}",
        // The edges of the grammar: empty branches and groups, the hyphen first and last in a
        // class, `^` and `$` as characters, every SingleCharEsc, counts with leading zeros.
        "",
        "a",
        "|",
        "a|",
        "|a",
        "()",
        "(|)",
        "()*",
        "(((a)))",
        "(a|b)*c",
        "a{0}",
        "a{0,0}",
        "a{007}",
        "a{2,}",
        "a{0,}",
        "[-]",
        "[--]",
        "[a-]",
        "[-a]",
        "[^-]",
        "[^^]",
        "[a^]",
        r"[\^]",
        "^",
        "$",
        "^ab$",
        ".",
        "[.]",
        r"\.",
        r"\\",
        r"\(\)\*\+\-\.\?\[\\\]\^\{\|\}\n\r\t",
        r"[\]\-\[]",
        "[!-/]",
        r"[!-\-]",
        r"[\n-\r]",
        "a,b",
        "-",
        ",",
        "a-b",
        "[a-zA-Z0-9_]",
        r"[\p{L}\P{N}x]",
        "\u{e9}*",
        "\u{10101}",
        "\u{e000}",
        "[\u{e9}-\u{10ffff}]",
    ];
    for text in accepted {
        assert!(
            read(text).is_ok(),
            "{text:?} is an I-Regexp: {:?}",
            read(text)
        );
    }
    for name in CATEGORIES {
        for form in [
            format!("\\p{{{name}}}"),
            format!("\\P{{{name}}}"),
            format!("[\\p{{{name}}}]"),
            format!("[^\\P{{{name}}}a-z]"),
        ] {
            assert!(
                read(&form).is_ok(),
                "{form:?} names a category: {:?}",
                read(&form)
            );
        }
    }
    // Every other name of one or two letters is no category: a script, a block, `Cs`, `LC`.
    for major in 'A'..='Z' {
        let major_named = CATEGORIES.contains(&major.to_string().as_str());
        if !major_named {
            let text = format!("\\p{{{major}}}");
            let refused = Err(Error::Invalid {
                offset: 3,
                reason: Reason::Category,
            });
            assert_eq!(read(&text), refused, "{text:?}");
        }
        for minor in ('a'..='z').chain('A'..='Z') {
            let name = format!("{major}{minor}");
            if !CATEGORIES.contains(&name.as_str()) {
                let text = format!("\\p{{{name}}}");
                let refused = Err(Error::Invalid {
                    offset: if major_named { 4 } else { 3 },
                    reason: Reason::Category,
                });
                assert_eq!(read(&text), refused, "{text:?}");
            }
        }
    }

    let refused = [
        // Section 3's additional restriction, and the empty class it implies.
        ("[^]", 2, Reason::EmptyClass),
        ("[]", 1, Reason::EmptyClass),
        // Section 3: no multi-character escapes, no Unicode blocks, no class subtraction.
        (r"\d", 1, Reason::Escape),
        (r"\D", 1, Reason::Escape),
        (r"\s", 1, Reason::Escape),
        (r"\S", 1, Reason::Escape),
        (r"\w", 1, Reason::Escape),
        (r"\W", 1, Reason::Escape),
        (r"\i", 1, Reason::Escape),
        (r"\c", 1, Reason::Escape),
        (r"\p{IsBasicLatin}", 3, Reason::Category),
        (r"\p{Cs}", 4, Reason::Category),
        (r"\p{LC}", 4, Reason::Category),
        ("[a-z-[aeiou]]", 5, Reason::Unexpected),
        // Section 1: no capture groups' syntax, no lookahead, no backreferences.
        ("(?:a)", 1, Reason::NothingToRepeat),
        ("(?=a)", 1, Reason::NothingToRepeat),
        ("(?!a)", 1, Reason::NothingToRepeat),
        (r"(a)\1", 4, Reason::Escape),
        // No code point escape: a code point is written as itself.
        (r"\u0041", 1, Reason::Escape),
        (r"\x41", 1, Reason::Escape),
        // One quantifier to a piece: no lazy or possessive forms, no quantifier on nothing.
        ("a**", 2, Reason::NothingToRepeat),
        ("a*?", 2, Reason::NothingToRepeat),
        ("a+?", 2, Reason::NothingToRepeat),
        ("a{2}{3}", 4, Reason::NothingToRepeat),
        ("*a", 0, Reason::NothingToRepeat),
        ("a|*", 2, Reason::NothingToRepeat),
        ("(*)", 1, Reason::NothingToRepeat),
        ("{1}", 0, Reason::NothingToRepeat),
        // Range quantifiers: digits first, a most no less than the least.
        ("a{,2}", 2, Reason::Unexpected),
        ("a{}", 2, Reason::Unexpected),
        ("a{2,x}", 4, Reason::Unexpected),
        ("a{2 }", 3, Reason::Unexpected),
        ("a{3,2}", 1, Reason::QuantifierOutOfOrder),
        ("a{99999999999999999999,1}", 1, Reason::QuantifierOutOfOrder),
        (
            "a{99999999999999999999,99999999999999999998}",
            1,
            Reason::QuantifierOutOfOrder,
        ),
        // Classes: a range in order, a hyphen only where no range wants it, an escape a range
        // can end with.
        ("[b-a]", 1, Reason::RangeOutOfOrder),
        (r"[a-\-]", 1, Reason::RangeOutOfOrder),
        ("[a--]", 3, Reason::Unexpected),
        ("[---]", 3, Reason::Unexpected),
        ("[a-b-c]", 5, Reason::Unexpected),
        (r"[\p{L}-a]", 7, Reason::Unexpected),
        (r"[a-\p{L}]", 4, Reason::Escape),
        ("[a[b]", 2, Reason::Unexpected),
        // Characters with no place outside a class; an offset counts bytes.
        ("a)", 1, Reason::Unexpected),
        ("]", 0, Reason::Unexpected),
        ("}", 0, Reason::Unexpected),
        ("\u{e9})", 2, Reason::Unexpected),
        (r"\pL", 2, Reason::Unexpected),
        // A text that ends too soon.
        ("(a", 2, Reason::UnexpectedEnd),
        ("((a)", 4, Reason::UnexpectedEnd),
        ("[a", 2, Reason::UnexpectedEnd),
        ("[a-", 3, Reason::UnexpectedEnd),
        ("[^", 2, Reason::UnexpectedEnd),
        (r"\", 1, Reason::UnexpectedEnd),
        (r"[\", 2, Reason::UnexpectedEnd),
        (r"\p", 2, Reason::UnexpectedEnd),
        (r"\p{", 3, Reason::UnexpectedEnd),
        (r"\p{L", 4, Reason::UnexpectedEnd),
        (r"\p{Lu", 5, Reason::UnexpectedEnd),
        ("a{2", 3, Reason::UnexpectedEnd),
        ("a{2,", 4, Reason::UnexpectedEnd),
    ];
    for (text, offset, reason) in refused {
        assert_eq!(
            read(text),
            Err(Error::Invalid { offset, reason }),
            "{text:?} is no I-Regexp"
        );
    }

    // Section 8: `a{20,200000}` is an I-Regexp, refused for the states it expands to (200000
    // characters, 199980 splits and the accepting state), counted without building one.
    assert_eq!(
        read("a{20,200000}"),
        Err(Error::TooLarge {
            states: 399_981,
            bound: BOUND
        })
    );
    // A form error past the point where the count passes the bound is a form error still.
    assert_eq!(
        read("a{20,200000}("),
        Err(Error::Invalid {
            offset: 13,
            reason: Reason::UnexpectedEnd
        })
    );
}
