// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! How far a misspelling is from what the author meant.
//!
//! # Why one module
//!
//! Three refusals in this crate name the nearest legal spelling — an unknown
//! `sce:` attribute, an unknown mesh binding key, an undeclared ECMAScript
//! identifier — and each carried its own copy of the same Levenshtein
//! distance. A fourth was about to be written for the forge expression
//! layer's undeclared identifiers and enum variants. The metric is one
//! function; the copies are gone.
//!
//! ⚠ What stays at each call site is the SELECTION rule — how close is close
//! enough, and how many to name. Those are per-surface decisions whose output
//! is pinned by golden records, and they are not the same rule: sharing the
//! metric does not mean sharing the policy. [`near_misses`] is the one policy
//! two surfaces do share, because they emit the same diagnostic code.

/// The Levenshtein distance between `a` and `b`, over `char`s.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

/// At most this many suggestions ride a diagnostic. A repair the consumer
/// has to choose from twenty candidates is not a repair.
pub const SUGGESTION_LIMIT: usize = 3;

/// The names in `pool` close enough to `name` to be what the author meant,
/// nearest first, ties broken by spelling, at most [`SUGGESTION_LIMIT`].
///
/// The budget scales with the name: one edit for a name of up to three
/// characters, two up to seven, three beyond. A flat budget either names an
/// unrelated short word or misses a long one's transposition.
///
/// This is the rule `expression/unknown-identifier` carries, on both the
/// ECMAScript datamodel and the forge expression layer — one code, one rule
/// for what it suggests.
pub fn near_misses<'p>(name: &str, pool: impl IntoIterator<Item = &'p str>) -> Vec<String> {
    let budget = match name.chars().count() {
        0..=3 => 1,
        4..=7 => 2,
        _ => 3,
    };
    let mut scored: Vec<(usize, &str)> = pool
        .into_iter()
        .filter_map(|candidate| {
            let distance = edit_distance(name, candidate);
            (distance <= budget).then_some((distance, candidate))
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    scored.dedup_by(|a, b| a.1 == b.1);
    scored
        .into_iter()
        .take(SUGGESTION_LIMIT)
        .map(|(_, c)| c.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_the_levenshtein_distance() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("conut", "count"), 2);
        // Over chars, not bytes: `é` is two bytes and one edit.
        assert_eq!(edit_distance("café", "cafe"), 1);
    }

    #[test]
    fn near_misses_scale_the_budget_with_the_name() {
        let pool = ["count", "counter", "RUN_BATCH", "OFF", "NONE"];
        assert_eq!(near_misses("conut", pool), vec!["count".to_string()]);
        // A short name gets one edit: `RUN` is six edits from `RUN_BATCH`
        // and must NOT be offered — offering it would guess which mode.
        assert!(near_misses("RUN", pool).is_empty());
        assert_eq!(near_misses("OF", pool), vec!["OFF".to_string()]);
    }
}
