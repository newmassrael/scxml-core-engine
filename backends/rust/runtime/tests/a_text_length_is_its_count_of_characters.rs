// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The TTCN-3 length of a text (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201
// 873-1 B.1.4.1 counts a universal charstring in characters, so a character
// of two, three or four UTF-8 bytes is one, as a character of one byte is.
//
// The Rust runtime has no function for it: a Rust text is UTF-8 by
// construction, and its count of characters is `str::chars().count()`, which a
// generated template's length restriction on a `string[N]` compares. These
// cases are the twins of tests/test_runtime/TextLengthTest.cpp, and pin that
// count apart from the byte length `str::len()`, which bounds a text in SCE.
// The C++ file's one compile-time case has no twin: the count is not a
// constant expression in Rust.

/// The length a template's length restriction holds a text to.
fn ttcn_length(text: &str) -> usize {
    text.chars().count()
}

mod text_length {
    use super::ttcn_length;

    #[test]
    fn an_empty_text_holds_no_character() {
        assert_eq!(ttcn_length(""), 0);
    }

    #[test]
    fn an_ascii_text_holds_one_character_per_byte() {
        assert_eq!(ttcn_length("started"), 7);
    }

    // U+00E9, two bytes: C3 A9.
    #[test]
    fn a_character_of_two_bytes_is_one() {
        let text = "caf\u{e9}";
        assert_eq!(text.len(), 5);
        assert_eq!(ttcn_length(text), 4);
    }

    // U+20AC, three bytes: E2 82 AC.
    #[test]
    fn a_character_of_three_bytes_is_one() {
        let text = "\u{20ac}5";
        assert_eq!(text.len(), 4);
        assert_eq!(ttcn_length(text), 2);
    }

    // U+1F600, four bytes: F0 9F 98 80.
    #[test]
    fn a_character_of_four_bytes_is_one() {
        let text = "a\u{1f600}b";
        assert_eq!(text.len(), 6);
        assert_eq!(ttcn_length(text), 3);
    }

    // One character of each width in a row: ten bytes, four characters.
    #[test]
    fn characters_of_every_width_are_counted_together() {
        let text = "a\u{e9}\u{20ac}\u{1f600}";
        assert_eq!(text.len(), 10);
        assert_eq!(ttcn_length(text), 4);
    }
}
