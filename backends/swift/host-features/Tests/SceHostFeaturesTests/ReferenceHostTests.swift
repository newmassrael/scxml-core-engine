// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import SceHostFeatures

// The host's semantics, apart from any engine: what it answers a start with, the failures
// it can be given, and what it keeps of what it was asked.

/// The record a completion carries, read back.
private func record(_ outcome: HostOutcome?) throws -> JSONValue {
    guard case .done(let data)? = outcome else {
        throw HostTestError(description: "a completion was expected, not \(String(describing: outcome))")
    }
    return try parseJSON(data)
}

private struct HostTestError: Error, CustomStringConvertible {
    let description: String
}

private func hostOn(
    utcMs: Int64 = 1_760_000_000_000,
    monotonicMs: Int64 = 5000
) -> (ReferenceHost, ScriptedClockSource) {
    let clock = ScriptedClockSource()
    clock.reading = ClockReading(utcMs: utcMs, monotonicMs: monotonicMs)
    clock.zoneNow = ClockZone(id: "Asia/Seoul", utcOffsetS: 32400)
    return (ReferenceHost(clock: clock), clock)
}

/// The `start`-th start of the `<invoke>` `id` of one session. Only a put-if keeps what
/// it answered under its identity, so only a put-if asked twice needs a number of its own.
private func invocation(_ id: String, start: UInt64 = 1) -> Invocation {
    Invocation(sessionID: "session", id: id, start: start)
}

@Test func aTypeThatIsNotAStandardHostFeatureIsNotServed() {
    let (host, _) = hostOn()
    #expect(host.start(operation: "sce-host-unknown", invocation: invocation("x"), params: [:]) == nil)
    // Nothing was asked of the host, so nothing is recorded.
    #expect(host.takeStarts().isEmpty)
}

@Test func bothReadingsOfOneNowAreTheInstantTheHostReads() throws {
    let (host, _) = hostOn()
    let answer = try record(host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:]))
    #expect(answer.member("utc_ms") == .text("1760000000000"))
    #expect(answer.member("monotonic_ms") == .text("5000"))
    #expect(answer.member("error") == .text(""))
}

@Test func theMonotonicReadingNeverGoesBelowOneAlreadyGiven() throws {
    let (host, clock) = hostOn()
    _ = host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:])
    clock.reading = ClockReading(utcMs: 1_759_999_999_000, monotonicMs: 4000)
    let back = try record(host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:]))
    // UTC follows the clock backwards; the monotonic reading does not.
    #expect(back.member("utc_ms") == .text("1759999999000"))
    #expect(back.member("monotonic_ms") == .text("5000"))
    clock.reading = ClockReading(utcMs: 1_759_999_999_000, monotonicMs: 7500)
    let forward = try record(host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:]))
    #expect(forward.member("monotonic_ms") == .text("7500"))
}

@Test func theZoneIsTheIdentifierAndTheOffsetInForce() throws {
    let (host, clock) = hostOn()
    let seoul = try record(host.start(operation: "sce-host-clock-zone", invocation: invocation("zone"), params: [:]))
    #expect(seoul.member("zone_id") == .text("Asia/Seoul"))
    #expect(seoul.member("utc_offset_s") == .number("32400"))
    // A host that knows only an offset answers an empty identifier, and a negative
    // offset is west of UTC.
    clock.zoneNow = ClockZone(id: "", utcOffsetS: -18000)
    let west = try record(host.start(operation: "sce-host-clock-zone", invocation: invocation("zone"), params: [:]))
    #expect(west.member("zone_id") == .text(""))
    #expect(west.member("utc_offset_s") == .number("-18000"))
}

@Test func aClockThatCannotBeReadFailsUnavailableAndOnlyForTheOperationThatNeedsIt() throws {
    let (host, clock) = hostOn()
    clock.reading = nil
    let now = try record(host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:]))
    #expect(now.member("error") == .text("unavailable"))
    #expect(now.member("utc_ms") == .text("0"))
    // The failure belongs to the operation: the zone is still answered.
    let zone = try record(host.start(operation: "sce-host-clock-zone", invocation: invocation("zone"), params: [:]))
    #expect(zone.member("error") == .text(""))
}

