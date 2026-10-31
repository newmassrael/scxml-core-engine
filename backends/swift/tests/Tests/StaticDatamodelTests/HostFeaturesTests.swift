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

/// `value` with the members of every object in the order of their keys' bytes: an object a
/// scenario writes has no order, and a host's record of what it holds has no reason to
/// write one in the author's.
private func byKey(_ value: JSONValue) -> JSONValue {
    switch value {
    case .object(let members):
        return .object(
            members.map { (key: $0.key, value: byKey($0.value)) }
                .sorted { $0.key.utf8.lexicographicallyPrecedes($1.key.utf8) }
        )
    case .array(let items):
        return .array(items.map(byKey))
    default:
        return value
    }
}

/// A transport that answers what a scenario says and keeps what it was given.
private final class ScriptedTransport: HostTransport {
    private var answer: HostExchange?
    private(set) var log: [HostHTTPRequest] = []

    func exchange(_ request: HostHTTPRequest) -> HostExchange {
        log.append(request)
        return answer ?? .unreachable
    }

    /// Sets what a step's `http` says the transport answers: `hold` does not answer, `fail`
    /// is a transport that got no response, and otherwise the status, the header pairs and
    /// a body given as text, as hex bytes or as a text repeated.
    func apply(_ directive: [(key: String, value: JSONValue)], at: SourceLocation) {
        func member(_ name: String) -> JSONValue? {
            directive.first { $0.key == name }?.value
        }
        if case .bool(true)? = member("hold") {
            answer = .pending
            return
        }
        if member("fail") != nil {
            answer = .unreachable
            return
        }
        var headers: [HostHTTPHeader] = []
        if case .array(let pairs)? = member("headers") {
            for pair in pairs {
                if case .array(let items) = pair, items.count == 2, case .text(let name) = items[0],
                    case .text(let value) = items[1]
                {
                    headers.append(HostHTTPHeader(name: name, value: value))
                }
            }
        }
        var body: [UInt8] = []
        if case .text(let text)? = member("body") {
            body = Array(text.utf8)
        } else if case .text(let hex)? = member("body_hex") {
            let digits = Array(hex.utf8)
            var index = 0
            while index + 1 < digits.count {
                if let byte = UInt8(String(decoding: digits[index...(index + 1)], as: UTF8.self), radix: 16) {
                    body.append(byte)
                }
                index += 2
            }
        } else if case .array(let repeated)? = member("body_repeat"), repeated.count == 2,
            case .text(let unit) = repeated[0], let count = integerValue(repeated[1], as: Int.self)
        {
            for _ in 0..<count {
                body.append(contentsOf: Array(unit.utf8))
            }
        }
        guard let status = member("status").flatMap({ integerValue($0, as: Int.self) }) else {
            fail("an `http` directive answers a status, hold, or fail", at)
            return
        }
        answer = .response(HostHTTPResponse(status: status, headers: headers, body: body))
    }
}

