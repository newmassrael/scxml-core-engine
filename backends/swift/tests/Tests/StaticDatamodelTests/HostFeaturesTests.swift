// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceHostFeatures
import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated Swift machine sees
// them through the reference host: every scenario of `tests/host_features/scenarios`
// replayed against the machine it names, with `ReferenceHost` registered as the invoker
// of every standard type.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` is what the host was asked. The expected values are derived from
// the specification, not observed from the host.

/// Where the host feature scenarios are, from the repository root this file lies under.
private let hostScenarioDirectory: String = {
    // <root>/backends/swift/tests/Tests/StaticDatamodelTests/HostFeaturesTests.swift
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url.appendingPathComponent("tests/host_features/scenarios").path
}()

/// Fails the test that is replaying, at the line of the test that asked for it.
private func fail(_ message: String, _ at: SourceLocation) {
    _ = Issue.record(Comment(rawValue: message), sourceLocation: at)
}

/// An integer a scenario writes, read as the type a reading takes.
private func integerValue<T: FixedWidthInteger>(_ value: JSONValue, as: T.Type) -> T? {
    guard case .number(let text) = value else {
        return nil
    }
    return T(text)
}

/// The host and its clock, which a step's `host` directives set.
private struct HostRig {
    let host: ReferenceHost
    let clock: ScriptedClockSource

    /// Applies the `host` object of a step. A directive this host does not serve fails the
    /// step that names it, and is never ignored: a step whose host was not set up as it
    /// says judges nothing.
    func apply(_ directives: [(key: String, value: JSONValue)], at: SourceLocation) {
        for (name, value) in directives {
            switch name {
            case "clock":
                guard case .object(let members) = value else {
                    fail("the `clock` directive is an object", at)
                    continue
                }
                var reading = clock.reading ?? ClockReading(utcMs: 0, monotonicMs: 0)
                var zone = clock.zoneNow ?? ClockZone(id: "", utcOffsetS: 0)
                for (member, setting) in members {
                    switch member {
                    case "utc_ms":
                        reading.utcMs = integerValue(setting, as: Int64.self) ?? reading.utcMs
                    case "monotonic_ms":
                        reading.monotonicMs = integerValue(setting, as: Int64.self) ?? reading.monotonicMs
                    case "zone_id":
                        if case .text(let id) = setting {
                            zone.id = id
                        }
                    case "utc_offset_s":
                        zone.utcOffsetS = integerValue(setting, as: Int32.self) ?? zone.utcOffsetS
                    default:
                        fail("`\(member)` is not a member of the `clock` directive", at)
                    }
                }
                clock.reading = reading
                clock.zoneNow = zone
            case "fail":
                guard case .object(let members) = value else {
                    fail("the `fail` directive is an object", at)
                    continue
                }
                var faults: [String: String] = [:]
                for (operation, code) in members {
                    if case .text(let text) = code {
                        faults[operation] = text
                    }
                }
                host.setFaults(faults)
            default:
                fail("`\(name)` is a host directive this host does not serve yet", at)
            }
        }
    }

    /// What the host was started with since the last step, as a scenario writes it.
    func starts() -> JSONValue {
        .array(
            host.takeStarts().map { start in
                let request = start.params.sorted { $0.key < $1.key }.map {
                    (key: $0.key, value: JSONValue.text($0.value))
                }
                return .object([
                    (key: "type", value: .text(start.operation)),
                    (key: "id", value: .text(start.id)),
                    (key: "request", value: .object(request)),
                ])
            }
        )
    }
}

/// Replays scenario `name` against `policy`, a machine the host serves. `variables` read
/// what the machine published.
private func replayHost<Policy: StatePolicy>(
    _ name: String,
    policy: Policy,
    variables: [String: () -> JSONValue],
    sourceLocation: SourceLocation = #_sourceLocation
) {
    let path = "\(hostScenarioDirectory)/\(name).json"
    let scenario: JSONValue
    do {
        scenario = try parseJSON(String(contentsOfFile: path, encoding: .utf8))
    } catch {
        fail("scenario \(name) is not readable: \(error)", sourceLocation)
        return
    }
    guard case .array(let steps)? = scenario.member("steps"), !steps.isEmpty else {
        fail("scenario \(name) has no steps, which judges nothing", sourceLocation)
        return
    }

    let clock = ScriptedClockSource()
    let rig = HostRig(host: ReferenceHost(clock: clock), clock: clock)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    serve(rig.host, on: engine)
    engine.initialize()
    engine.step()

    // What the replay compared. A scenario none of whose expectations was judged passed
    // for want of anything to fail on.
    var judged = 0
    defer {
        if judged == 0 {
            fail("scenario \(name) judged no expectation", sourceLocation)
        }
    }
    for (index, step) in steps.enumerated() {
        var note = ""
        if case .text(let text)? = step.member("note") {
            note = text
        }
        let label = "step \(index) (\(note))"
        if case .object(let directives)? = step.member("host") {
            rig.apply(directives, at: sourceLocation)
        }
        if case .number(let text)? = step.member("advance_ms"), let ms = Int64(text) {
            engine.advanceTimeMs(ms)
            engine.step()
        }
        if case .text(let event)? = step.member("event") {
            guard engine.resolveEventByName(event) != nil else {
                fail("\(label): the machine's events do not match `\(event)`", sourceLocation)
                continue
            }
            var data = ""
            if let payload = step.member("data") {
                data = writeJSON(payload)
            }
            engine.raiseExternalByName(event, eventData: data)
            engine.step()
        }
        guard case .object(let expect)? = step.member("expect") else {
            fail("\(label): a step states what it expects", sourceLocation)
            continue
        }
        let started = rig.starts()
        for (member, want) in expect {
            switch member {
            case "host_invokes":
                judged += 1
                if !sameValue(started, want) {
                    fail("\(label): what the host was asked: \(started), want \(want)", sourceLocation)
                }
            case "state":
                judged += 1
                let got = policy.getStateName(engine.getCurrentState())
                if case .text(let wanted) = want, got != wanted {
                    fail("\(label): the current state is \(got), want \(wanted)", sourceLocation)
                }
            case "variables":
                guard case .object(let wanted) = want else {
                    fail("\(label): `variables` is an object", sourceLocation)
                    continue
                }
                for (variable, value) in wanted {
                    guard let read = variables[variable] else {
                        fail("\(label): the driver reads no variable `\(variable)`", sourceLocation)
                        continue
                    }
                    judged += 1
                    if !sameValue(read(), value) {
                        fail("\(label): variable `\(variable)` is \(read()), not \(value)", sourceLocation)
                    }
                }
            default:
                fail("\(label): `expect.\(member)` is not judged by this driver yet", sourceLocation)
            }
        }
    }
}

@Test func aMachineReadsTheClockOfItsHost() {
    let policy = StaticHostClockPolicy()
    replayHost(
        "static_host_clock",
        policy: policy,
        variables: [
            "outcome": { .text(policy.outcome) },
            "utc_ms": { integer(policy.utcMs) },
            "monotonic_ms": { integer(policy.monotonicMs) },
            "zone_id": { .text(policy.zoneId) },
            "utc_offset_s": { integer(policy.utcOffsetS) },
        ]
    )
}
