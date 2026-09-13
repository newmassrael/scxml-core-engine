// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.12.1: event descriptor matching.
//!
//! Mirrors `sce/include/core/EventMatchingHelper.h` and `sce-build`'s
//! `event_descriptor` module: each reduces a descriptor to one token prefix
//! the same way. They called themselves 1:1 ports of one another while they
//! disagreed, so the reduction is written out here rather than claimed.
//!
//! Event matching rules (§scxml-3.12.1):
//! 1. Event descriptor may contain multiple tokens separated by spaces
//! 2. Each token is matched against the event name using prefix matching
//! 3. Prefix matching uses dot (.) as token separator
//! 4. `error`, `error.` and `error.*` are "functionally equivalent": each
//!    reduces to the token prefix `error`, so each matches bare `error` as
//!    well as `error.send`
//! 5. `*` matches any event, and so does a bare `.*`, whose token prefix is
//!    empty
//! 6. Token boundaries are enforced: `foo` matches `foo.bar` but NOT `foobar`
//!
//! ⚠ Rules 4 and 5 were wrong here until 2026-09-13. `foo.*` was read as the
//! string prefix `foo.`, so bare `foo` did not match it, `foo.` matched
//! nothing, and `.*` matched nothing. No W3C fixture delivers any of those
//! cases, which is why a green conformance suite never said so; the fixture
//! at `integration_resources/event_descriptor_spellings_agree/` now does, on
//! all seven channels.

/// §scxml-3.12.1: Check if an event name matches a descriptor.
///
/// Mirrors C++ `EventMatchingHelper::matchesEventDescriptor` and `sce-build`'s
/// `event_descriptor` module: each reduces a descriptor to one token prefix
/// the same way, so build time and run time cannot disagree about what a
/// transition catches.
///
/// # Examples
///
/// ```
/// use sce_rust_runtime::helpers::event_matching::matches_event_descriptor;
///
/// assert!(matches_event_descriptor("foo", "foo bar"));        // exact match
/// assert!(matches_event_descriptor("bar", "foo bar"));        // second token
/// assert!(matches_event_descriptor("foo.zoo", "foo bar"));    // prefix match
/// assert!(!matches_event_descriptor("foos", "foo"));          // token boundary
/// assert!(matches_event_descriptor("foo.zoo", "foo.*"));      // wildcard suffix
/// assert!(matches_event_descriptor("anything", "*"));         // universal wildcard
///
/// // The three spellings the clause calls functionally equivalent:
/// assert!(matches_event_descriptor("foo", "foo.*"));
/// assert!(matches_event_descriptor("foo", "foo."));
/// assert!(matches_event_descriptor("foo.zoo", "foo."));
/// // An empty token prefix is a prefix of every event name:
/// assert!(matches_event_descriptor("anything.at.all", ".*"));
/// // ...but a prefix is still whole tokens, never raw characters:
/// assert!(!matches_event_descriptor("foobar", "foo.*"));
/// ```
pub fn matches_event_descriptor(event_name: &str, descriptor: &str) -> bool {
    // §scxml-3.12.1: Iterate space-separated tokens directly. An empty or
    // whitespace-only descriptor produces zero tokens — the loop body never
    // executes and the fn falls through to `false`, matching the W3C "no
    // match" semantics. The previous `Vec<&str>` accumulation was redundant
    // (each token is independently testable in iteration order) and
    // alloc-coupled; iterating the `split_whitespace` adapter directly is
    // both no_std-portable and zero-allocation. SCE Protocol-Synthesis RFC §synth-5-J-2.
    for token in descriptor.split_whitespace() {
        // §scxml-3.12.1: Universal wildcard "*" matches any event
        if token == "*" {
            return true;
        }

        // §scxml-3.12.1: a transition with `event` of "error", one with
        // "error." and one with "error.*" are "functionally equivalent since
        // they are token prefixes of exactly the same set of event names", so
        // every spelling reduces to one token prefix before anything is
        // compared. Reducing rather than branching is what keeps the three
        // equivalent by construction.
        let without_wildcard = token.strip_suffix(".*").unwrap_or(token);
        let prefix = without_wildcard
            .strip_suffix('.')
            .unwrap_or(without_wildcard);

        // §scxml-3.12.1: a descriptor ending in ".*" matches "zero or more
        // tokens", so a bare ".*" is an empty token prefix — a prefix of every
        // event name, and thus a wildcard like "*".
        if prefix.is_empty() {
            return true;
        }

        // §scxml-3.12.1: the descriptor's tokens must be "an exact match or a
        // prefix of the set of tokens in the event's name". The boundary is a
        // whole token, so "foo" matches "foo.bar" and never "foobar".
        if event_name == prefix
            || (event_name.len() > prefix.len()
                && event_name.starts_with(prefix)
                && event_name.as_bytes()[prefix.len()] == b'.')
        {
            return true;
        }
    }

    false
}

/// Whether `event_name` names an error the processor itself raised, as opposed
/// to an event the document asked for.
///
/// The clause reserves the whole `error.` prefix for them: it defines
/// `error.execution` and `error.communication`, lets a platform add a suffix
/// to either, and reserves `error.platform` with or without a suffix on top of
/// that. The prefix is therefore the test — an enumeration would be wrong the
/// first time the set is extended, which the same paragraph says may happen.
///
/// Used by the engine's internal-queue drain to tell an error nobody answered
/// from an author's own unmatched `<raise>`. The two are indistinguishable in
/// the queue and are not the same event to a host: the author wrote one and
/// can read its fate in the document, while the other was written by the
/// engine to report that the document did not do what it said.
pub fn is_error_event(event_name: &str) -> bool {
    // §scxml-3.12.2: the processor "MUST signal any errors that occur by
    // raising SCXML events whose names begin with 'error.'", and reserves the
    // `error.platform` family on top of the two it defines. Cited in the body
    // rather than the doc comment because the ledger's Rust resolver binds a
    // citation to the symbol enclosing it, and a `///` line encloses nothing.
    event_name.starts_with("error.")
}
