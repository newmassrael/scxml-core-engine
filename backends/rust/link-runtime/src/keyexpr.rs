// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which declared queryables a query key expression reaches (SCE
//! Protocol-Synthesis RFC §synth-5-R, Queryables a machine serves).
//!
//! `machines.<m>.queryables` declares the keys a machine answers. The
//! generated `answering()` asks this module one question per declared key:
//! does the query's key expression intersect it. The declared side is
//! constrained at deploy time so that the question has a bounded answer on a
//! device that reads queries from the network:
//!
//! - a declared key is made of literal chunks and whole-chunk `*` and `**`,
//!   never `$*` (`deploy/queryable-key-invalid`), and has at most
//!   [`MAX_DECLARED_CHUNKS`] chunks;
//! - the query side is whatever arrived, so it may carry `*`, `**` and `$*`.
//!
//! The cost is the number of query chunks times the number of declared chunks,
//! with no allocation and no recursion: the declared key's positions are the
//! bits of one `u64`, and each query chunk moves them. A query of any length
//! therefore costs time linear in its length, and a `$*` chunk costs at most
//! its length times the length of the declared literal it is compared with.
//!
//! Key expressions are the Zenoh ones: `/` separates chunks, `*` is exactly
//! one chunk, `**` is any number of chunks (including none), and the empty
//! string is the key with no chunks, the prefix itself.

/// The most chunks a declared key may have. One bit of the matcher's state per
/// declared position, plus the position past the last, must fit a `u64`.
pub const MAX_DECLARED_CHUNKS: usize = 32;

/// Whether `key` contains no wildcard, so that equality with a declared
/// literal key is the whole question.
pub fn is_literal(key: &str) -> bool {
    !key.bytes().any(|b| b == b'*' || b == b'$')
}

/// Whether the key expressions `query` and `declared` have a key in common.
///
/// `declared` must satisfy the deploy-time grammar described in the module
/// documentation. One that has more than [`MAX_DECLARED_CHUNKS`] chunks
/// intersects nothing, because the deploy refused it and a function that
/// answered for it would answer for a key no machine can declare.
pub fn intersects(query: &str, declared: &str) -> bool {
    let mut chunks: [&str; MAX_DECLARED_CHUNKS] = [""; MAX_DECLARED_CHUNKS];
    let mut n = 0;
    if !declared.is_empty() {
        for chunk in declared.split('/') {
            if n == MAX_DECLARED_CHUNKS {
                return false;
            }
            chunks[n] = chunk;
            n += 1;
        }
    }
    let declared = &chunks[..n];

    // Bit i: the query chunks read so far can have consumed the declared
    // chunks before position i. Bit n: all of them.
    let mut state = close(1, declared);
    if !query.is_empty() {
        for chunk in query.split('/') {
            if chunk == "**" {
                // Any number of chunks: every position at or after the first
                // reachable one is reachable.
                if state == 0 {
                    return false;
                }
                let first = state.trailing_zeros();
                state = close(!0u64 << first & mask(n), declared);
            } else {
                let mut next = 0u64;
                for (i, d) in declared.iter().enumerate() {
                    if state >> i & 1 == 0 {
                        continue;
                    }
                    if *d == "**" {
                        // `**` takes this chunk and stays.
                        next |= 1 << i;
                    } else if chunk_intersects(chunk, d) {
                        next |= 1 << (i + 1);
                    }
                }
                state = close(next, declared);
            }
            if state == 0 {
                return false;
            }
        }
    }
    state >> n & 1 == 1
}

/// The bits `0..=n`.
fn mask(n: usize) -> u64 {
    if n >= 63 {
        !0
    } else {
        (1u64 << (n + 1)) - 1
    }
}

/// Add the positions reached by a declared `**` that consumes no chunk.
fn close(mut state: u64, declared: &[&str]) -> u64 {
    for (i, d) in declared.iter().enumerate() {
        if *d == "**" && state >> i & 1 == 1 {
            state |= 1 << (i + 1);
        }
    }
    state
}

/// One query chunk against one declared chunk that is `*` or a literal.
fn chunk_intersects(query: &str, declared: &str) -> bool {
    if declared == "*" || query == "*" {
        return true;
    }
    if query.contains("$*") {
        return glob(query, declared);
    }
    query == declared
}

