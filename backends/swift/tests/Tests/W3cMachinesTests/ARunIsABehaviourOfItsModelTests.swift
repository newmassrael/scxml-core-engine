// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Written by hand, beside `W3cSupport.swift`: a run of the Swift lane, recorded as the trace
// `sce-codegen formal traces` judges a behaviour of the document's model (docs/adr/0115, D4 and D8).
// The twin of the Rust lane's `a_run_is_a_behaviour_of_its_model` and of the C11 lane's
// `c11_a_run_of_test_*_is_the_recorded_run_of_its_model`.
//
// Each test runs one W3C document that `docs/formal` has a model of (`W3CTest388`, `W3CTest570`,
// `W3CTest403c` over `AppendixD.tla`), as `generate-w3c -l swift` writes its machine, with the
// engine's trace port set (the `TracePort` trait the package asks of the runtime), and holds what the
// port was told to the recorded run `docs/formal/traces.json` declares of that model, byte for byte.
// The judge reads the recorded file, so a run that moves is seen here first, and a recorded run that
// no longer is the lane's is never judged in its place. A Swift run of a document is judged against
// the same generated model as the Rust run of it, and these documents leave a run's steps no choice,
// so the two recorded runs differ only in their lane, or the judge refuses one.
//
// One test per recorded run of this lane: the `w3c-swift` gate counts the files
// `docs/formal/traces/*.swift.json` beside the fixtures the generator wrote.
//
// Recording again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 swift test --filter W3cMachinesTests.ARunIsABehaviourOfItsModel
// in backends/swift/tests, once `scripts/regen_swift_w3c.sh` has written the machines; then
// `sce-codegen formal traces --manifest docs/formal/traces.json`, and read the diff of the recorded
// runs.

import Foundation
import SceRuntime
import Testing
import W3cMachines

/// The recorded run of `model`'s document by this lane, as the traces manifest names it:
/// `<root>/docs/formal/traces/<model>.swift.json`, the root found from this file's own path,
/// `<root>/backends/swift/tests/Tests/W3cMachinesTests/<this file>`.
private func recordedRun(_ model: String, file: String = #filePath) -> URL {
    var url = URL(fileURLWithPath: file)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url.appendingPathComponent("docs/formal/traces/\(model).\(traceLane).json")
}

/// The trace port's account of one run of `policy`'s machine to the end, and the id of the
/// top-level final state the run ended in. The run is the one `w3cSettled` gives every fixture of the
/// suite, with the port set before the start, so the start is told.
private func tracedRun<Policy: StatePolicy>(_ policy: Policy) throws -> (trace: RunTrace, ended: String?) {
    var trace = RunTrace()
    let engine = Engine(policy: policy)
    engine.setTracePort { trace.push($0) }
    try engine.initialize()
    let completed = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))
    #expect(completed, "the run reached no final state")
    return (trace, engine.terminalState.map { policy.getStateName($0) })
}

/// The run starts, ends in its pass state, and the interpreter's exit leaves that state alone: the
/// first and the last step a recorded run must hold.
private func startsAndStopsInPass(_ trace: RunTrace, ended: String?) {
    #expect(ended == "pass", "the traced run ended in \(String(describing: ended)), not in `pass`")
    #expect(trace.steps.first?.kind == .start, "the run does not begin with a start step")
    #expect(trace.steps.last?.kind == .stop, "the run does not end with a stop step")
    #expect(trace.steps.last?.outcome == "pass", "the stop names no outcome `pass`")
    #expect(trace.steps.last?.exited == ["pass"], "the stop does not exit `pass` alone")
}

/// Holds `trace` to the recorded run of `model`, or records it again when `UPDATE_EXPECT` is set.
private func holdsToTheRecordedRun(_ model: String, _ trace: RunTrace) throws {
    let text = trace.toJSON()
    let url = recordedRun(model)
    if ProcessInfo.processInfo.environment["UPDATE_EXPECT"] != nil {
        try Data(text.utf8).write(to: url)
        return
    }
    guard let recorded = try? Data(contentsOf: url) else {
        Issue.record(
            "\(url.path) cannot be read: record it with `UPDATE_EXPECT=1 swift test --filter W3cMachinesTests.ARunIsABehaviourOfItsModel`"
        )
        return
    }
    #expect(
        Array(text.utf8) == Array(recorded),
        "the run of \(model) is not the recorded one that `sce-codegen formal traces` judges. The run:\n\(text)"
    )
}

@Suite struct ARunIsABehaviourOfItsModel {
    /// The document re-enters a state through a deep and a shallow history, which restore the
    /// configuration the state last left.
    @Test func aRunOfTest388IsTheRecordedRunOfItsModel() throws {
        let run = try tracedRun(Test388Policy())
        startsAndStopsInPass(run.trace, ended: run.ended)
        try holdsToTheRecordedRun("W3CTest388", run.trace)
    }

    /// The regions of a parallel state each reach their final state, and the done events of the
    /// regions come before the done event of the state.
    @Test func aRunOfTest570IsTheRecordedRunOfItsModel() throws {
        let run = try tracedRun(Test570Policy())
        startsAndStopsInPass(run.trace, ended: run.ended)
        try holdsToTheRecordedRun("W3CTest570", run.trace)
    }

    /// Transitions of a parallel state's regions that conflict are removed from the set a microstep
    /// takes, the one of the descendant kept.
    @Test func aRunOfTest403cIsTheRecordedRunOfItsModel() throws {
        let run = try tracedRun(Test403cPolicy())
        startsAndStopsInPass(run.trace, ended: run.ended)
        try holdsToTheRecordedRun("W3CTest403c", run.trace)
    }
}
