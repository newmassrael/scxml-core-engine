// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Written by hand: each scenario of the properties judge's turnstile, replayed by the Swift lane
// (docs/adr/0115, D15 and the converse of D8). A scenario is the run TLC gave of a violation or a
// witness, with the events its environment put on the external queue; this test gives the machine
// those events, on a manual clock, with its trace port on (the `TracePort` trait the package asks of
// the runtime), and holds the run the port is told to the recorded run `<name>.swift.json` beside the
// scenario, byte for byte. `sce_build::formal::replay` judges each recorded run to be the scenario's,
// step for step (sce-build/tests/a_scenario_is_the_run_of_every_lane.rs), as it judges the Rust and
// the C11 lanes'. The twin of the Rust lane's `a_scenario_of_the_turnstile_is_its_run` and of the C11
// lane's `c11_a_scenario_of_the_turnstile_is_its_run`.
//
// The harness gives an input once the machine has taken every step it can, which is after the step
// the input names unless the model's environment put it on the queue in the middle of a macrostep. A
// macrostep that also sends to its own external queue after that point would take the two in another
// order here; the judge then finds the runs part, which is a defect of this harness to settle, and
// not a verdict about the document. A delayed send comes due by the clock moving to the next
// deadline, which is the earliest: a scenario in which the model's time makes a later one due first
// is one no clock replays, and the judge finds that too.
//
// The machine is the formal fixture's (the swift-ecmascript tree); the scenarios are written by
// `sce-codegen formal properties --scenarios sce-build/tests/fixtures/formal_properties/scenarios`.
// Recording the lane's runs again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 swift test --filter AScenarioOfTheTurnstileIsItsRun
// in backends/swift/tests.
//
// Fixture: sce-build/tests/fixtures/formal_properties/turnstile.scxml

import EcmaScriptMachines
import Foundation
import SceRuntime
import Testing

/// What a scenario's file is named by after its name (`sce_build::formal::replay::SCENARIO_SUFFIX`).
private let scenarioSuffix = ".scenario.json"

/// The directory of the turnstile's scenarios and of each lane's run of them:
/// `<root>/sce-build/tests/fixtures/formal_properties/scenarios`, the root found from this file's own
/// path, `<root>/backends/swift/tests/Tests/EcmaScriptMachinesTests/<this file>`.
private func scenariosDirectory(file: String = #filePath) -> URL {
    var url = URL(fileURLWithPath: file)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url.appendingPathComponent("sce-build/tests/fixtures/formal_properties/scenarios")
}

/// One event the environment put on the external queue during a scenario, `after` the count of
/// steps of the run that were over when it came, from 1.
private struct ScenarioInput {
    enum Kind {
        /// It came from outside.
        case arrive
        /// A delayed send the machine made came due.
        case due
    }

    let kind: Kind
    let after: Int
    let event: String
}

/// Why a scenario is not replayed.
private struct ScenarioRefusal: Error, CustomStringConvertible {
    let description: String
}

/// The inputs of the scenario `data` holds, which `sce_build::run_trace` reads of reference: every
/// other member of the document is read there and stepped over here. An input with a member the form
/// does not have, or without one it has, is refused rather than replayed in part.
private func scenarioInputs(_ name: String, _ data: Data) throws -> [ScenarioInput] {
    guard let document = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw ScenarioRefusal(description: "\(name): the scenario is no JSON object")
    }
    guard let listed = document["inputs"] else {
        return []
    }
    guard let elements = listed as? [Any] else {
        throw ScenarioRefusal(description: "\(name): `inputs` is no array")
    }
    var inputs: [ScenarioInput] = []
    for (at, element) in elements.enumerated() {
        guard let input = element as? [String: Any], Set(input.keys) == ["kind", "after", "event"],
            let kind = input["kind"] as? String, let after = input["after"] as? Int, after >= 1,
            let event = input["event"] as? String, !event.isEmpty
        else {
            throw ScenarioRefusal(
                description: "\(name): inputs[\(at)] is not an object of a `kind`, an `after` and an `event`")
        }
        switch kind {
        case "arrive":
            inputs.append(ScenarioInput(kind: .arrive, after: after, event: event))
        case "due":
            inputs.append(ScenarioInput(kind: .due, after: after, event: event))
        default:
            throw ScenarioRefusal(
                description: "\(name): inputs[\(at)] is of the kind '\(kind)', which the form does not have")
        }
    }
    return inputs
}

/// The run the trace port is told when the turnstile is given `inputs`.
private func replay(_ name: String, _ inputs: [ScenarioInput]) throws -> RunTrace {
    var trace = RunTrace()
    let engine = Engine(policy: TurnstilePolicy())
    // Before the start: the entry configuration's `<onentry>` may arm a delayed send.
    engine.setClock(ManualClock())
    engine.setTracePort { trace.push($0) }
    try engine.initialize()
    for input in inputs {
        let told = trace.steps.count
        guard told >= input.after else {
            throw ScenarioRefusal(
                description:
                    "\(name): the machine took \(told) steps, and the model \(input.after) before `\(input.event)` came"
            )
        }
        switch input.kind {
        case .arrive:
            engine.raiseExternalByName(input.event)
            engine.tick()
        case .due:
            guard let wait = engine.timeUntilNextScheduled() else {
                throw ScenarioRefusal(
                    description: "\(name): `\(input.event)` comes due, and the machine holds no delayed send")
            }
            let (seconds, attoseconds) = wait.components
            engine.advanceTimeMs(seconds * 1000 + (attoseconds + 999_999_999_999_999) / 1_000_000_000_000_000)
        }
    }
    return trace
}

/// Holds `trace` to the recorded run at `url`, or records it again when `UPDATE_EXPECT` is set.
private func holdsToTheRecordedRun(_ url: URL, _ trace: RunTrace) throws {
    let text = trace.toJSON()
    if ProcessInfo.processInfo.environment["UPDATE_EXPECT"] != nil {
        try Data(text.utf8).write(to: url)
        return
    }
    guard let recorded = try? Data(contentsOf: url) else {
        Issue.record(
            "\(url.path) cannot be read: record it with `UPDATE_EXPECT=1 swift test --filter AScenarioOfTheTurnstileIsItsRun` in backends/swift/tests"
        )
        return
    }
    #expect(
        Array(text.utf8) == Array(recorded),
        "the run of \(url.path) is not the recorded one the replay judge reads. The run:\n\(text)"
    )
}

@Suite struct AScenarioOfTheTurnstileIsItsRun {
    @Test func eachScenarioOfTheTurnstileIsItsRunOnTheSwiftLane() throws {
        let directory = scenariosDirectory()
        let files = (try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []
        let names = files
            .filter { $0.hasSuffix(scenarioSuffix) && $0.count > scenarioSuffix.count }
            .map { String($0.dropLast(scenarioSuffix.count)) }
            .sorted()
        try #require(
            !names.isEmpty,
            "\(directory.path) holds no scenario: write them with `sce-codegen formal properties --scenarios`"
        )
        for name in names {
            let data = try Data(contentsOf: directory.appendingPathComponent(name + scenarioSuffix))
            let trace = try replay(name, try scenarioInputs(name, data))
            try holdsToTheRecordedRun(directory.appendingPathComponent("\(name).\(traceLane).json"), trace)
        }
    }
}