/// Whether `text` is in the language of `pattern`, where `$*` stands for any
/// run of bytes, including none.
fn glob(pattern: &str, text: &str) -> bool {
    let mut parts = pattern.split("$*").peekable();
    let first = parts.next().unwrap_or("");
    if parts.peek().is_none() {
        return pattern == text;
    }
    if !text.starts_with(first) {
        return false;
    }
    let mut rest = &text[first.len()..];
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_keys_intersect_only_when_equal() {
        assert!(intersects("config", "config"));
        assert!(intersects("status/connect", "status/connect"));
        assert!(!intersects("config", "status/connect"));
        assert!(!intersects("status", "status/connect"));
        assert!(!intersects("status/connect/x", "status/connect"));
    }

    #[test]
    fn the_empty_key_is_the_prefix_itself() {
        assert!(intersects("", ""));
        assert!(!intersects("config", ""));
        assert!(!intersects("", "config"));
        assert!(intersects("**", ""));
        assert!(!intersects("*", ""));
    }

    #[test]
    fn one_star_is_exactly_one_chunk() {
        assert!(intersects("peer/x", "peer/*"));
        assert!(!intersects("peer", "peer/*"));
        assert!(!intersects("peer/x/y", "peer/*"));
        assert!(intersects("*/connect", "status/connect"));
        assert!(intersects("*", "config"));
        assert!(!intersects("*", "status/connect"));
    }

    #[test]
    fn two_stars_are_any_number_of_chunks() {
        assert!(intersects("**", "config"));
        assert!(intersects("**", "status/connect"));
        assert!(intersects("peer/**", "peer"));
        assert!(intersects("peer/**", "peer/a/b"));
        assert!(!intersects("peer/**", "other"));
        assert!(intersects("**/connect", "status/connect"));
        assert!(!intersects("**/connect", "status/config"));
        assert!(intersects("a/**/z", "a/z"));
        assert!(intersects("a/**/z", "a/b/c/z"));
    }

    #[test]
    fn a_declared_double_star_takes_what_the_query_gives() {
        assert!(intersects("a", "**"));
        assert!(intersects("a/b/c", "**"));
        assert!(intersects("", "**"));
        assert!(intersects("a/b", "a/**"));
        assert!(intersects("a", "a/**"));
        assert!(!intersects("b/a", "a/**"));
        assert!(intersects("x/a/y", "**/a/**"));
        assert!(intersects("a", "**/a/**"));
        assert!(!intersects("x/b/y", "**/a/**"));
    }

    #[test]
    fn a_double_star_on_each_side_meets_in_the_middle() {
        assert!(intersects("**/a", "a/**"));
        assert!(intersects("**/a/**", "b/a/c"));
        assert!(intersects("**/x", "**/y/x"));
        assert!(!intersects("a/**", "b/**"));
        assert!(intersects("a/**", "**/b"));
    }

    #[test]
    fn a_sub_chunk_wildcard_in_the_query_is_a_glob() {
        assert!(intersects("con$*", "config"));
        assert!(intersects("$*fig", "config"));
        assert!(intersects("c$*g", "config"));
        assert!(intersects("$*", "config"));
        assert!(intersects("c$*n$*g", "config"));
        assert!(!intersects("x$*", "config"));
        assert!(!intersects("config$*x", "config"));
        assert!(intersects("config$*", "config"));
        assert!(intersects("status/c$*", "status/connect"));
        assert!(!intersects("status/c$*", "status/xonnect"));
    }

    #[test]
    fn a_query_chunk_star_meets_a_declared_star_and_a_declared_literal() {
        assert!(intersects("a/*/c", "a/*/c"));
        assert!(intersects("a/*/c", "a/b/c"));
        assert!(!intersects("a/*/c", "a/b/d"));
    }

    #[test]
    fn a_declared_key_past_the_chunk_limit_intersects_nothing() {
        let mut long = [b'a'; 2 * (MAX_DECLARED_CHUNKS + 1) - 1];
        for i in (1..long.len()).step_by(2) {
            long[i] = b'/';
        }
        let long = core::str::from_utf8(&long).unwrap();
        assert!(!intersects(long, long));
        let at_limit = &long[..2 * MAX_DECLARED_CHUNKS - 1];
        assert!(intersects(at_limit, at_limit));
    }

    #[test]
    fn literal_means_no_wildcard_byte() {
        assert!(is_literal("status/connect"));
        assert!(is_literal(""));
        assert!(!is_literal("peer/*"));
        assert!(!is_literal("peer/**"));
        assert!(!is_literal("co$*"));
    }
}
