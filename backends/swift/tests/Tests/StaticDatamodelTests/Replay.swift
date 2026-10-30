// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import Testing

// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated Swift:
// a variable is a property of the policy, every expression was lowered to Swift at
// build time, and no script engine is built.
//
// The scenarios replayed here are the ones Kotlin, Rust, C++, Go and the Interpreter
// replay — `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`,
// whose expected values are derived from the document, not observed from a backend —
// so the engines are held to one answer. A scenario is a list of steps, each an
// external event with its payload and what the machine must hold after it runs to
// quiescence: its current state and any of its variables.
//
// A member of a step this driver does not replay yet is a failure of the step, never
// a pass: a step that judged nothing would be one.

/// A machine as a scenario sees it: what it is sent and what it is asked.
struct Machine {
    /// Sends the event `name` with its JSON `data` (empty when it has none) and runs
    /// what that made due to quiescence.
    var send: (String, String) -> Void

    /// Whether an event arriving under `name` reaches the machine at all, as the
    /// engine decides it (§scxml-3.12.1) — false is a name it drops.
    var resolves: (String) -> Bool

    /// Moves the machine's time on by `ms` and runs what that made due: the machine
    /// runs on a manual clock, so a wait is the one a step names.
    var advance: (Int64) -> Void

    /// The id the document gives the current state.
    var state: () -> String

    /// Every active state, a compound or a parallel one with the atomic ones below
    /// it, by the id the document gives it.
    var configuration: () -> [String]

    /// Whether the machine ended in a top-level `<final>`.
    var ended: () -> Bool

    /// The data the top-level `<final>`'s `<donedata>` handed its done event, as the text
    /// the engine stashed (§scxml-5.5): JSON for a pair or a value, empty when the final
    /// carries none.
    var donedata: () -> String

    /// The variables a scenario may state, each as the JSON a scenario writes it.
    var variables: [String: () -> JSONValue]

    /// What the machine asked of its host since it was last asked, oldest first, each as
    /// a scenario's `host_calls` writes it: the name the document gives the action and its
    /// arguments in the order the document gives them, an absent one as `null`. Asking
    /// forgets what was asked. Nil for a machine whose host is not one that records.
    var hostCalls: (() -> [JSONValue])? = nil

    /// The child hosts the machine's host was asked for since it was last asked, oldest asked
    /// first, and what each child asked of the host it was given, each as a scenario's
    /// `child_hosts` writes it (`ChildHostBook.take`). Asking forgets what was asked. Nil for a
    /// machine whose host gives no child one.
    var childHosts: (() -> [JSONValue])? = nil
}

/// The child hosts a machine's host was asked for, in the order asked, and what each child
/// asked of its host. A child owns the host it was given, so each host is made by the test
/// and opened here, and what it was asked to do is read through `calls`, which answers the
/// calls made since it was last asked and forgets them.
final class ChildHostBook {
    private final class Booked {
        let invoke: String
        let start: Int
        let candidate: String
        let calls: () -> [JSONValue]
        var announced = false

        init(invoke: String, start: Int, candidate: String, calls: @escaping () -> [JSONValue]) {
            self.invoke = invoke
            self.start = start
            self.candidate = candidate
            self.calls = calls
        }
    }

    private var children: [Booked] = []

    /// The machine's host being asked for the host of a child its `invoke` started:
    /// `candidate` is the stem of the document when the invoke names one of several, and
    /// empty for a child written inline.
    func open(_ invoke: String, candidate: String = "", calls: @escaping () -> [JSONValue]) {
        let start = children.filter { $0.invoke == invoke }.count + 1
        children.append(Booked(invoke: invoke, start: start, candidate: candidate, calls: calls))
    }

    /// The children asked for or called since it was last called, oldest asked first, each as
    /// a scenario's `child_hosts` writes it: the invoke, the start of that invoke, the
    /// candidate's document stem where the invoke names one, and the calls made in this step
    /// alone. A child asked for earlier that called nothing since has no entry.
    func take() -> [JSONValue] {
        var fresh: [JSONValue] = []
        for child in children {
            let calls = child.calls()
            if child.announced && calls.isEmpty {
                continue
            }
            child.announced = true
            var members: [(key: String, value: JSONValue)] = [
                (key: "invoke", value: .text(child.invoke)),
                (key: "start", value: .number(String(child.start))),
            ]
            if !child.candidate.isEmpty {
                members.append((key: "candidate", value: .text(child.candidate)))
            }
            members.append((key: "host_calls", value: .array(calls)))
            fresh.append(.object(members))
        }
        return fresh
    }
}

/// One call of a host as a scenario's `host_calls` writes it.
func hostCall(_ action: String, _ args: [JSONValue] = []) -> JSONValue {
    .object([(key: "action", value: .text(action)), (key: "args", value: .array(args))])
}