@Test func aFailureTheEmbeddingInjectsIsTheCodeOfTheOperationItNames() throws {
    let (host, _) = hostOn()
    host.setFaults(["sce-host-clock-now": "unavailable"])
    let failed = try record(host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:]))
    #expect(failed.member("error") == .text("unavailable"))
    let other = try record(host.start(operation: "sce-host-clock-zone", invocation: invocation("zone"), params: [:]))
    #expect(other.member("error") == .text(""))
    // An empty map ends them.
    host.setFaults([:])
    let healed = try record(host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:]))
    #expect(healed.member("error") == .text(""))
}

@Test func theHostKeepsWhatItWasAskedAndWhatItWasToldToStop() {
    let (host, _) = hostOn()
    _ = host.start(operation: "sce-host-clock-now", invocation: invocation("now"), params: [:])
    _ = host.start(operation: "sce-host-clock-zone", invocation: invocation("zone"), params: [:])
    #expect(
        host.takeStarts() == [
            HostStart(operation: "sce-host-clock-now", id: "now", params: [:], sessionID: "session", number: 1),
            HostStart(operation: "sce-host-clock-zone", id: "zone", params: [:], sessionID: "session", number: 1),
        ]
    )
    // Taken once: the next call has only what came after.
    #expect(host.takeStarts().isEmpty)
    host.cancel("zone")
    host.cancel("now")
    #expect(host.cancelled == ["zone", "now"])
}

// MARK: - HTTP

private final class OneAnswer: HostTransport {
    var answer: HostExchange
    private(set) var asked: [HostHTTPRequest] = []

    init(_ answer: HostExchange) {
        self.answer = answer
    }

    func exchange(_ request: HostHTTPRequest) -> HostExchange {
        asked.append(request)
        return answer
    }
}

private func fetch(
    _ host: ReferenceHost,
    method: String = "GET",
    url: String = "http://example.test/a",
    headers: String = "",
    body: String = ""
) -> HostOutcome? {
    host.start(
        operation: "sce-host-http-fetch",
        invocation: invocation("fetch"),
        params: ["method": method, "url": url, "headers": headers, "body": body]
    )
}

private func httpHost(_ answer: HostExchange) -> (ReferenceHost, OneAnswer) {
    let transport = OneAnswer(answer)
    return (ReferenceHost(clock: ScriptedClockSource(), transport: transport), transport)
}

@Test func aResponseIsItsStatusItsLowercasedHeadersAndItsBody() throws {
    let (host, transport) = httpHost(
        .response(
            HostHTTPResponse(
                status: 201,
                headers: [HostHTTPHeader(name: "Content-Type", value: "  text/plain "), HostHTTPHeader(name: "X-A", value: "1")],
                body: Array("hi".utf8)
            )
        )
    )
    let answer = try record(fetch(host, method: "POST", headers: "Accept: */*\r\nX-B:  2 \n\n", body: "payload"))
    #expect(answer.member("status") == .number("201"))
    #expect(answer.member("headers") == .text("content-type: text/plain\nx-a: 1"))
    #expect(answer.member("body") == .text("hi"))
    #expect(answer.member("error") == .text(""))
    // The request lines were split into name and value, the spaces and tabs around a value
    // dropped, and the empty lines ignored.
    #expect(
        transport.asked == [
            HostHTTPRequest(
                method: "POST",
                url: "http://example.test/a",
                headers: [HostHTTPHeader(name: "Accept", value: "*/*"), HostHTTPHeader(name: "X-B", value: "2")],
                body: "payload"
            )
        ]
    )
}

