// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// Event descriptor matching (§scxml-3.12.1): each spelling of a descriptor reduces
// to one token prefix, and a prefix ends on a token boundary.

@Test(arguments: [
    ("foo", "foo"),
    ("foo.bar", "foo"),
    ("foo.bar.baz", "foo.bar"),
    ("error", "error"),
    ("error.send", "error"),
    ("error", "error."),
    ("error.send", "error."),
    ("error", "error.*"),
    ("error.send", "error.*"),
    ("anything", "*"),
    ("anything.at.all", ".*"),
    ("b", "a b"),
    ("b", "a  \t b"),
])
func aDescriptorMatchesTheNameItIsAPrefixOf(eventName: String, descriptor: String) {
    #expect(matchesEventDescriptor(eventName: eventName, descriptor: descriptor))
}

@Test(arguments: [
    ("foobar", "foo"),
    ("foo", "foo.bar"),
    ("bar", "foo"),
    ("errors", "error"),
    ("anything", ""),
    ("anything", "   "),
])
func aDescriptorDoesNotMatchAcrossATokenBoundary(eventName: String, descriptor: String) {
    #expect(!matchesEventDescriptor(eventName: eventName, descriptor: descriptor))
}

@Test func theNameIsComparedByItsBytesAndNotByCanonicalEquivalence() {
    // "e" followed by U+0301 and the precomposed U+00E9 are one string to
    // Swift's `==` and two names on the wire.
    #expect(!matchesEventDescriptor(eventName: "e\u{301}", descriptor: "\u{E9}"))
    #expect(matchesEventDescriptor(eventName: "\u{E9}.x", descriptor: "\u{E9}"))
}

@Test func anErrorEventIsOneWhoseNameStartsWithErrorAndADot() {
    #expect(isErrorEvent("error.execution"))
    #expect(isErrorEvent("error.platform.anything"))
    #expect(!isErrorEvent("error"))
    #expect(!isErrorEvent("errors.x"))
    #expect(!isErrorEvent("my.error.x"))
}