/// How many rounds a host gives a machine that runs child sessions after an event
/// before it reads the machine back. A child takes an event its parent forwarded in
/// one round and its end reaches the parent in the next, which is two for each level
/// of child; a round that finds nothing to do changes nothing. The count the other
/// backends' suites give a machine.
private let settleTicks = 5

/// Starts `policy` on a manual clock and returns it as a machine. `hostCalls` is how a
/// machine whose host records reads what it was asked ([`Machine.hostCalls`]).
func drive<Policy: StatePolicy>(
    _ policy: Policy,
    variables: [String: () -> JSONValue],
    hostCalls: (() -> [JSONValue])? = nil,
    childHosts: (() -> [JSONValue])? = nil
) -> Machine {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    // `step` drains the queues and nothing else: a child session is run by the
    // parent's `tick`, so a machine that has one is driven by rounds of it.
    let settle = {
        guard policy.hasChildTick else {
            engine.step()
            return
        }
        for _ in 0..<settleTicks {
            engine.tick()
        }
    }
    // A machine that starts a child in its first state has that child running, and
    // maybe ended, before any event arrives.
    if policy.hasChildTick {
        settle()
    }
    return Machine(
        send: { name, data in
            engine.raiseExternalByName(name, eventData: data)
            settle()
        },
        resolves: { engine.resolveEventByName($0) != nil },
        advance: { engine.advanceTimeMs($0) },
        state: { policy.getStateName(engine.getCurrentState()) },
        configuration: { engine.getActiveStates().map { policy.getStateName($0) } },
        ended: { engine.isInFinalState },
        donedata: { engine.donedataAtFinal },
        variables: variables,
        hostCalls: hostCalls,
        childHosts: childHosts
    )
}

// MARK: - Values as a scenario writes them

/// An integer variable as a scenario writes it: its digits.
func integer<T: BinaryInteger>(_ value: T) -> JSONValue {
    .number(String(value))
}

/// A list of integers as a scenario writes it: its elements, in order.
func integers<T: BinaryInteger>(_ values: [T]) -> JSONValue {
    .array(values.map { integer($0) })
}

/// An integer variable that may be absent as a scenario writes it: its digits when it is
/// present, `null` when it is not. A zero is a value, not an absence.
func optionalInteger<T: BinaryInteger>(_ value: T?) -> JSONValue {
    guard let value else {
        return .null
    }
    return integer(value)
}

/// A 64-bit integer that may be absent, as a scenario writes one held in a record: the
/// text of its digits, exactly, or `null`.
func optionalDigits(_ value: Int64?) -> JSONValue {
    guard let value else {
        return .null
    }
    return .text(String(value))
}

/// A text that may be absent as a scenario writes it: the text or `null`.
func optionalText(_ value: String?) -> JSONValue {
    guard let value else {
        return .null
    }
    return .text(value)
}

/// A real variable as a scenario writes it. A single widens to the double it is,
/// exactly, wherever it crosses a boundary (SCE_FORGE.md §3.4.1).
func real<T: BinaryFloatingPoint>(_ value: T) -> JSONValue {
    .number("\(Double(value))")
}

/// A list of reals as a scenario writes it: its elements, in order.
func reals<T: BinaryFloatingPoint>(_ values: [T]) -> JSONValue {
    .array(values.map { real($0) })
}

/// Whether two JSON values are one value. An integer is the same digits, which a
/// `Double` would lose past 2^53; a real is the same number however it is spelled
/// (`5.0` and `5`).
func sameValue(_ got: JSONValue, _ want: JSONValue) -> Bool {
    switch (got, want) {
    case (.number(let a), .number(let b)):
        return sameNumber(a, b)
    case (.array(let a), .array(let b)):
        return a.count == b.count && zip(a, b).allSatisfy { sameValue($0, $1) }
    case (.object(let a), .object(let b)):
        return a.count == b.count
            && zip(a, b).allSatisfy {
                $0.key.utf8.elementsEqual($1.key.utf8) && sameValue($0.value, $1.value)
            }
    default:
        return got == want
    }
}

private func sameNumber(_ a: String, _ b: String) -> Bool {
    let isInteger = { (text: String) in !text.contains { ".eE".contains($0) } }
    if isInteger(a) && isInteger(b) {
        return a.utf8.elementsEqual(b.utf8)
    }
    guard let x = Double(a), let y = Double(b) else {
        return false
    }
    return x == y
}

// MARK: - Replay

/// Where the shared scenarios are, from the repository root this file lies under.
private let scenarioDirectory: String = {
    // <root>/backends/swift/tests/Tests/StaticDatamodelTests/Replay.swift
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url.appendingPathComponent("sce-build/tests/fixtures/static_datamodel/scenarios").path
}()

/// The members of a step that a driver replays only by doing something this one does
/// not: each fails the step that carries it.
private let unreplayedStepMembers = ["restore", "invoke"]

/// The members of a step's `expect` this driver does not judge yet.
private let unreplayedExpectMembers = ["sent"]

