// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A class is one state of the automaton whatever its ranges (docs/adr/0149, D3): `\p{L}`, whose
//! letters the pinned Unicode data spreads over hundreds of ranges, counts as `[a]` does, so the
//! bound a `regexp[N]` is held to reads the expression alone and is the same under every pin.

use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};
use icu_properties::CodePointMapData;
use sce_rust_regexp::{check, Error, Frame, Measure, Space};

fn measure(text: &str) -> Measure {
    let mut frames = [Frame::VACANT; 16];
    check(text, 1024, &mut frames).unwrap_or_else(|error| panic!("{text:?}: {error:?}"))
}

fn utf8(c: char) -> Vec<u8> {
    c.to_string().into_bytes()
}

#[test]
fn a_class_is_one_state_whatever_its_ranges() {
    let letters = CodePointMapData::<GeneralCategory>::new()
        .iter_ranges_for_group(GeneralCategoryGroup::Letter)
        .count();
    assert!(
        letters > 500,
        "the pinned data spreads the letters over {letters} ranges"
    );

    // One state reads the class and one accepts, whatever the class holds.
    for text in [
        "a",
        "[a]",
        ".",
        r"\p{L}",
        r"\P{L}",
        r"[^\p{L}]",
        "[^a]",
        r"[\p{L}\p{N}a-z_]",
        r"[\p{L}\p{M}\p{N}\p{P}\p{S}\p{Z}\p{C}]",
        r"[\P{L}\P{N}]",
    ] {
        assert_eq!(measure(text).states, 2, "{text:?}");
    }
    // Counts compose as the classes do not enter them.
    assert_eq!(measure(r"\p{L}{3}").states, measure("[a]{3}").states);
    assert_eq!(measure(r"\p{L}{3}").states, 4);
    assert_eq!(measure(r"(\p{L}|\p{N})*").states, measure("(a|b)*").states);
    assert_eq!(measure(r"(\p{L}|\p{N})*").states, 5);
    assert_eq!(
        measure(r"(\p{Lu}\p{Ll}+){2,3}").states,
        measure("(ab+){2,3}").states
    );

    // A class's storage is the ranges its text writes, never the data's: none for `\p{L}`.
    let named = measure(r"\p{L}");
    assert_eq!((named.classes, named.ranges), (1, 0));
    let written = measure(r"[a-zA-Z_\p{N}]");
    assert_eq!((written.classes, written.ranges), (1, 3));

    // So two states of storage build `\p{L}`, and the class reads the pinned data when a text
    // is tested: Latin, Cyrillic, Hebrew and Han letters in, a digit, a space, a combining mark
    // and a currency sign out.
    let mut space = Space::<2, 8>::new();
    let mut letter = space.compile(r"\p{L}").expect("two states hold a class");
    assert_eq!(letter.states(), 2);
    for c in ['A', 'z', '\u{416}', '\u{5d0}', '\u{4e00}'] {
        assert!(
            letter.matches(&utf8(c)),
            "U+{:04X} is a letter",
            u32::from(c)
        );
    }
    for c in ['1', ' ', '\u{301}', '\u{20ac}'] {
        assert!(
            !letter.matches(&utf8(c)),
            "U+{:04X} is no letter",
            u32::from(c)
        );
    }
    // And one state short is too large, by the same count.
    let mut short = Space::<1, 8>::new();
    assert_eq!(
        short.compile(r"\p{L}").unwrap_err(),
        Error::TooLarge {
            states: 2,
            bound: 1
        }
    );
}
