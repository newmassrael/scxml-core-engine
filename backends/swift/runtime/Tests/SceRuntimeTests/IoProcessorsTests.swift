// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

private func locationOf(_ descriptors: [IoProcessorDescriptor], _ name: String) -> String {
    descriptors.first { $0.name == name }?.location ?? ""
}

@Test func theScxmlProcessorIsPublishedUnderBothSpellings() {
    let descriptors = buildIoProcessors(sessionID: "session-1", basicHTTPAccessURI: "")
    #expect(locationOf(descriptors, scxmlProcessorAlias) == locationOf(descriptors, scxmlEventProcessorType))
    #expect(locationOf(descriptors, scxmlProcessorAlias) == "sce://scxml/session-1")
    #expect(descriptors.count == 2, "no BasicHTTP entry without an endpoint")
}

@Test func theBasicHttpProcessorIsPublishedOnlyWhereItHasAnEndpoint() {
    let descriptors = buildIoProcessors(sessionID: "s", basicHTTPAccessURI: "http://host:8080/in")
    #expect(locationOf(descriptors, basicHTTPEventProcessorType) == "http://host:8080/in")
    #expect(locationOf(descriptors, basicHTTPProcessorAlias) == "http://host:8080/in")
}

@Test(arguments: ["session-1", "a b/c#d", "parent.state.1.inv_peer", "\u{E9}t\u{E9}"])
func aPublishedLocationReadsBackAsTheSessionItNames(sessionID: String) {
    // The round trip is the clause: an origin that cannot be decoded back to a
    // session is not an address a peer can answer.
    #expect(sessionIDFromScxmlLocation(scxmlProcessorLocation(sessionID: sessionID)) == sessionID)
}

@Test(arguments: ["sce://scxml/", "session-1", "http://host/x", ""])
func aTextThatIsNoLocationNamesNoSession(text: String) {
    #expect(sessionIDFromScxmlLocation(text) == "")
}

@Test func aMalformedEscapeIsLeftVerbatimAndNeverRewrittenIntoAnotherAddress() {
    #expect(sessionIDFromScxmlLocation("sce://scxml/a%zzb") == "a%zzb")
    #expect(sessionIDFromScxmlLocation("sce://scxml/a%4") == "a%4")
    #expect(sessionIDFromScxmlLocation("sce://scxml/a%41b") == "aAb")
}

@Test func thePublishedOriginIsTheLocationTheSenderPublishes() {
    let descriptors = buildIoProcessors(sessionID: "session-1", basicHTTPAccessURI: "")
    #expect(publishedOrigin(originSessionID: "session-1") == locationOf(descriptors, scxmlProcessorAlias))
}

@Test func thePublishedOriginPassesAnAddressThroughAndKeepsEmptyEmpty() {
    // A remote child is stamped with a URI, not an id; wrapping it again would
    // produce an address naming nothing.
    #expect(publishedOrigin(originSessionID: "sce://mesh/peer-7") == "sce://mesh/peer-7")
    #expect(publishedOrigin(originSessionID: "") == "")
}
