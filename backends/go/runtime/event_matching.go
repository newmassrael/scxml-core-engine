// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import "strings"

// MatchesEventDescriptor checks if an event name matches a descriptor
// (§scxml-5.9.3).
//
// 1:1 port of Rust event_matching::matches_event_descriptor from
// backends/rust/runtime/src/helpers/event_matching.rs.
//
// Event matching rules (§scxml-5.9.3):
//  1. Event descriptor may contain multiple tokens separated by spaces
//  2. Each token is matched against the event name using prefix matching
//  3. Prefix matching uses dot (.) as token separator
//  4. Special wildcards:
//     - "*" matches any event
//     - "foo.*" matches any event starting with "foo."
//  5. Token boundaries are enforced: "foo" matches "foo.bar" but NOT "foobar"
func MatchesEventDescriptor(eventName, descriptor string) bool {
	// §scxml-5.9.3: Split descriptor into space-separated tokens
	tokens := strings.Fields(descriptor)

	// Empty descriptor: no match
	if len(tokens) == 0 {
		return false
	}

	// §scxml-5.9.3: Event matches if it matches ANY token
	for _, token := range tokens {
		// §scxml-5.9.3: Universal wildcard "*" matches any event
		if token == "*" {
			return true
		}

		// §scxml-5.9.3: Wildcard suffix "foo.*" matches "foo.xxx"
		if len(token) >= 2 && strings.HasSuffix(token, ".*") {
			prefix := token[:len(token)-1] // "foo."
			if strings.HasPrefix(eventName, prefix) {
				return true
			}
		}

		// §scxml-5.9.3: Exact match
		if eventName == token {
			return true
		}

		// §scxml-5.9.3: Prefix match with dot separator
		// "foo" matches "foo.bar" but NOT "foobar"
		if len(eventName) > len(token) &&
			eventName[len(token)] == '.' &&
			strings.HasPrefix(eventName, token) {
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
