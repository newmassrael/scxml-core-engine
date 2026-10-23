// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// Whether an event name matches a descriptor (§scxml-3.12.1).
///
/// Mirrors Rust `event_matching::matches_event_descriptor` and C++
/// `EventMatchingHelper::matchesEventDescriptor`: each reduces a descriptor to
/// one token prefix the same way. They called themselves 1:1 ports of one
/// another while they disagreed, so the reduction is written out and not
/// claimed.
///
/// Event matching rules (§scxml-3.12.1):
///  1. An event descriptor may hold several tokens separated by spaces.
///  2. Each token is matched against the event name by prefix.
///  3. A prefix ends at a dot (.), the token separator.
///  4. "error", "error." and "error.*" are "functionally equivalent": each
///     reduces to the token prefix "error", so each matches bare "error" as
///     well as "error.send".
///  5. "*" matches any event, and so does a bare ".*", whose token prefix is
///     empty.
///  6. A token boundary is kept: "foo" matches "foo.bar" and never "foobar".
///
/// Names are compared as UTF-8 bytes, as every other backend compares them. A
/// Swift `String` compares by canonical equivalence and counts grapheme
/// clusters, which would let two spellings of one name differ from the bytes
/// the wire carries.
public func matchesEventDescriptor(eventName: String, descriptor: String) -> Bool {
    // §scxml-3.12.1: Split the descriptor into space-separated tokens. Splitting
    // is by Unicode scalar, as a rune is in the Go and Rust runtimes.
    let tokens = descriptor.unicodeScalars.split(whereSeparator: { $0.properties.isWhitespace })

    // An empty descriptor matches nothing.
    if tokens.isEmpty {
        return false
    }

    let name = Array(eventName.utf8)

    // §scxml-3.12.1: An event matches if it matches any token.
    for scalars in tokens {
        let token = Array(String(String.UnicodeScalarView(scalars)).utf8)

        // §scxml-3.12.1: The universal wildcard "*" matches any event.
        if token == Array("*".utf8) {
            return true
        }

        // §scxml-3.12.1: A transition with "event" of "error", one with
        // "error." and one with "error.*" are "functionally equivalent since
        // they are token prefixes of exactly the same set of event names", so
        // every spelling reduces to one token prefix before anything is
        // compared. Reducing and not branching keeps the three equivalent by
        // construction.
        let prefix = trimmingSuffix(trimmingSuffix(token, Array(".*".utf8)), Array(".".utf8))

        // §scxml-3.12.1: A descriptor ending in ".*" matches "zero or more
        // tokens", so a bare ".*" is an empty token prefix: a prefix of every
        // event name, and thus a wildcard like "*".
        if prefix.isEmpty {
            return true
        }

        // §scxml-3.12.1: The descriptor's tokens must be "an exact match or a
        // prefix of the set of tokens in the event's name". The boundary is a
        // whole token, so "foo" matches "foo.bar" and never "foobar".
        if name == prefix
            || (name.count > prefix.count
                && name.starts(with: prefix)
                && name[prefix.count] == UInt8(ascii: "."))
        {
            return true
        }
    }

    return false
}

/// Whether an event name names an error the processor itself raised, as opposed
/// to an event the document asked for (§scxml-3.12.2).
///
/// The clause reserves the whole "error." prefix for them: it defines
/// error.execution and error.communication, lets a platform add a suffix to
/// either, and reserves error.platform with or without a suffix on top of that.
/// The prefix is therefore the test; an enumeration would be wrong the first time
/// the set is extended, which the same paragraph says may happen.
///
/// Used by the engine's internal-queue drain to tell an error nobody answered
/// from an author's own unmatched `<raise>`. The two are indistinguishable in the
/// queue and are not the same event to a host: the author wrote one and can read
/// its fate in the document, while the other was written by the engine to report
/// that the document did not do what it said.
public func isErrorEvent(_ eventName: String) -> Bool {
    eventName.utf8.starts(with: "error.".utf8)
}

/// `bytes` without `suffix`, or all of `bytes` if it does not end with it.
private func trimmingSuffix(_ bytes: [UInt8], _ suffix: [UInt8]) -> [UInt8] {
    if bytes.count >= suffix.count && bytes.suffix(suffix.count).elementsEqual(suffix) {
        return Array(bytes.dropLast(suffix.count))
    }
    return bytes
}
