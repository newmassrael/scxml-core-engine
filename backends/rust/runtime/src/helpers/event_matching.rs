// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-5.9.3: Event descriptor matching algorithm.
//!
//! 1:1 port of `sce/include/core/EventMatchingHelper.h`. Provides the single
//! source of truth for event name matching against descriptors.
//!
//! Event matching rules (§scxml-5.9.3):
//! 1. Event descriptor may contain multiple tokens separated by spaces
//! 2. Each token is matched against the event name using prefix matching
//! 3. Prefix matching uses dot (.) as token separator
//! 4. Special wildcards:
//!    - `*` matches any event
//!    - `foo.*` matches any event starting with `foo.`
//! 5. Token boundaries are enforced: `foo` matches `foo.bar` but NOT `foobar`

/// §scxml-5.9.3: Check if an event name matches a descriptor.
///
/// Ports C++ `EventMatchingHelper::matchesEventDescriptor`.
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
/// ```
pub fn matches_event_descriptor(event_name: &str, descriptor: &str) -> bool {
    // §scxml-5.9.3: Iterate space-separated tokens directly. An empty or
    // whitespace-only descriptor produces zero tokens — the loop body never
    // executes and the fn falls through to `false`, matching the W3C "no
    // match" semantics. The previous `Vec<&str>` accumulation was redundant
    // (each token is independently testable in iteration order) and
    // alloc-coupled; iterating the `split_whitespace` adapter directly is
    // both no_std-portable and zero-allocation. SCE Protocol-Synthesis RFC §synth-5-J-2.
    for token in descriptor.split_whitespace() {
        // §scxml-5.9.3: Universal wildcard "*" matches any event
        if token == "*" {
            return true;
        }

        // §scxml-5.9.3: Wildcard suffix "foo.*" matches "foo.xxx"
        if token.len() >= 2 && token.ends_with(".*") {
            let prefix = &token[..token.len() - 1]; // "foo."
            if event_name.starts_with(prefix) {
                return true;
            }
        }

        // §scxml-5.9.3: Exact match
        if event_name == token {
            return true;
        }

        // §scxml-5.9.3: Prefix match with dot separator
        // "foo" matches "foo.bar" but NOT "foobar"
        if event_name.len() > token.len()
            && event_name.as_bytes()[token.len()] == b'.'
            && event_name.starts_with(token)
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