@Test func anArgumentTheOperationDoesNotAcceptIsRefusedAndNothingIsAsked() {
    let (host, transport) = httpHost(.unreachable)
    let refusal = HostOutcome.refuse(#"{"reason":"invalid_argument"}"#)
    #expect(fetch(host, method: "TRACE") == refusal)
    #expect(fetch(host, method: "get") == refusal)
    for url in ["ftp://example.test/", "http://", "http://:80/", "http://exa mple.test/", "/relative"] {
        #expect(fetch(host, url: url) == refusal, "\(url) is not an http URL")
    }
    #expect(fetch(host, headers: "NoColon") == refusal)
    #expect(fetch(host, headers: "Bad Name: v") == refusal)
    #expect(transport.asked.isEmpty)
}

@Test func aTransportThatGotNoResponseFailsNetworkAndAHostWithNoTransportDoes() throws {
    let (host, _) = httpHost(.unreachable)
    #expect(try record(fetch(host)).member("error") == .text("network"))
    let bare = ReferenceHost(clock: ScriptedClockSource())
    let answer = try record(fetch(bare))
    #expect(answer.member("error") == .text("network"))
    #expect(answer.member("status") == .number("0"))
}

@Test func aTransportThatHasNotAnsweredIsHeldUntilTheHostIsToldToStop() {
    let (host, _) = httpHost(.pending)
    #expect(fetch(host) == .hold)
    host.cancel("fetch")
    #expect(host.cancelled == ["fetch"])
}

@Test func aResponseToHeadHasNoBodyAndOneTooLargeOrNotTextIsAFailure() throws {
    let big = HostHTTPResponse(status: 200, headers: [], body: [UInt8](repeating: 0x61, count: 16385))
    let (tooBig, _) = httpHost(.response(big))
    #expect(try record(fetch(tooBig)).member("error") == .text("response_too_large"))
    // The same body is none at all to a HEAD.
    let (head, _) = httpHost(.response(big))
    let headAnswer = try record(fetch(head, method: "HEAD"))
    #expect(headAnswer.member("error") == .text(""))
    #expect(headAnswer.member("body") == .text(""))
    let bad = HostHTTPResponse(status: 200, headers: [], body: [0xFF, 0xFE])
    let (notText, _) = httpHost(.response(bad))
    #expect(try record(fetch(notText)).member("error") == .text("invalid_body"))
    let wide = HostHTTPResponse(
        status: 200,
        headers: [HostHTTPHeader(name: "X", value: String(repeating: "a", count: 4096))],
        body: []
    )
    let (tooWide, _) = httpHost(.response(wide))
    #expect(try record(fetch(tooWide)).member("error") == .text("response_too_large"))
}

// MARK: - Key-value store

private func put(_ host: ReferenceHost, _ key: String, _ value: String) throws -> JSONValue {
    try record(host.start(operation: "sce-host-kv-put", invocation: invocation("put"), params: ["key": key, "value": value]))
}

@Test func aKeyIsPutReadReplacedAndDeleted() throws {
    let (host, _) = hostOn()
    #expect(try put(host, "a", "1").member("replaced") == .bool(false))
    let found = try record(host.start(operation: "sce-host-kv-get", invocation: invocation("get"), params: ["key": "a"]))
    #expect(found.member("found") == .bool(true))
    #expect(found.member("value") == .text("1"))
    #expect(try put(host, "a", "2").member("replaced") == .bool(true))
    let deleted = try record(host.start(operation: "sce-host-kv-delete", invocation: invocation("del"), params: ["key": "a"]))
    #expect(deleted.member("existed") == .bool(true))
    let gone = try record(host.start(operation: "sce-host-kv-get", invocation: invocation("get"), params: ["key": "a"]))
    #expect(gone.member("found") == .bool(false))
    #expect(gone.member("value") == .text(""))
}

@Test func aKeyThatIsEmptyOrHoldsNulIsRefusedAndStartsNothing() {
    let (host, _) = hostOn()
    for key in ["", "a\u{0}b"] {
        let outcome = host.start(operation: "sce-host-kv-put", invocation: invocation("put"), params: ["key": key, "value": "v"])
        #expect(outcome == .refuse(#"{"reason":"invalid_argument"}"#))
    }
    #expect(host.storeEntries().isEmpty)
}

/// A put-if of `asked`, the first start of `putif` in one session unless another is named.
private func putIf(
    _ host: ReferenceHost, _ key: String, _ value: String, _ expected: String,
    _ asked: Invocation = invocation("putif")
) -> HostOutcome? {
    host.start(
        operation: "sce-host-kv-put-if",
        invocation: asked,
        params: ["key": key, "value": value, "expected_version": expected]
    )
}

@Test func aPutIfWritesOnlyAtTheVersionAReadAnswered() throws {
    let (host, _) = hostOn()
    let created = try record(putIf(host, "a", "1", "0", invocation("putif", start: 1)))
    #expect(created.member("written") == .bool(true))
    #expect(created.member("version") == .text("1"))
    let read = try record(host.start(operation: "sce-host-kv-get", invocation: invocation("get"), params: ["key": "a"]))
    #expect(read.member("version") == .text("1"))
    let stale = try record(putIf(host, "a", "2", "0", invocation("putif", start: 2)))
    #expect(stale.member("written") == .bool(false))
    #expect(stale.member("version") == .text("1"))
    _ = try record(host.start(operation: "sce-host-kv-delete", invocation: invocation("del"), params: ["key": "a"]))
    // The delete took the number 2, so the key written again is past it.
    let again = try record(putIf(host, "a", "3", "0", invocation("putif", start: 3)))
    #expect(again.member("version") == .text("3"))
    #expect(host.storeEntries().map(\.value) == ["3"])
}

/// docs/adr/0107, D4: a put-if started again under the identity it had is answered with its
/// first result and writes nothing, and a start of another number, of another session or of
/// another `<invoke>` is an operation of its own.
@Test func aPutIfRepeatedUnderItsIdentityIsAnsweredWithItsFirstResult() throws {
    let (host, _) = hostOn()
    let first = Invocation(sessionID: "s1", id: "putif", start: 1)
    let wrote = putIf(host, "k", "a", "0", first)
    let created = try record(wrote)
    #expect(created.member("written") == .bool(true) && created.member("version") == .text("1"))
    // The repeat finds the key at version 1, which would answer written false; it is
    // answered as the first start was, and the store is as it was.
    #expect(putIf(host, "k", "a", "0", first) == wrote)
    #expect(host.storeEntries().map { "\($0.key)=\($0.value)" } == ["k=a"])
    for other in [
        Invocation(sessionID: "s1", id: "putif", start: 2), Invocation(sessionID: "s2", id: "putif", start: 1),
        Invocation(sessionID: "s1", id: "other", start: 1),
    ] {
        let lost = try record(putIf(host, "k", "b", "0", other))
        #expect(lost.member("written") == .bool(false), "\(other)")
        #expect(lost.member("version") == .text("1"), "\(other)")
    }
    let starts = host.takeStarts()
    #expect(starts.count == 5)
    #expect(
        starts.map { "\($0.sessionID) \($0.id) \($0.number)" }
            == ["s1 putif 1", "s1 putif 1", "s1 putif 2", "s2 putif 1", "s1 other 1"])
}

