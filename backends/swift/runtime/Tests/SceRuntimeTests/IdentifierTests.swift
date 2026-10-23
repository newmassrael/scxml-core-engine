// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

@Test func eachGeneratorNamesWhatItGeneratesAndNeverRepeats() {
    let ids = [
        generateSessionID(),
        generateSessionID(prefix: "root"),
        generateSendID(),
        generateInvokeID(),
        generateEventID(),
        generateCorrelationID(),
        generateActionID(),
        generateActionID(prefix: "act"),
    ]
    let prefixes = ["session_", "root_", "send_", "invoke_", "event_", "corr_", "action_", "act_"]
    for (id, prefix) in zip(ids, prefixes) {
        #expect(id.hasPrefix(prefix), "\(id) does not start with \(prefix)")
    }
    #expect(Set(ids).count == ids.count)
}

@Test func anInvokeIdOfAStateIsTheStateAndAPlatformCounter() {
    let first = generateInvokeID(stateID: "s1")
    let second = generateInvokeID(stateID: "s1")
    #expect(first.hasPrefix("s1.platform_"))
    #expect(second.hasPrefix("s1.platform_"))
    #expect(first != second)
    #expect(UInt64(first.dropFirst("s1.platform_".count)) != nil)
}

@Test func theInvokeCounterOnlyCountsUp() {
    let first = nextInvokeCounter()
    let second = nextInvokeCounter()
    #expect(second > first)
}

@Test func aSessionIdAHostGivesIsHeldToAnNmtoken() {
    #expect(isNmtoken("a"))
    #expect(isNmtoken("a.b-c_d:e9"))
    #expect(isNmtoken("\u{E9}t\u{E9}"))
    #expect(!isNmtoken(""))
    #expect(!isNmtoken("a b"))
    #expect(!isNmtoken("a/b"))
    #expect(sessionIDRefusalOf("").contains("empty"))
    #expect(sessionIDRefusalOf("a b").contains("U+0020"))
    #expect(sessionIDRefusalOf("ok").isEmpty)
}

@Test func aDrawnSessionIdIsAVersionFourUuidInLowercaseHexadecimal() {
    let ids = (0..<50).map { _ in drawSessionID() }
    for id in ids {
        let groups = id.split(separator: "-", omittingEmptySubsequences: false)
        #expect(groups.map(\.count) == [8, 4, 4, 4, 12], "\(id)")
        #expect(id.allSatisfy { $0 == "-" || ("0"..."9").contains($0) || ("a"..."f").contains($0) }, "\(id)")
        #expect(groups[2].first == "4", "\(id) is not version 4")
        #expect("89ab".contains(groups[3].first!), "\(id) has no RFC 9562 variant")
        #expect(isNmtoken(id))
    }
    #expect(Set(ids).count == ids.count)
}
