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

    /// The variables a scenario may state, each as the JSON a scenario writes it.
    var variables: [String: () -> JSONValue]
}

/// Starts `policy` on a manual clock and returns it as a machine.
func drive<Policy: StatePolicy>(
    _ policy: Policy,
    variables: [String: () -> JSONValue]
) -> Machine {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    return Machine(
        send: { name, data in
            engine.raiseExternalByName(name, eventData: data)
            engine.step()
        },
        resolves: { engine.resolveEventByName($0) != nil },
        advance: { engine.advanceTimeMs($0) },
        state: { policy.getStateName(engine.getCurrentState()) },
        configuration: { engine.getActiveStates().map { policy.getStateName($0) } },
        ended: { engine.isInFinalState },
        variables: variables
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

/// A real variable as a scenario writes it. A single widens to the double it is,
/// exactly, wherever it crosses a boundary (SCE_FORGE.md §3.4.1).
func real(_ value: Double) -> JSONValue {
    .number("\(value)")
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
private let unreplayedExpectMembers = ["donedata", "host_calls", "child_hosts", "sent"]

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
        guard let expect = step.member("expect") else {
            continue
        }
        for member in unreplayedExpectMembers where expect.member(member) != nil {
            fail("this driver does not judge `\(member)` yet")
        }
        if case .bool(true)? = expect.member("ended") {
            if !machine.ended() {
                fail("the machine ended in a top-level <final>")
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