/// A refusal is no result, and a failure is a result a repeat answers.
@Test func aPutIfKeepsItsFailureAndNotItsRefusal() throws {
    let (host, _) = hostOn()
    let refusal = HostOutcome.refuse(#"{"reason":"invalid_argument"}"#)
    let refused = Invocation(sessionID: "s1", id: "putif", start: 1)
    #expect(putIf(host, "", "a", "0", refused) == refusal)
    #expect(putIf(host, "", "a", "0", refused) == refusal, "judged again, not answered from a result")
    #expect(try record(putIf(host, "k", "a", "0", refused)).member("written") == .bool(true))
    host.setQuota(1)
    let failed = Invocation(sessionID: "s1", id: "putif", start: 2)
    let first = putIf(host, "n", "a", "0", failed)
    #expect(try record(first).member("error") == .text("quota_exceeded"))
    host.setQuota(8)
    #expect(putIf(host, "n", "a", "0", failed) == first)
    #expect(host.storeEntries().map(\.key) == ["k"])
}

@Test func aPutIfWithANulKeyOrADigitOfAnotherScriptIsRefused() {
    let (host, _) = hostOn()
    let refusal = HostOutcome.refuse(#"{"reason":"invalid_argument"}"#)
    #expect(putIf(host, "a\u{0}b", "v", "0") == refusal)
    // U+0661, ARABIC-INDIC DIGIT ONE, is a number to `Character.isNumber` and not one of
    // the digits 0 to 9 an expected version is written in.
    #expect(putIf(host, "k", "v", "\u{661}") == refusal)
    #expect(host.storeEntries().isEmpty)
}

@Test func theStoreOrdersKeysByTheirUtf8BytesAndWalksThemFromAfterByPrefix() throws {
    let (host, _) = hostOn()
    for key in ["b", "ab", "a", "\u{e9}", "z"] {
        _ = try put(host, key, key)
    }
    // The order of the bytes: a, ab, b, z, then the two-byte e-acute.
    #expect(host.storeEntries().map(\.key) == ["a", "ab", "b", "z", "\u{e9}"])
    func next(after: String, prefix: String) throws -> JSONValue {
        try record(host.start(operation: "sce-host-kv-next-key", invocation: invocation("walk"), params: ["after": after, "prefix": prefix]))
    }
    #expect(try next(after: "", prefix: "").member("key") == .text("a"))
    // `after` is excluded.
    #expect(try next(after: "a", prefix: "").member("key") == .text("ab"))
    #expect(try next(after: "a", prefix: "b").member("key") == .text("b"))
    let end = try next(after: "\u{e9}", prefix: "")
    #expect(end.member("found") == .bool(false))
    #expect(end.member("key") == .text(""))
}

@Test func aNewKeyPastTheQuotaFailsAndAReplacementDoesNot() throws {
    let (host, _) = hostOn()
    host.setQuota(1)
    _ = try put(host, "a", "1")
    let over = try put(host, "b", "2")
    #expect(over.member("error") == .text("quota_exceeded"))
    #expect(over.member("replaced") == .bool(false))
    #expect(try put(host, "a", "3").member("replaced") == .bool(true))
    #expect(host.storeEntries().map(\.value) == ["3"])
}

// MARK: - Notifications

private func post(_ host: ReferenceHost, _ id: String, atMs: String = "0") throws -> JSONValue {
    try record(
        host.start(
            operation: "sce-host-notify-post",
            invocation: invocation("post"),
            params: ["id": id, "title": "t", "body": "b", "at_ms": atMs]
        )
    )
}

@Test func aPostIsDeniedUnlessThePermissionIsGranted() throws {
    let (host, _) = hostOn()
    #expect(try post(host, "n").member("status") == .text("denied"))
    #expect(host.shown().isEmpty && host.waiting().isEmpty)
}

@Test func permissionIsAskedOfAUserOnlyWhileUndetermined() throws {
    let (host, _) = hostOn()
    host.setUserDecision("granted")
    let asked = try record(host.start(operation: "sce-host-notify-request-permission", invocation: invocation("ask"), params: [:]))
    #expect(asked.member("state") == .text("granted"))
    // Decided: a later answer of the user changes nothing.
    host.setUserDecision("denied")
    let again = try record(host.start(operation: "sce-host-notify-request-permission", invocation: invocation("ask"), params: [:]))
    #expect(again.member("state") == .text("granted"))
    let (nobody, _) = hostOn()
    let none = try record(nobody.start(operation: "sce-host-notify-request-permission", invocation: invocation("ask"), params: [:]))
    #expect(none.member("state") == .text("denied"))
}

@Test func aNotificationIsShownAtOnceOrWaitsForItsInstantAndIsReplacedByItsId() throws {
    let (host, _) = hostOn()
    host.setPermission("granted")
    #expect(try post(host, "now", atMs: "1760000000000").member("status") == .text("posted"))
    #expect(try post(host, "later", atMs: "1760000000001").member("status") == .text("scheduled"))
    #expect(host.shown() == ["now"])
    #expect(host.waiting() == ["later"])
    // The same id again replaces what it named, wherever it was.
    #expect(try post(host, "later", atMs: "0").member("status") == .text("posted"))
    #expect(host.waiting().isEmpty)
    #expect(host.shown() == ["now", "later"])
}

@Test func aCancelWithdrawsFromBothListsAndSaysWhetherItNamedAnything() throws {
    let (host, _) = hostOn()
    host.setPermission("granted")
    _ = try post(host, "n")
    let cancel = { (id: String) throws -> JSONValue in
        try record(host.start(operation: "sce-host-notify-cancel", invocation: invocation("cancel"), params: ["id": id]))
    }
    #expect(try cancel("n").member("existed") == .bool(true))
    #expect(try cancel("n").member("existed") == .bool(false))
    #expect(host.start(operation: "sce-host-notify-cancel", invocation: invocation("cancel"), params: ["id": ""]) == .refuse(#"{"reason":"invalid_argument"}"#))
}
