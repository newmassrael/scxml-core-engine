// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `search` is true when some part of the text is in the language, the text around that part
//! being any well-formed text (docs/adr/0149, D1 and D3; RFC 9535, section 2.4.7). A part is a run
//! of code points, never part of one, and a byte sequence that is not UTF-8 (RFC 3629, section 4)
//! is no text: neither test accepts it, wherever it stands from the part.

use sce_rust_regexp::Space;

/// `matches` and `search` of `regexp` on `text`.
fn answers(regexp: &str, text: &[u8]) -> (bool, bool) {
    let mut space = Space::<64, 32>::new();
    let mut automaton = space
        .compile(regexp)
        .unwrap_or_else(|error| panic!("{regexp:?}: {error:?}"));
    (automaton.matches(text), automaton.search(text))
}

#[test]
fn search_finds_a_part_and_only_a_well_formed_text_around_it() {
    // A part at the start, in the middle, at the end, the whole, nowhere.
    assert_eq!(answers("ab", b"abyy"), (false, true));
    assert_eq!(answers("ab", b"xaby"), (false, true));
    assert_eq!(answers("ab", b"xxab"), (false, true));
    assert_eq!(answers("ab", b"ab"), (true, true));
    assert_eq!(answers("ab", b"a b"), (false, false));
    assert_eq!(answers("ab", b"ba"), (false, false));
    // A part that begins where an earlier attempt failed.
    assert_eq!(answers("aab", b"aaab"), (false, true));
    // The empty part is in every text, the empty one too.
    assert_eq!(answers("", b""), (true, true));
    assert_eq!(answers("", b"xyz"), (false, true));
    assert_eq!(answers("a*", b"bbb"), (false, true));

    // A part is code points. U+00E9 is C3 A9 and U+00A9 is C2 A9: a reader of bytes would find
    // the second in the first; no part of U+00E9 is U+00A9.
    assert_eq!(answers("\u{a9}", "\u{e9}".as_bytes()), (false, false));
    assert_eq!(answers("\u{e9}", "x\u{e9}y".as_bytes()), (false, true));
    // A dot reads one code point of any length, a supplementary one too.
    assert_eq!(answers(".", "\u{10101}".as_bytes()), (true, true));
    assert_eq!(answers("a.b", "a\u{10101}b".as_bytes()), (true, true));
    assert_eq!(
        answers("a.b", "a\u{10101}\u{10101}b".as_bytes()),
        (false, false)
    );
    assert_eq!(answers(r"\p{Sc}", "1 \u{20ac}".as_bytes()), (false, true));

    // Around a part, only well-formed UTF-8: an overlong form, a surrogate, a code point past
    // U+10FFFF, a continuation byte with no lead, a lead with no continuation, and a byte that is
    // never UTF-8, each before the part, right after it and further on.
    let ill_formed: [&[u8]; 7] = [
        b"\xc0\x80",
        b"\xe0\x80\x80",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"\x80",
        b"\xc3",
        b"\xff",
    ];
    let ab: &[u8] = b"ab";
    for ill in ill_formed {
        let before = [ill, ab].concat();
        let after = [ab, ill].concat();
        let further = [ab, b"cdef".as_slice(), ill, b"gh".as_slice()].concat();
        for text in [&before, &after, &further] {
            assert_eq!(answers("ab", text), (false, false), "{text:x?}");
            assert_eq!(answers("ab.*", text), (false, false), "{text:x?}");
            assert_eq!(answers(".*", text), (false, false), "{text:x?}");
        }
        assert_eq!(answers("", ill), (false, false), "{ill:x?}");
    }
    // The same text with a well-formed character where the ill-formed bytes stood is accepted.
    let mended = [ab, "\u{fffd}".as_bytes()].concat();
    assert_eq!(answers("ab", &mended), (false, true));
    assert_eq!(answers("ab.*", &mended), (true, true));
}