/// The host, its clock and its transport, which a step's `host` directives set.
private struct HostRig {
    let host: ReferenceHost
    let clock: ScriptedClockSource
    let transport: ScriptedTransport

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
            case "permission":
                if case .text(let state) = value {
                    host.setPermission(state)
                }
            case "user_decision":
                if case .text(let decision) = value {
                    host.setUserDecision(decision)
                }
            case "quota":
                if let keys = integerValue(value, as: Int.self) {
                    host.setQuota(keys)
                }
            case "http":
                guard case .object(let members) = value else {
                    fail("the `http` directive is an object", at)
                    continue
                }
                transport.apply(members, at: at)
            default:
                fail("`\(name)` is a host directive this host does not serve yet", at)
            }
        }
    }

    /// What the host holds, for the members a scenario names, as it writes them.
    func hostState(_ wanted: [(key: String, value: JSONValue)], at: SourceLocation) -> JSONValue {
        var state: [(key: String, value: JSONValue)] = []
        for (name, _) in wanted {
            switch name {
            case "store":
                state.append(
                    (
                        key: name,
                        value: .object(host.storeEntries().map { (key: $0.key, value: JSONValue.text($0.value)) })
                    )
                )
            case "waiting":
                state.append((key: name, value: .array(host.waiting().map { JSONValue.text($0) })))
            case "shown":
                state.append((key: name, value: .array(host.shown().map { JSONValue.text($0) })))
            case "cancelled":
                state.append((key: name, value: .array(host.cancelled.map { JSONValue.text($0) })))
            case "http":
                // What the transport was given in all: method, URL, the request's header
                // lines as name and value, and the body.
                let given = transport.log.map { request -> JSONValue in
                    let headers = request.headers.map { header -> JSONValue in
                        .object([(key: "name", value: .text(header.name)), (key: "value", value: .text(header.value))])
                    }
                    return .object([
                        (key: "method", value: .text(request.method)),
                        (key: "url", value: .text(request.url)),
                        (key: "headers", value: .array(headers)),
                        (key: "body", value: .text(request.body)),
                    ])
                }
                state.append((key: name, value: .array(given)))
            default:
                fail("`\(name)` is host state this host does not hold yet", at)
            }
        }
        return .object(state)
    }

    /// What the host was started with since the last step, as a scenario writes it.
    func starts() -> JSONValue {
        .array(
            host.takeStarts().map { start in
                // The request as the record: `at_ms`, the one integer field a request
                // has, is a number, and every other field is the text it was handed.
                let request = start.params.sorted { $0.key < $1.key }.map { member -> (key: String, value: JSONValue) in
                    if member.key == "at_ms" {
                        return (key: member.key, value: .number(member.value))
                    }
                    return (key: member.key, value: .text(member.value))
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
    let transport = ScriptedTransport()
    let rig = HostRig(host: ReferenceHost(clock: clock, transport: transport), clock: clock, transport: transport)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    serve(rig.host, on: engine)
    engine.initialize()
    // `tick` and not `step`: it takes what has come due in the scheduler, which is where
    // the end of an invocation's time waits.
    engine.tick()

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
            engine.tick()
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
            engine.tick()
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
                if !sameValue(byKey(started), byKey(want)) {
                    fail("\(label): what the host was asked: \(started), want \(want)", sourceLocation)
                }
            case "host_state":
                guard case .object(let wanted) = want else {
                    fail("\(label): `host_state` is an object", sourceLocation)
                    continue
                }
                judged += 1
                let held = rig.hostState(wanted, at: sourceLocation)
                if !sameValue(byKey(held), byKey(want)) {
                    fail("\(label): what the host holds: \(held), want \(want)", sourceLocation)
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

@Test func aMachineAsksTheKeyValueStoreOfItsHost() {
    let policy = StaticHostKvPolicy()
    replayHost(
        "static_host_kv",
        policy: policy,
        variables: [
            "outcome": { .text(policy.outcome) },
            "found": { .bool(policy.found) },
            "got": { .text(policy.got) },
            "replaced": { .bool(policy.replaced) },
            "existed": { .bool(policy.existed) },
            "next_found": { .bool(policy.nextFound) },
            "next": { .text(policy.next) },
        ]
    )
}

@Test func aMachineMakesAnHTTPExchangeThroughItsHost() {
    let policy = StaticHostHttpPolicy()
    replayHost(
        "static_host_http",
        policy: policy,
        variables: [
            "outcome": { .text(policy.outcome) },
            "status": { integer(policy.status) },
            "response_headers": { .text(policy.responseHeaders) },
            "response_body": { .text(policy.responseBody) },
        ]
    )
}

@Test func aMachinePostsNotificationsThroughItsHost() {
    let policy = StaticHostNotifyPolicy()
    replayHost(
        "static_host_notify",
        policy: policy,
        variables: [
            "outcome": { .text(policy.outcome) },
            "permission": { .text(policy.permission) },
            "status": { .text(policy.status) },
            "existed": { .bool(policy.existed) },
        ]
    )
}
