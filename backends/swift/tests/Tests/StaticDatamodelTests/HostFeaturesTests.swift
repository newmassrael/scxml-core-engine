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
    let model = ScriptedProvider()

    /// Applies a `model` directive to the scripted provider: each member it names. An
    /// answer is `text`, or `text_repeat` as a text and a count, with `finish`, `stop` when
    /// not written.
    func applyModel(_ members: [(key: String, value: JSONValue)], at: SourceLocation) {
        for (member, setting) in members {
            switch (member, setting) {
            case ("answers", .array(let answers)):
                model.answers = answers.compactMap { answer in
                    var text: String?
                    if case .text(let written)? = answer.member("text") {
                        text = written
                    } else if case .array(let repeated)? = answer.member("text_repeat"), repeated.count == 2,
                        case .text(let unit) = repeated[0], let count = integerValue(repeated[1], as: Int.self)
                    {
                        text = String(repeating: unit, count: count)
                    }
                    guard let text else {
                        fail("an answer of the `model` directive has no text", at)
                        return nil
                    }
                    var finish = "stop"
                    if case .text(let written)? = answer.member("finish") {
                        finish = written
                    }
                    return ScriptedAnswer(text: text, finish: finish)
                }
            case ("constrained", .bool(let constrained)):
                model.constrained = constrained
            case ("name", .text(let name)):
                model.name = name
            case ("context", _):
                model.context = integerValue(setting, as: UInt64.self) ?? model.context
            case ("hold", .bool(let hold)):
                model.hold = hold
            default:
                fail("`\(member)` is not a member of the `model` directive", at)
            }
        }
    }

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
            case "model":
                guard case .object(let members) = value else {
                    fail("the `model` directive is an object", at)
                    continue
                }
                applyModel(members, at: at)
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
                // The request as the record: its integer fields are numbers, and every
                // other field is the text it was handed.
                let request = start.params.sorted { $0.key < $1.key }.map { member -> (key: String, value: JSONValue) in
                    if ["at_ms", "max_tokens", "temperature_milli"].contains(member.key) {
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

/// Replays the scenario `scenario` (`name`'s own when none) against `policy`, a machine of
/// the document `name` the host serves, which the scenario names: a machine whose run ends
/// has one scenario a run (the reference harness's cases). `variables` read what the
/// machine published.
private func replayHost<Policy: StatePolicy>(
    _ name: String,
    scenario stemGiven: String? = nil,
    policy: Policy,
    variables: [String: () -> JSONValue],
    answerSchemas: [AnswerSchema] = [],
    sourceLocation: SourceLocation = #_sourceLocation
) {
    let stem = stemGiven ?? name
    let path = "\(hostScenarioDirectory)/\(stem).json"
    let scenario: JSONValue
    do {
        scenario = try parseJSON(String(contentsOfFile: path, encoding: .utf8))
    } catch {
        fail("scenario \(stem) is not readable: \(error)", sourceLocation)
        return
    }
    guard case .text(let machine)? = scenario.member("machine"), machine == name else {
        fail("scenario \(stem) names another machine than \(name)", sourceLocation)
        return
    }
    guard case .array(let steps)? = scenario.member("steps"), !steps.isEmpty else {
        fail("scenario \(stem) has no steps, which judges nothing", sourceLocation)
        return
    }

    let clock = ScriptedClockSource()
    let transport = ScriptedTransport()
    let rig = HostRig(host: ReferenceHost(clock: clock, transport: transport), clock: clock, transport: transport)
    rig.host.setModelProvider(rig.model)
    rig.host.setAnswerSchemas(answerSchemas)
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    do {
        try serve(rig.host, on: engine)
    } catch {
        fail("the reference host could not be registered: \(error)", sourceLocation)
        return
    }
    try engine.initialize()
    // `tick` and not `step`: it takes what has come due in the scheduler, which is where
    // the end of an invocation's time waits.
    engine.tick()

    // What the replay compared. A scenario none of whose expectations was judged passed
    // for want of anything to fail on.
    var judged = 0
    defer {
        if judged == 0 {
            fail("scenario \(stem) judged no expectation", sourceLocation)
        }
    }
    for (index, step) in steps.enumerated() {
        var note = ""
        if case .text(let text)? = step.member("note") {
            note = text
        }
        let label = "\(stem) step \(index) (\(note))"
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
                    if !sameValue(byKey(read()), byKey(value)) {
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

/// What static_host_model publishes, each as a scenario states it: a record by its fields in
/// the order the scenario writes them, a 64-bit field as its digits, an enum by its name.
private func modelVariables(_ policy: StaticHostModelPolicy) -> [String: () -> JSONValue] {
    let answer = { (a: StaticHostModelAnswerRecord) -> JSONValue in
        .object([
            (key: "id", value: integer(a.id)),
            (key: "amount", value: .text(String(a.amount))),
            (key: "title", value: .text(a.title)),
            (key: "kind", value: .text(a.kind.declaredName)),
            (key: "note", value: optionalText(a.note)),
            (key: "tag", value: .text(bytesAsPayloadText(a.tag))),
            (key: "priority", value: integer(a.priority)),
        ])
    }
    let why = { (w: StaticHostModelLiftRefusalRecord) -> JSONValue in
        .object([
            (key: "code", value: .text(w.code.declaredName)),
            (key: "path", value: .text(w.path)),
            (key: "bound", value: optionalInteger(w.bound)),
        ])
    }
    return [
        "calls": { integer(policy.calls) },
        "outcome": { .text(policy.outcome) },
        "finish": { .text(policy.finish) },
        "constrained": { .bool(policy.constrained) },
        "answered_by": { .text(policy.answeredBy) },
        "prompt_tokens": { integer(policy.promptTokens) },
        "answer_tokens": { integer(policy.answerTokens) },
        "answer": { policy.answer.map(answer) ?? .null },
        "why": { policy.why.map(why) ?? .null },
    ]
}

@Test func aMachineAsksItsHostForATextUnderAnAnswerSchema() {
    let policy = StaticHostModelPolicy()
    replayHost(
        "static_host_model",
        policy: policy,
        variables: modelVariables(policy),
        answerSchemas: StaticHostModelAnswerSchemas.all
    )
}

/// What the exhaustive provider answers for a class of the table.
private func outcomeClass(_ cls: JSONValue) -> OutcomeClass {
    func text(_ name: String) -> String {
        if case .text(let value)? = cls.member(name) {
            return value
        }
        return ""
    }
    switch text("end") {
    case "answer":
        return .answer(text: text("text"), finish: text("finish"))
    case "failure":
        return .failure(text("error"))
    case "refusal":
        return .refusal
    default:
        return .hold
    }
}

/// Runs static_host_model against the exhaustive provider answering the classes `sequence`
/// names, one call a class, and holds what the machine publishes after each call to what its
/// class says.
private func runSequence(_ table: JSONValue, _ classes: [JSONValue], _ sequence: [Int], at: SourceLocation) {
    let named = sequence.map { index -> String in
        if case .text(let name)? = classes[index].member("class") {
            return name
        }
        return "?"
    }
    let policy = StaticHostModelPolicy()
    let host = ReferenceHost(clock: ScriptedClockSource(), transport: ScriptedTransport())
    host.setAnswerSchemas(StaticHostModelAnswerSchemas.all)
    host.setModelProvider(ExhaustiveProvider(sequence.map { outcomeClass(classes[$0]) }))
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    do {
        try serve(host, on: engine)
    } catch {
        fail("the reference host could not be registered: \(error)", at)
        return
    }
    try engine.initialize()
    engine.tick()
    guard case .text(let event)? = table.member("ask")?.member("event"),
        let data = table.member("ask")?.member("data"),
        case .text(let leave)? = table.member("leave"),
        case .number(let deadline)? = table.member("deadline_ms"), let deadlineMs = Int64(deadline)
    else {
        fail("the class table names no request, leave event or deadline", at)
        return
    }
    let variables = modelVariables(policy)
    var answer: JSONValue = .null
    for (n, index) in sequence.enumerated() {
        let cls = classes[index]
        engine.raiseExternalByName(event, eventData: writeJSON(data))
        engine.tick()
        if case .text(let then)? = cls.member("then") {
            if then == "advance" {
                engine.advanceTimeMs(deadlineMs)
                engine.tick()
            } else if then == "leave" {
                engine.raiseExternalByName(leave, eventData: "")
                engine.tick()
            }
        }
        guard let expect = cls.member("expect") else {
            fail("a class states what it expects", at)
            return
        }
        if let read = expect.member("answer") {
            answer = read
        }
        let label = "\(named), call \(n + 1)"
        if policy.getStateName(engine.getCurrentState()) != "idle" {
            fail("\(label): the machine is not idle", at)
        }
        for name in ["outcome", "why", "finish"] {
            let got = variables[name]?() ?? .null
            let want = expect.member(name) ?? .null
            if !sameValue(byKey(got), byKey(want)) {
                fail("\(label): `\(name)` is \(got), not \(want)", at)
            }
        }
        let held = variables["answer"]?() ?? .null
        if !sameValue(byKey(held), byKey(answer)) {
            fail("\(label): `answer` is \(held), not \(answer)", at)
        }
        if !sameValue(variables["calls"]?() ?? .null, integer(n + 1)) {
            fail("\(label): `calls` is \(variables["calls"]?() ?? .null)", at)
        }
    }
}

/// The machine around a model is tested exhaustively (docs/adr/0087, section 9.4): every
/// sequence of outcome classes up to the table's bound, each call read as its class says, and
/// every refusal code of the library either reached by a class or named as deferred with its
/// reason.
@Test func everySequenceOfOutcomeClassesIsReadAsItsClassesSay() {
    let at = #_sourceLocation
    let table: JSONValue
    do {
        table = try parseJSON(
            String(contentsOfFile: "\(hostScenarioDirectory)/../model_outcome_classes.json", encoding: .utf8)
        )
    } catch {
        fail("the class table is not readable: \(error)", at)
        return
    }
    guard case .array(let classes)? = table.member("classes"), !classes.isEmpty,
        case .number(let boundText)? = table.member("bound"), let bound = Int(boundText), bound > 0,
        case .text(let schema)? = table.member("answer_schema"),
        case .array(let deferred)? = table.member("deferred")
    else {
        fail("the class table names nothing to run", at)
        return
    }
    #expect(StaticHostModelAnswerSchemas.all.first?.id == schema)
    var runs = 0
    var expected = 0
    var sequences: [[Int]] = [[]]
    for _ in 1...bound {
        sequences = sequences.flatMap { prefix in classes.indices.map { prefix + [$0] } }
        expected += sequences.count
        for sequence in sequences {
            runSequence(table, classes, sequence, at: at)
            runs += 1
        }
    }
    #expect(runs == expected)
    var codes = Set<String>()
    for cls in classes {
        if case .text(let code)? = cls.member("expect")?.member("why")?.member("code") {
            codes.insert(code)
        }
    }
    for entry in deferred {
        if case .text(let code)? = entry.member("code") {
            codes.insert(code)
        }
    }
    // The fourteen codes, tooFew the last (docs/adr/0086, section 15.11).
    #expect(codes.count == 14, "every refusal code is reached or deferred: \(codes.sorted())")
}

/// The cases of the reference harness's scenario (docs/adr/0087, section 9.1 item 8), one
/// scenario each: a run of the harness ends at one of its finals, so each case is a run of its
/// own.
private let harnessScenarios = [
    "static_model_harness_accepted",
    "static_model_harness_exhausted",
    "static_model_harness_not_offered",
    "static_model_harness_repaired",
    "static_model_harness_timeout",
]

/// What static_model_harness publishes, each as a scenario states it: a record by its fields,
/// a 64-bit field as its digits, an enum by its name.
private func harnessVariables(_ policy: StaticModelHarnessPolicy) -> [String: () -> JSONValue] {
    [
        "outcome": { .text(policy.outcome) },
        "attempts": { integer(policy.attempts) },
        "offered": { integers(policy.offered) },
        "refusals": {
            .array(
                policy.refusals.map { refusal -> JSONValue in
                    .object([
                        (key: "code", value: .text(refusal.code.declaredName)),
                        (key: "path", value: .text(refusal.path)),
                    ])
                }
            )
        },
        "answer": {
            policy.answer.map { a -> JSONValue in
                .object([
                    (key: "id", value: integer(a.id)),
                    (key: "amount", value: .text(String(a.amount))),
                    (key: "title", value: .text(a.title)),
                    (key: "kind", value: .text(a.kind.declaredName)),
                    (key: "note", value: optionalText(a.note)),
                    (key: "tag", value: .text(bytesAsPayloadText(a.tag))),
                    (key: "priority", value: integer(a.priority)),
                ])
            } ?? .null
        },
        "why": {
            policy.why.map { w -> JSONValue in
                .object([
                    (key: "code", value: .text(w.code.declaredName)),
                    (key: "path", value: .text(w.path)),
                    (key: "bound", value: optionalInteger(w.bound)),
                ])
            } ?? .null
        },
    ]
}

@Test func theReferenceHarnessRunsEachCaseOfItsScenario() {
    for stem in harnessScenarios {
        let policy = StaticModelHarnessPolicy()
        replayHost(
            "static_model_harness",
            scenario: stem,
            policy: policy,
            variables: harnessVariables(policy),
            answerSchemas: StaticModelHarnessAnswerSchemas.all
        )
    }
}

/// The codes of the application's own checks, after the fourteen of the lift its enum repeats
/// (enum_harness_code.scxml), each named for the class that reaches it.
private let harnessApplicationCodes = ["notOffered"]

/// The ticks a harness run is given after each event: a run may make its calls and take each
/// completion within one event, a call a round.
private let harnessSettleTicks = 8

private func settleHarness<Policy: StatePolicy>(_ engine: Engine<Policy>) {
    for _ in 0..<harnessSettleTicks {
        engine.tick()
    }
}

/// A member of a class or a section that is a text, or "" for one that is not.
private func textMember(_ value: JSONValue, _ name: String) -> String {
    if case .text(let text)? = value.member(name) {
        return text
    }
    return ""
}

/// The end of the host operation a class gives a call (docs/adr/0087, section 9.4): an answer
/// by its `finish`, a failure by its code, the refusal, and a held call, whichever way it is
/// left.
private func hostOutcome(_ cls: JSONValue) -> String {
    switch textMember(cls, "end") {
    case "answer":
        return "answer:" + textMember(cls, "finish")
    case "failure":
        return textMember(cls, "error")
    case "refusal":
        return "invalid_argument"
    default:
        return "hold"
    }
}

/// What one run of the harness against the exhaustive provider showed.
private struct HarnessRun {
    /// Every row the transition observer was told, in order.
    var rows: [TransitionTaken<StaticModelHarnessState>]
    /// How many calls the host was asked to start.
    var calls: Int
    /// The state the run ended in.
    var ended: String
    /// The code of each refusal the harness kept, in order.
    var codes: [String]
}

private func harnessRowName(_ row: TransitionTaken<StaticModelHarnessState>) -> String {
    "\(row.source)[\(row.index)]"
}

/// Runs the reference harness against the exhaustive provider answering the classes `sequence`
/// names, one call a class: offers it the table's ids, starts it, and leaves each call it holds
/// as the table's harness section says, until it ends. Nil, with the failure recorded, when the
/// run cannot be made.
private func runHarness(_ table: JSONValue, _ classes: [JSONValue], _ sequence: [Int], at: SourceLocation)
    -> HarnessRun?
{
    guard let harness = table.member("harness"),
        case .array(let offered)? = harness.member("offered"),
        let data = harness.member("data"),
        case .number(let boundText)? = harness.member("bound"), let bound = Int(boundText),
        case .number(let deadlineText)? = harness.member("deadline_ms"), let deadlineMs = Int64(deadlineText)
    else {
        fail("the class table's harness section names no offer, start, bound or deadline", at)
        return nil
    }
    if textMember(harness, "then") != "advance" {
        fail("a held call of the harness is left by its own deadline", at)
        return nil
    }
    let policy = StaticModelHarnessPolicy()
    let host = ReferenceHost(clock: ScriptedClockSource(), transport: ScriptedTransport())
    host.setAnswerSchemas(StaticModelHarnessAnswerSchemas.all)
    host.setModelProvider(ExhaustiveProvider(sequence.map { outcomeClass(classes[$0]) }))
    let engine = Engine(policy: policy)
    var rows: [TransitionTaken<StaticModelHarnessState>] = []
    engine.setTransitionObserver { rows.append($0) }
    engine.setClock(ManualClock())
    do {
        try serve(host, on: engine)
    } catch {
        fail("the reference host could not be registered: \(error)", at)
        return nil
    }
    try engine.initialize()
    settleHarness(engine)
    for id in offered {
        let offer = writeJSON(.object([(key: "id", value: id)]))
        engine.raiseExternalByName(textMember(harness, "offer"), eventData: offer)
        settleHarness(engine)
    }
    engine.raiseExternalByName(textMember(harness, "start"), eventData: writeJSON(data))
    settleHarness(engine)
    let current = { policy.getStateName(engine.getCurrentState()) }
    var waits = 0
    while current() != "accepted" && current() != "gave_up" {
        if waits > bound {
            fail("\(sequence): the run neither ended nor held a call it could be moved past", at)
            return nil
        }
        engine.advanceTimeMs(deadlineMs)
        settleHarness(engine)
        waits += 1
    }
    return HarnessRun(
        rows: rows,
        calls: host.takeStarts().count,
        ended: current(),
        codes: policy.refusals.map { $0.code.declaredName }
    )
}

/// The reference harness is tested exhaustively, and its coverage is measured (docs/adr/0087,
/// section 9.4 item 1, Landing order item 7): every sequence of `bound` outcome classes is a
/// run, the transition observer of docs/adr/0089 tells each row of the emitted transition table
/// the run takes, and the runs together take every row of the table, reach every refusal code,
/// and reach every host outcome at every call site (a row that enters the state whose invoke is
/// the call).
///
/// The harness makes at most `bound` calls and the provider answers a call by its position
/// alone, so every sequence of fewer classes the harness can consume is a prefix of a sequence
/// run here, and none is asked past.
@Test func everySequenceOfOutcomeClassesTakesEveryRowOfTheReferenceHarness() {
    let at = #_sourceLocation
    let table: JSONValue
    do {
        table = try parseJSON(
            String(contentsOfFile: "\(hostScenarioDirectory)/../model_outcome_classes.json", encoding: .utf8)
        )
    } catch {
        fail("the class table is not readable: \(error)", at)
        return
    }
    guard case .array(let classes)? = table.member("classes"), !classes.isEmpty,
        let harness = table.member("harness"),
        case .number(let boundText)? = harness.member("bound"), let bound = Int(boundText), bound > 0,
        case .text(let schema)? = table.member("answer_schema"),
        case .array(let deferredEntries)? = table.member("deferred")
    else {
        fail("the class table names nothing to run", at)
        return
    }
    #expect(textMember(harness, "machine") == "static_model_harness")
    #expect(StaticModelHarnessAnswerSchemas.all.first?.id == schema)
    let call = textMember(harness, "call")
    let rows = Set(StaticModelHarnessTransition.all.map { TransitionTaken(source: $0.from, index: $0.index) })
    let sites = Set(
        StaticModelHarnessTransition.all.filter { $0.targets == call }.map {
            TransitionTaken(source: $0.from, index: $0.index)
        }
    )
    if sites.isEmpty {
        fail("no row of the table enters `\(call)`", at)
        return
    }
    let outcomes = Set(classes.map(hostOutcome))

    var taken = Set<TransitionTaken<StaticModelHarnessState>>()
    var codes = Set<String>()
    var reached = Set<String>()
    var sequences: [[Int]] = [[]]
    for _ in 0..<bound {
        sequences = sequences.flatMap { prefix in classes.indices.map { prefix + [$0] } }
    }
    var runs = 0
    for sequence in sequences {
        let named = sequence.map { textMember(classes[$0], "class") }
        guard let run = runHarness(table, classes, sequence, at: at) else {
            return
        }
        if run.ended != "accepted" && run.ended != "gave_up" {
            fail("\(named): the run ended in `\(run.ended)`, not a final", at)
            return
        }
        let called = run.rows.filter { sites.contains($0) }
        if called.count != run.calls || run.calls > bound {
            fail("\(named): \(run.calls) calls, \(called.count) of them by a row that enters `\(call)`", at)
            return
        }
        for (site, index) in zip(called, sequence) {
            reached.insert("\(harnessRowName(site)) \(hostOutcome(classes[index]))")
        }
        taken.formUnion(run.rows)
        codes.formUnion(run.codes)
        runs += 1
    }
    var expected = 1
    for _ in 0..<bound {
        expected *= classes.count
    }
    #expect(runs == expected, "every sequence was run")

    let toldNoRow = taken.subtracting(rows).map(harnessRowName).sorted()
    #expect(toldNoRow.isEmpty, "the observer was told pairs that are no row of the table: \(toldNoRow)")
    let neverTaken = rows.subtracting(taken).map(harnessRowName).sorted()
    #expect(neverTaken.isEmpty, "every row of the harness's transition table is taken; never taken: \(neverTaken)")

    let deferred = Set(deferredEntries.map { textMember($0, "code") })
    var expectedCodes = Set(harnessApplicationCodes).union(deferred)
    for cls in classes {
        if case .text(let code)? = cls.member("expect")?.member("why")?.member("code") {
            expectedCodes.insert(code)
        }
    }
    #expect(expectedCodes.count == 15, "the fourteen lift codes and the application's own: \(expectedCodes.sorted())")
    let neverReached = expectedCodes.subtracting(codes).subtracting(deferred).sorted()
    #expect(neverReached.isEmpty, "every refusal code is reached; never reached: \(neverReached)")
    let unknown = codes.subtracting(expectedCodes).sorted()
    #expect(unknown.isEmpty, "the harness kept codes no class reaches: \(unknown)")

    var notAtEverySite: [String] = []
    for site in sites {
        for outcome in outcomes where !reached.contains("\(harnessRowName(site)) \(outcome)") {
            notAtEverySite.append("\(harnessRowName(site)) \(outcome)")
        }
    }
    notAtEverySite.sort()
    #expect(
        notAtEverySite.isEmpty,
        "every host outcome is reached at every call site; never reached: \(notAtEverySite)"
    )
}

@Test func theReferenceHostJudgesAGenerationBeforeItsProvider() {
    let host = ReferenceHost(clock: ScriptedClockSource(), transport: ScriptedTransport())
    host.setAnswerSchemas([AnswerSchema(id: "urn:sce:schema:answer:1", jsonSchema: "{}", gbnf: "root ::= \"{}\"")])
    let scripted = ScriptedProvider()
    scripted.constrained = true
    scripted.context = 10
    scripted.answers = [
        ScriptedAnswer(text: String(repeating: "a", count: answerMaxBytes + 1)), ScriptedAnswer(text: "kept"),
    ]
    host.setModelProvider(scripted)
    func ask(_ prompt: String, _ schema: String = "", _ maxTokens: String = "8", _ temperature: String = "0")
        -> HostOutcome?
    {
        host.start(
            operation: modelGenerate,
            id: "gen",
            params: [
                "prompt": prompt, "answer_schema": schema, "max_tokens": maxTokens, "temperature_milli": temperature,
            ]
        )
    }
    func done(_ outcome: HostOutcome?) -> String {
        if case .done(let data)? = outcome {
            return data
        }
        return ""
    }
    for refused in [ask(""), ask("p", "", "0"), ask("p", "", "8", "2001"), ask("p", "urn:sce:schema:other:1")] {
        #expect(refused == .refuse(#"{"reason":"invalid_argument"}"#))
    }
    #expect(done(ask("p")).contains(#""error":"answer_too_large""#))
    #expect(done(ask("abc")).contains(#""error":"context_exceeded""#))
    let kept = done(ask("abc", "urn:sce:schema:answer:1", "7", "2000"))
    #expect(kept.contains(#""text":"kept""#) && kept.contains(#""constrained":true"#))
    #expect(done(ask("p")).contains(#""error":"unavailable""#))
    host.setModelProvider(ExhaustiveProvider([.answer(text: "t", finish: "length"), .failure("busy"), .refusal, .hold]))
    #expect(done(ask("p")).contains(#""model":"exhaustive""#))
    #expect(done(ask("p")).contains(#""error":"busy""#))
    #expect(ask("p") == .refuse(#"{"reason":"invalid_argument"}"#))
    #expect(ask("p") == .hold)
    #expect(done(ask("p")).contains(#""error":"unavailable""#))
}
