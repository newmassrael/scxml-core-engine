// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceRuntime

// A <send> delay is read as one CSS2 time on every engine (ARCHITECTURE.md,
// "Durations (Single Source of Truth)"). The cases live in
// tests/durations/css2_time.json, read here where they are, so this reader is
// measured against the same table as every other engine's.
@Test func aDelayIsReadAsTheOneCSS2TimeEveryEngineReads() throws {
    let cases = try sharedCases("durations/css2_time.json")
    #expect(cases.count >= 20, "the table lost cases: \(cases.count)")
    for c in cases {
        let name = try #require(c["name"] as? String)
        let text = try #require(c["text"] as? String)
        let got = parseDelayToMs(text)
        if c["ms"] is NSNull || c["ms"] == nil {
            #expect(got == nil, "\(name): read \(text.debugDescription) as \(got ?? 0) ms, the table says it is not a time")
        } else {
            let ms = try #require((c["ms"] as? NSNumber)?.uint64Value, "\(name)")
            #expect(got == ms, "\(name): read \(text.debugDescription) as \(got.map(String.init) ?? "nothing"), the table says \(ms)")
        }
    }
}

// MeshPeer holds to tests/mesh/mesh_target_cases.json, the table the C++ core's
// SendHelper::isMeshTarget, the build and the other runtimes read too.
@Test func aMeshPeerIsNamedAsTheSharedTableNamesIt() throws {
    let cases = try sharedCases("mesh/mesh_target_cases.json")
    // A floor: an empty table would pass every assertion below.
    #expect(cases.count >= 10, "the table lost cases: \(cases.count)")
    for c in cases {
        let target = try #require(c["target"] as? String)
        let peer = c["peer"] as? String
        #expect(meshPeer(target) == peer, "meshPeer(\(target.debugDescription))")
        #expect(isMeshTarget(target) == (peer != nil), "isMeshTarget(\(target.debugDescription))")
    }
}

@Test(arguments: [
    ("", SendTarget(kind: .unreachable)),
    ("undefined", SendTarget(kind: .unreachable)),
    ("!bad", SendTarget(kind: .unsupported)),
    ("#_internal", SendTarget(kind: .internalQueue)),
    ("#_parent", SendTarget(kind: .parent)),
    ("#_scxml_other", SendTarget(kind: .session, id: "other")),
    ("#_scxml_me", SendTarget(kind: .selfExternal)),
    ("#_scxml_", SendTarget(kind: .selfExternal)),
    ("sce://scxml/other", SendTarget(kind: .session, id: "other")),
    ("sce://scxml/me", SendTarget(kind: .selfExternal)),
    ("#_child1", SendTarget(kind: .invocation, id: "child1")),
    ("#peer", SendTarget(kind: .mesh)),
    ("bogus", SendTarget(kind: .unsupported)),
    ("http://host/x", SendTarget(kind: .unsupported)),
])
func aTargetIsRoutedWhereTheTableSaysItGoes(target: String, want: SendTarget) {
    #expect(classifyTarget(target, ownSessionID: "me") == want, "\(target.debugDescription)")
}

@Test func theInvokeTargetsAreTheOnesThatAreNotReserved() {
    #expect(isChildInvokeTarget("#_worker"))
    #expect(!isChildInvokeTarget("#_parent"))
    #expect(!isChildInvokeTarget("#_internal"))
    #expect(!isChildInvokeTarget("#_scxml_abc"))
    #expect(!isChildInvokeTarget("worker"))
    #expect(extractInvokeID("#_worker") == "worker")
    #expect(extractInvokeID("worker") == "worker")
}

@Test func aBasicHttpSendNeedsItsTarget() throws {
    #expect(isSupportedSendType(""))
    #expect(isSupportedSendType(scxmlEventProcessorType))
    #expect(isSupportedSendType(basicHTTPEventProcessorType))
    #expect(!isSupportedSendType("http://example.com/other"))
    #expect(throws: SendArgumentError.self) {
        try validateBasicHTTPSend(sendType: basicHTTPEventProcessorType, target: "", targetExpr: "")
    }
    try validateBasicHTTPSend(sendType: basicHTTPEventProcessorType, target: "http://x", targetExpr: "")
    try validateBasicHTTPSend(sendType: basicHTTPEventProcessorType, target: "", targetExpr: "t")
    try validateBasicHTTPSend(sendType: "", target: "", targetExpr: "")
    #expect(isHTTPTarget("https://x"))
    #expect(!isHTTPTarget("ftp://x"))
    #expect(throws: SendArgumentError.self) { try validateTarget("!no") }
    try validateTarget("#_parent")
}
