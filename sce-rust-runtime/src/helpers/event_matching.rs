// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 5.9.3: Event descriptor matching algorithm.
//!
//! 1:1 port of `sce/include/core/EventMatchingHelper.h`. Provides the single
//! source of truth for event name matching against descriptors.
//!
//! Event matching rules (W3C SCXML 5.9.3):
//! 1. Event descriptor may contain multiple tokens separated by spaces
//! 2. Each token is matched against the event name using prefix matching
//! 3. Prefix matching uses dot (.) as token separator
//! 4. Special wildcards:
//!    - `*` matches any event
//!    - `foo.*` matches any event starting with `foo.`
//! 5. Token boundaries are enforced: `foo` matches `foo.bar` but NOT `foobar`

/// W3C SCXML 5.9.3: Check if an event name matches a descriptor.
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
    // W3C SCXML 5.9.3: Split descriptor into space-separated tokens
    let tokens: Vec<&str> = descriptor.split_whitespace().collect();

    // Empty descriptor: no match
    if tokens.is_empty() {
        return false;
    }

    // W3C SCXML 5.9.3: Event matches if it matches ANY token
    for token in &tokens {
        // W3C SCXML 5.9.3: Universal wildcard "*" matches any event
        if *token == "*" {
            return true;
        }

        // W3C SCXML 5.9.3: Wildcard suffix "foo.*" matches "foo.xxx"
        if token.len() >= 2 && token.ends_with(".*") {
            let prefix = &token[..token.len() - 1]; // "foo."
            if event_name.starts_with(prefix) {
                return true;
            }
        }

        // W3C SCXML 5.9.3: Exact match
        if event_name == *token {
            return true;
        }

        // W3C SCXML 5.9.3: Prefix match with dot separator
        // "foo" matches "foo.bar" but NOT "foobar"
        if event_name.len() > token.len()
            && event_name.as_bytes()[token.len()] == b'.'
            && event_name.starts_with(*token)
        {
            return true;
        }
    }

    false
}
