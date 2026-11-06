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

@Test(arguments: ["session-1", "a b/c#d", "parent.state.1.inv_peer", "\u{E9}t\u{E9}", "parent:invoke.1", "x%2Fy", "100%"])
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

@Test func aReservedCharacterIsPercentEncodedOnce() {
    // `%` is itself reserved: an id that already looks encoded is encoded again, so
    // decoding gives back the id and not what it looked like.
    #expect(scxmlProcessorLocation(sessionID: "a b/c#d") == "sce://scxml/a%20b%2Fc%23d")
    #expect(scxmlProcessorLocation(sessionID: "x%2Fy") == "sce://scxml/x%252Fy")
    #expect(sessionIDFromScxmlLocation("sce://scxml/x%252Fy") == "x%2Fy")
}

// docs/adr/0090, section 2: the pair the SCXML processor writes for a sender is its type and
// the location the sender publishes, the one `_ioprocessors` holds.
@Test func theSendersPairIsTheProcessorsTypeAndItsPublishedLocation() {
    let descriptors = buildIoProcessors(sessionID: "a b", basicHTTPAccessURI: "")
    let origin = scxmlOrigin(sender: "a b")
    #expect(origin.processor == scxmlEventProcessorType)
    #expect(origin.address == locationOf(descriptors, scxmlProcessorAlias))
    #expect(scxmlOrigin(of: "a b") == origin)
    #expect(scxmlOrigin(of: "") == nil, "no sender has no address to publish")
}

@Test func anEventASessionSendsCarriesItsPairAndItsSender() {
    let metadata = EventMetadata.external(sendID: "s1", sender: "kid")
    #expect(metadata.origin == scxmlOrigin(sender: "kid"))
    #expect(metadata.sender == "kid")
    #expect(metadata.originAddress == "sce://scxml/kid")
    #expect(metadata.originProcessor == scxmlEventProcessorType)
    let none = EventMetadata(eventType: .externalEvent)
    #expect(none.origin == nil && none.sender == nil)
    #expect(none.originAddress == "" && none.originProcessor == "", "no origin reads both fields as empty")
}

// docs/adr/0090, section 7: a format-1 or format-2 origin, as a saved state stores it, is read
// back as the pair and the sender.
@Test func aFormat2OriginIsReadAsThePairItStoodFor() {
    let none = readStoredOrigin("", originType: "")
    #expect(none.origin == nil && none.sender == nil)
    // An empty address is none whatever type was saved beside it: format 2 wrote the SCXML type
    // beside events no processor stamped, and half a pair is never read back.
    let blankScxml = readStoredOrigin("", originType: scxmlEventProcessorType)
    #expect(blankScxml.origin == nil && blankScxml.sender == nil)
    let blankHost = readStoredOrigin("", originType: "x-sce-host")
    #expect(blankHost.origin == nil && blankHost.sender == nil)
    // An address saved with no type is the SCXML processor's, as format 2 meant.
    let untypedMesh = readStoredOrigin("mesh://motor", originType: "")
    #expect(untypedMesh.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://motor"))
    #expect(untypedMesh.sender == nil)
    let untypedID = readStoredOrigin("kid", originType: "")
    #expect(untypedID.origin == scxmlOrigin(sender: "kid"))
    #expect(untypedID.sender == "kid")
    // A bare id with the SCXML type: that session's pair, the id its sender.
    let bare = readStoredOrigin("root.work.1.kid", originType: scxmlEventProcessorType)
    #expect(bare.origin == scxmlOrigin(sender: "root.work.1.kid"))
    #expect(bare.sender == "root.work.1.kid")
    // An SCXML location, whatever the stored type: the pair, the decoded id its sender.
    let located = readStoredOrigin("sce://scxml/a%20b", originType: "")
    #expect(located.origin == scxmlOrigin(sender: "a b"))
    #expect(located.sender == "a b")
    // Anything else: the pair as stored, no sender.
    let host = readStoredOrigin("motor", originType: "x-sce-host")
    #expect(host.origin == EventOrigin(processor: "x-sce-host", address: "motor"))
    #expect(host.sender == nil)
    let mesh = readStoredOrigin("mesh://motor", originType: scxmlEventProcessorType)
    #expect(mesh.origin == EventOrigin(processor: scxmlEventProcessorType, address: "mesh://motor"))
    #expect(mesh.sender == nil)
    // A waiting send's origin names its sender, bare or located.
    #expect(readStoredSender("fixture-root") == "fixture-root")
    #expect(readStoredSender("sce://scxml/a%20b") == "a b")
}

// A format-1 text written back stores the origin by the rule format 2 used (docs/adr/0090, section 7).
@Test func anEventOfAFormat1TextIsStoredAsFormat2StoredIt() {
    #expect(storedOriginText(scxmlOrigin(sender: "kid"), sender: "kid") == "kid")
    #expect(storedOriginText(EventOrigin(processor: "x-sce-host", address: "motor"), sender: nil) == "motor")
    #expect(storedOriginText(nil, sender: nil) == "")
}
