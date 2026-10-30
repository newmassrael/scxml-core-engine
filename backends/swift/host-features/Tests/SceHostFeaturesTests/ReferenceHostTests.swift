// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

@Test func aTypeThatIsNotAStandardHostFeatureIsNotServed() {
    let (host, _) = hostOn()
    #expect(host.start(operation: "sce-host-unknown", id: "x", params: [:]) == nil)
    // Nothing was asked of the host, so nothing is recorded.
    #expect(host.takeStarts().isEmpty)
}

@Test func bothReadingsOfOneNowAreTheInstantTheHostReads() throws {
    let (host, _) = hostOn()
    let answer = try record(host.start(operation: "sce-host-clock-now", id: "now", params: [:]))
    #expect(answer.member("utc_ms") == .text("1760000000000"))
    #expect(answer.member("monotonic_ms") == .text("5000"))
    #expect(answer.member("error") == .text(""))
}

@Test func theMonotonicReadingNeverGoesBelowOneAlreadyGiven() throws {
    let (host, clock) = hostOn()
    _ = host.start(operation: "sce-host-clock-now", id: "now", params: [:])
    clock.reading = ClockReading(utcMs: 1_759_999_999_000, monotonicMs: 4000)
    let back = try record(host.start(operation: "sce-host-clock-now", id: "now", params: [:]))
    // UTC follows the clock backwards; the monotonic reading does not.
    #expect(back.member("utc_ms") == .text("1759999999000"))
    #expect(back.member("monotonic_ms") == .text("5000"))
    clock.reading = ClockReading(utcMs: 1_759_999_999_000, monotonicMs: 7500)
    let forward = try record(host.start(operation: "sce-host-clock-now", id: "now", params: [:]))
    #expect(forward.member("monotonic_ms") == .text("7500"))
}

@Test func theZoneIsTheIdentifierAndTheOffsetInForce() throws {
    let (host, clock) = hostOn()
    let seoul = try record(host.start(operation: "sce-host-clock-zone", id: "zone", params: [:]))
    #expect(seoul.member("zone_id") == .text("Asia/Seoul"))
    #expect(seoul.member("utc_offset_s") == .number("32400"))
    // A host that knows only an offset answers an empty identifier, and a negative
    // offset is west of UTC.
    clock.zoneNow = ClockZone(id: "", utcOffsetS: -18000)
    let west = try record(host.start(operation: "sce-host-clock-zone", id: "zone", params: [:]))
    #expect(west.member("zone_id") == .text(""))
    #expect(west.member("utc_offset_s") == .number("-18000"))
}

@Test func aClockThatCannotBeReadFailsUnavailableAndOnlyForTheOperationThatNeedsIt() throws {
    let (host, clock) = hostOn()
    clock.reading = nil
    let now = try record(host.start(operation: "sce-host-clock-now", id: "now", params: [:]))
    #expect(now.member("error") == .text("unavailable"))
    #expect(now.member("utc_ms") == .text("0"))
    // The failure belongs to the operation: the zone is still answered.
    let zone = try record(host.start(operation: "sce-host-clock-zone", id: "zone", params: [:]))
    #expect(zone.member("error") == .text(""))
}

@Test func aFailureTheEmbeddingInjectsIsTheCodeOfTheOperationItNames() throws {
    let (host, _) = hostOn()
    host.setFaults(["sce-host-clock-now": "unavailable"])
    let failed = try record(host.start(operation: "sce-host-clock-now", id: "now", params: [:]))
    #expect(failed.member("error") == .text("unavailable"))
    let other = try record(host.start(operation: "sce-host-clock-zone", id: "zone", params: [:]))
    #expect(other.member("error") == .text(""))
    // An empty map ends them.
    host.setFaults([:])
    let healed = try record(host.start(operation: "sce-host-clock-now", id: "now", params: [:]))
    #expect(healed.member("error") == .text(""))
}

@Test func theHostKeepsWhatItWasAskedAndWhatItWasToldToStop() {
    let (host, _) = hostOn()
    _ = host.start(operation: "sce-host-clock-now", id: "now", params: [:])
    _ = host.start(operation: "sce-host-clock-zone", id: "zone", params: [:])
    #expect(
        host.takeStarts() == [
            HostStart(operation: "sce-host-clock-now", id: "now", params: [:]),
            HostStart(operation: "sce-host-clock-zone", id: "zone", params: [:]),
        ]
    )
    // Taken once: the next call has only what came after.
    #expect(host.takeStarts().isEmpty)
    host.cancel("zone")
    host.cancel("now")
    #expect(host.cancelled == ["zone", "now"])
}