/// Fails the test that is replaying, at the line of the test that asked for it.
private func record(_ message: String, _ at: SourceLocation) {
    _ = Issue.record(Comment(rawValue: message), sourceLocation: at)
}

/// Replays scenario `name` against `machine`. Every step names an event (or none,
/// for the machine as started) and what it must hold afterwards. A name no event of
/// the machine matches is a misspelt step unless the step says it expects the drop
/// (`"dropped": true`), which is then what is held.
func replay(_ name: String, _ machine: Machine, sourceLocation: SourceLocation = #_sourceLocation) {
    let path = "\(scenarioDirectory)/\(name).json"
    let scenario: JSONValue
    do {
        scenario = try parseJSON(String(contentsOfFile: path, encoding: .utf8))
    } catch {
        record("scenario \(name) is not readable: \(error)", sourceLocation)
        return
    }
    guard case .array(let steps)? = scenario.member("steps"), !steps.isEmpty else {
        record("scenario \(name) has no steps, which judges nothing", sourceLocation)
        return
    }
    if case .bool(false)? = scenario.member("router") {
        record(
            "scenario \(name) gives the machine no Mesh router, which this driver does not replay yet",
            sourceLocation
        )
        return
    }
    for (n, step) in steps.enumerated() {
        var note = ""
        if case .text(let text)? = step.member("note") {
            note = text
        }
        let where_ = "\(name) step \(n): \(note)"
        let fail = { (message: String) in
            record("\(where_): \(message)", sourceLocation)
        }
        for member in unreplayedStepMembers where step.member(member) != nil {
            fail("this driver does not replay `\(member)` yet")
        }
        if case .number(let ms)? = step.member("advance_ms"), let by = Int64(ms) {
            machine.advance(by)
        }
        if case .text(let event)? = step.member("event") {
            var data = ""
            if let payload = step.member("data") {
                data = writeJSON(payload)
            }
            let dropped: Bool
            if case .bool(true)? = step.member("dropped") {
                dropped = true
            } else {
                dropped = false
            }
            if machine.resolves(event) == dropped {
                fail("the machine's events \(dropped ? "match" : "do not match") `\(event)`")
            }
            machine.send(event, data)
        }
        // What the machine asked of its host in this step. Read on every step, so that
        // a step which states nothing does not hand its calls to the next, and a step
        // that states some needs a driver that records them.
        let asked = machine.hostCalls?()
        // The child hosts asked for in this step, and what each child asked of its host,
        // read and held for the same reason.
        let children = machine.childHosts?()
        guard let expect = step.member("expect") else {
            continue
        }
        for member in unreplayedExpectMembers where expect.member(member) != nil {
            fail("this driver does not judge `\(member)` yet")
        }
        if let want = expect.member("host_calls") {
            if let asked {
                if !sameValue(.array(asked), want) {
                    fail("the machine asked its host \(writeJSON(.array(asked))), not \(writeJSON(want))")
                }
            } else {
                fail("this driver records no host calls, so it cannot hold a step to them")
            }
        }
        if let want = expect.member("child_hosts") {
            if let children {
                if !sameValue(.array(children), want) {
                    fail("the child hosts were \(writeJSON(.array(children))), not \(writeJSON(want))")
                }
            } else {
                fail("this driver records no child hosts, so it cannot hold a step to them")
            }
        }
        if case .bool(true)? = expect.member("ended") {
            if !machine.ended() {
                fail("the machine ended in a top-level <final>")
            }
            // The done event's data is JSON for a pair or a value, which a scenario
            // writes as the value it is; text that is not JSON is the data as written,
            // and the empty text is a final that hands none.
            if let want = expect.member("donedata") {
                let text = machine.donedata()
                let got = (try? parseJSON(text)) ?? .text(text)
                if !sameValue(got, want) {
                    fail("the done event's data is \(text), not \(writeJSON(want))")
                }
            }
            continue
        }
        if case .text(let want)? = expect.member("state") {
            let got = machine.state()
            if !got.utf8.elementsEqual(want.utf8) {
                fail("the current state is \(got), not \(want)")
            }
        }
        // The configuration is a set: the order the machine lists its active states
        // in is not part of the answer.
        if case .array(let wanted)? = expect.member("configuration") {
            let want = wanted.compactMap { value -> String? in
                if case .text(let id) = value { return id }
                return nil
            }
            if machine.configuration().sorted() != want.sorted() {
                fail("the active states are \(machine.configuration().sorted()), not \(want.sorted())")
            }
        }
        if case .object(let wanted)? = expect.member("variables") {
            for (variable, want) in wanted {
                guard let read = machine.variables[variable] else {
                    fail("the driver publishes no variable \(variable)")
                    continue
                }
                let got = read()
                if !sameValue(got, want) {
                    fail("variable \(variable) is \(writeJSON(got)), not \(writeJSON(want))")
                }
            }
        }
    }
}
