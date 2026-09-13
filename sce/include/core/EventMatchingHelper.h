// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once
#include <algorithm>
#include <string>
#include <vector>

namespace SCE::Core::EventMatchingHelper {

/**
 * @brief §scxml-3.12.1: Event descriptor matching algorithm
 *
 * This is the Single Source of Truth for event matching logic shared between
 * Interpreter and AOT engines.
 *
 * Event Matching Rules (§scxml-3.12.1):
 * 1. Event descriptor may contain multiple tokens separated by spaces
 * 2. Each token is matched against the event name using prefix matching
 * 3. Prefix matching uses dot (.) as token separator
 * 4. "error", "error." and "error.*" are "functionally equivalent": each
 *    reduces to the token prefix "error", so each matches bare "error" as
 *    well as "error.send"
 * 5. "*" matches any event, and so does a bare ".*", whose token prefix is
 *    empty
 * 6. Token boundaries are enforced: "foo" matches "foo.bar" but NOT "foobar"
 *
 * Rules 4 and 5 were wrong here until 2026-09-13: "foo.*" was read as the
 * string prefix "foo.", so bare "foo" did not match it, "foo." matched
 * nothing, and ".*" matched nothing. No W3C fixture delivers any of those
 * cases, which is why a green conformance suite never said so; the fixture at
 * integration_resources/event_descriptor_spellings_agree/ now does, on all
 * seven channels.
 *
 * @param eventName The actual event name (e.g., "foo.zoo", "bar")
 * @param descriptor The event descriptor from transition (e.g., "foo bar", "foo.*", "*")
 * @return true if eventName matches descriptor, false otherwise
 *
 * @example
 * matchesEventDescriptor("foo", "foo bar")      → true  (exact match)
 * matchesEventDescriptor("bar", "foo bar")      → true  (second token)
 * matchesEventDescriptor("foo.zoo", "foo bar")  → true  (prefix match)
 * matchesEventDescriptor("foos", "foo")         → false (token boundary)
 * matchesEventDescriptor("foo.zoo", "foo.*")    → true  (wildcard suffix)
 * matchesEventDescriptor("anything", "*")       → true  (universal wildcard)
 * matchesEventDescriptor("foo", "foo.*")        → true  (the three spellings
 * matchesEventDescriptor("foo", "foo.")         → true   are one descriptor)
 * matchesEventDescriptor("a.b.c", ".*")         → true  (empty token prefix)
 * matchesEventDescriptor("foobar", "foo.*")     → false (whole tokens only)
 */
inline bool matchesEventDescriptor(const std::string &eventName, const std::string &descriptor) {
    // §scxml-3.12.1: Split descriptor into space-separated tokens
    std::vector<std::string> tokens;
    size_t start = 0;
    size_t end = descriptor.find(' ');

    while (end != std::string::npos) {
        if (end > start) {
            tokens.push_back(descriptor.substr(start, end - start));
        }
        start = end + 1;
        end = descriptor.find(' ', start);
    }
    if (start < descriptor.length()) {
        tokens.push_back(descriptor.substr(start));
    }

    // If no tokens (empty descriptor), no match
    if (tokens.empty()) {
        return false;
    }

    // §scxml-3.12.1: Event matches if it matches ANY token
    for (const auto &token : tokens) {
        // §scxml-3.12.1: Universal wildcard "*" matches any event
        if (token == "*") {
            return true;
        }

        // §scxml-3.12.1: a transition with 'event' of "error", one with
        // "error." and one with "error.*" are "functionally equivalent since
        // they are token prefixes of exactly the same set of event names", so
        // every spelling reduces to one token prefix before anything is
        // compared. Reducing rather than branching keeps the three equivalent
        // by construction.
        std::string prefix = token;
        if (prefix.length() >= 2 && prefix.compare(prefix.length() - 2, 2, ".*") == 0) {
            prefix.erase(prefix.length() - 2);
        }
        if (!prefix.empty() && prefix.back() == '.') {
            prefix.pop_back();
        }

        // §scxml-3.12.1: a descriptor ending in ".*" matches "zero or more
        // tokens", so a bare ".*" is an empty token prefix — a prefix of every
        // event name, and thus a wildcard like "*".
        if (prefix.empty()) {
            return true;
        }

        // §scxml-3.12.1: the descriptor's tokens must be "an exact match or a
        // prefix of the set of tokens in the event's name". The boundary is a
        // whole token, so "foo" matches "foo.bar" and never "foobar".
        if (eventName == prefix ||
            (eventName.length() > prefix.length() && eventName.compare(0, prefix.length(), prefix) == 0 &&
             eventName[prefix.length()] == '.')) {
            return true;
        }
    }

    return false;
}

/**
 * @brief §scxml-3.12.2: whether an event name is one the processor itself raised
 *
 * The clause reserves the whole `error.` prefix for them: it defines
 * `error.execution` and `error.communication`, lets a platform add a suffix to
 * either, and reserves `error.platform` with or without a suffix on top of that.
 * The prefix is therefore the test — an enumeration would be wrong the first
 * time the set is extended, which the same paragraph says may happen.
 *
 * Shared between Interpreter and AOT, like `matchesEventDescriptor` above.
 * Used by the internal-queue drain to tell an error nobody answered from an
 * author's own unmatched `<raise>`. The two are indistinguishable in the queue
 * and are not the same event to a host: the author wrote one and can read its
 * fate in the document, while the other was written by the engine to report
 * that the document did not do what it said.
 *
 * @param eventName The actual event name (e.g., "error.execution", "foo")
 * @return true when the processor is the sender of this event
 */
inline bool isErrorEvent(const std::string &eventName) {
    // §scxml-3.12.2: the processor "MUST signal any errors that occur by
    // raising SCXML events whose names begin with 'error.'". Cited in the body
    // rather than the doc block because the ledger's C++ resolver binds a
    // citation to the symbol enclosing it.
    return eventName.rfind("error.", 0) == 0;
}

}  // namespace SCE::Core::EventMatchingHelper
