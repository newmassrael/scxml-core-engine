//! §scxml-3.12.1 — event descriptors, the one definition in this crate.
//!
//! > Like an event name, an event descriptor is a series of alphanumeric
//! > characters segmented into tokens by the "." character. [...] An event
//! > descriptor matches an event name if its string of tokens is an exact
//! > match or a prefix of the set of tokens in the event's name. In all
//! > cases, the token matching is case sensitive. [...] an event descriptor
//! > MAY also end with the wildcard '.*', which matches zero or more tokens
//! > at the end of the processed event's name. Note that a transition with
//! > 'event' of "error", one with "error.", and one with "error.*" are
//! > functionally equivalent since they are token prefixes of exactly the
//! > same set of event names. An event designator consisting solely of "*"
//! > can be used as a wildcard matching any sequence of tokens, and thus
//! > any event.
//!
//! # ⭐ Why this module exists
//!
//! Before it, three build-time readers decided "does this descriptor match
//! that event" and each wrote its own answer. Measured 2026-09-13 against the
//! sentence above, they disagreed with the specification and with each
//! other:
//!
//! ```text
//!                                              error.* vs "error"   error. vs "error"
//!   mesh/topology::event_matches_any            match                no
//!   scxml_exhaustiveness::transition_matches    no                   no
//!   analyzer::build_prefix_matching             no                   no
//! ```
//!
//! The last one decides which events a transition catches in the code C11
//! and Kotlin generate, so the disagreement was a behaviour difference
//! between backends, not a lint preference. Three more places answered the
//! neighbouring question — which event a descriptor names literally — and the
//! parser's copy recorded `error.` as an event called `error.`. Every one of
//! them now routes through this module, and
//! `sce-build/tests/an_event_descriptor_matches_what_the_spec_says_it_matches.rs`
//! holds the result to the specification's own examples.
//!
//! # Two things it deliberately does not do
//!
//! - **`_*` is not a wildcard.** Two copies treated it as one. It appears in
//!   no W3C text, no document in this repository and no template, so it is
//!   read as the literal token it is spelled as.
//! - **A bare `.*` is universal.** The specification does not spell that
//!   case out, but it follows from its rule — `.*` is an empty token prefix
//!   followed by "zero or more tokens", and the empty prefix is a prefix of
//!   every name — and W3C tests 311 to 314 rely on it as a catch-all.

/// One descriptor from a transition's `event` attribute, reduced to what it
/// matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDescriptor<'a> {
    /// Matches every event: `*`, and `.*`, whose token prefix is empty.
    Any,
    /// Matches the named event and every event it is a token prefix of.
    ///
    /// `error`, `error.` and `error.*` all reduce to `Prefix("error")`,
    /// which is how the specification's "functionally equivalent" is made
    /// true by construction rather than by three branches agreeing.
    Prefix(&'a str),
}

impl<'a> EventDescriptor<'a> {
    /// Reduce one written descriptor to what it matches.
    pub fn parse(token: &'a str) -> Self {
        // §scxml-3.12.1: `error`, `error.` and `error.*` are "functionally
        // equivalent", so the three spellings reduce to one token prefix.
        if token == "*" {
            return EventDescriptor::Any;
        }
        let without_wildcard = token.strip_suffix(".*").unwrap_or(token);
        let prefix = without_wildcard
            .strip_suffix('.')
            .unwrap_or(without_wildcard);
        if prefix.is_empty() {
            EventDescriptor::Any
        } else {
            EventDescriptor::Prefix(prefix)
        }
    }

    /// Whether this descriptor matches `event`, by whole dot-tokens and
    /// case-sensitively — `error` matches `error.send` and not `errors`.
    pub fn matches(&self, event: &str) -> bool {
        // §scxml-3.12.1: a match is "an exact match or a prefix of the set
        // of tokens in the event's name".
        match self {
            EventDescriptor::Any => true,
            EventDescriptor::Prefix(prefix) => {
                event == *prefix
                    || (event.len() > prefix.len()
                        && event.starts_with(prefix)
                        && event.as_bytes()[prefix.len()] == b'.')
            }
        }
    }

    /// The token prefix, or `None` for a descriptor that matches everything.
    pub fn prefix(&self) -> Option<&'a str> {
        match self {
            EventDescriptor::Any => None,
            EventDescriptor::Prefix(prefix) => Some(prefix),
        }
    }
}

/// Every descriptor in a transition's `event` attribute, in written order.
pub fn descriptors(attribute: &str) -> impl Iterator<Item = EventDescriptor<'_>> {
    attribute.split_whitespace().map(EventDescriptor::parse)
}

/// Whether a transition whose `event` attribute is `attribute` matches
/// `event` — "if at least one of its event descriptors matches the event's
/// name".
pub fn attribute_matches(attribute: &str, event: &str) -> bool {
    descriptors(attribute).any(|descriptor| descriptor.matches(event))
}

/// The event a written descriptor names literally, or `None` when it is a
/// pattern.
///
/// A literal is a descriptor written exactly as its own token prefix: `foo`
/// or `foo.bar`. `foo.*`, `foo.` and `*` are patterns — they match events,
/// they do not declare one — so a reader building a document's event
/// vocabulary takes literals only. Before this, the parser tested for the
/// `.*` spelling alone and recorded `foo.` as an event called `foo.`.
pub fn literal_event(token: &str) -> Option<&str> {
    match EventDescriptor::parse(token) {
        EventDescriptor::Prefix(prefix) if prefix == token => Some(token),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_spellings_the_specification_calls_equivalent_reduce_alike() {
        let reduced: Vec<EventDescriptor<'_>> = ["error", "error.", "error.*"]
            .into_iter()
            .map(EventDescriptor::parse)
            .collect();
        assert!(
            reduced
                .iter()
                .all(|d| *d == EventDescriptor::Prefix("error")),
            "{reduced:?}"
        );
    }

    #[test]
    fn a_pattern_names_no_event_and_a_literal_names_itself() {
        assert_eq!(literal_event("foo.bar"), Some("foo.bar"));
        for pattern in ["foo.*", "foo.", "*", ".*"] {
            assert_eq!(literal_event(pattern), None, "{pattern}");
        }
        // Not a wildcard here: no text defines it as one.
        assert_eq!(literal_event("_*"), Some("_*"));
    }
}
