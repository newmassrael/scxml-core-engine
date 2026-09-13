// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import "strings"

// MatchesEventDescriptor checks if an event name matches a descriptor
// (§scxml-3.12.1).
//
// Mirrors Rust event_matching::matches_event_descriptor and C++
// EventMatchingHelper::matchesEventDescriptor: each reduces a descriptor to
// one token prefix the same way. They called themselves 1:1 ports of one
// another while they disagreed, so the reduction is written out rather than
// claimed.
//
// Event matching rules (§scxml-3.12.1):
//  1. Event descriptor may contain multiple tokens separated by spaces
//  2. Each token is matched against the event name using prefix matching
//  3. Prefix matching uses dot (.) as token separator
//  4. "error", "error." and "error.*" are "functionally equivalent": each
//     reduces to the token prefix "error", so each matches bare "error" as
//     well as "error.send"
//  5. "*" matches any event, and so does a bare ".*", whose token prefix is
//     empty
//  6. Token boundaries are enforced: "foo" matches "foo.bar" but NOT "foobar"
//
// Rules 4 and 5 were wrong here until 2026-09-13: "foo.*" was read as the
// string prefix "foo.", so bare "foo" did not match it, "foo." matched
// nothing, and ".*" matched nothing. No W3C fixture delivers any of those
// cases, which is why a green conformance suite never said so; the fixture at
// integration_resources/event_descriptor_spellings_agree/ now does, on all
// seven channels.
func MatchesEventDescriptor(eventName, descriptor string) bool {
	// §scxml-3.12.1: Split descriptor into space-separated tokens
	tokens := strings.Fields(descriptor)

	// Empty descriptor: no match
	if len(tokens) == 0 {
		return false
	}

	// §scxml-3.12.1: Event matches if it matches ANY token
	for _, token := range tokens {
		// §scxml-3.12.1: Universal wildcard "*" matches any event
		if token == "*" {
			return true
		}

		// §scxml-3.12.1: a transition with "event" of "error", one with
		// "error." and one with "error.*" are "functionally equivalent since
		// they are token prefixes of exactly the same set of event names", so
		// every spelling reduces to one token prefix before anything is
		// compared. Reducing rather than branching keeps the three equivalent
		// by construction.
		prefix := strings.TrimSuffix(strings.TrimSuffix(token, ".*"), ".")

		// §scxml-3.12.1: a descriptor ending in ".*" matches "zero or more
		// tokens", so a bare ".*" is an empty token prefix — a prefix of every
		// event name, and thus a wildcard like "*".
		if prefix == "" {
			return true
		}

		// §scxml-3.12.1: the descriptor's tokens must be "an exact match or a
		// prefix of the set of tokens in the event's name". The boundary is a
		// whole token, so "foo" matches "foo.bar" and never "foobar".
		if eventName == prefix ||
			(len(eventName) > len(prefix) &&
				strings.HasPrefix(eventName, prefix) &&
				eventName[len(prefix)] == '.') {
			return true
		}
	}

	return false
}

// IsErrorEvent reports whether eventName names an error the processor itself
// raised, as opposed to an event the document asked for (§scxml-3.12.2).
//
// The clause reserves the whole "error." prefix for them: it defines
// error.execution and error.communication, lets a platform add a suffix to
// either, and reserves error.platform with or without a suffix on top of that.
// The prefix is therefore the test — an enumeration would be wrong the first
// time the set is extended, which the same paragraph says may happen.
//
// Used by the engine's internal-queue drain to tell an error nobody answered
// from an author's own unmatched <raise>. The two are indistinguishable in the
// queue and are not the same event to a host: the author wrote one and can read
// its fate in the document, while the other was written by the engine to report
// that the document did not do what it said.
func IsErrorEvent(eventName string) bool {
	return strings.HasPrefix(eventName, "error.")
}
