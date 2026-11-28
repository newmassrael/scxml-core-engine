// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! No text makes a test take more than the automaton's states per code point (docs/adr/0149, D7,
//! first item, for the simulation). The families that drive a backtracking matcher to time
//! exponential in the text are run on texts of 1 to 2^20 bytes, and the steps the simulation
//! counts stay within the states of the automaton for each code point read and once for the
//! states it starts in. A count is exact where a clock is noise.

use sce_rust_regexp::{Space, Steps};

/// The families, each with the shortest text of `a`s in its language, if any is.
const FAMILIES: [(&str, Option<usize>); 4] = [
    ("(a|a)*b", None),
    ("(a*)*b", None),
    ("(a|b)*a(a|b){10}", Some(11)),
    (".*.*.*x", None),
];

#[test]
fn no_input_makes_matching_super_linear() {
    let text = vec![b'a'; 1 << 20];
    let mut space = Box::new(Space::<1024, 32>::new());
    for (family, shortest) in FAMILIES {
        let mut automaton = space
            .compile(family)
            .unwrap_or_else(|error| panic!("{family:?}: {error:?}"));
        let states = automaton.states() as u64;
        for power in 0..=20 {
            let length = 1usize << power;
            let input = &text[..length];
            let most = states * (length as u64 + 1);
            let holds = shortest.is_some_and(|shortest| length >= shortest);

            let mut steps = Steps::default();
            let whole = automaton.matches_metered(input, &mut steps);
            assert!(
                steps.0 <= most,
                "matches of {family:?} on {length} bytes took {} steps, past {most}",
                steps.0
            );
            assert_eq!(whole, holds, "matches of {family:?} on {length} bytes");

            let mut steps = Steps::default();
            let part = automaton.search_metered(input, &mut steps);
            assert!(
                steps.0 <= most,
                "search of {family:?} on {length} bytes took {} steps, past {most}",
                steps.0
            );
            assert_eq!(part, holds, "search of {family:?} on {length} bytes");
        }
    }
}
