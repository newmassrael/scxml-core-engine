// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// The trace port's steps, their writer and the recorder the engine fills (docs/adr/0115, D4 and D8),
// held to the cases of the Rust runtime's `trace_port` module and of the C11 runtime's
// `c11_unit_trace_port`, so the three lanes write one run in one layout.

private func step(_ kind: TraceStepKind) -> TraceStep {
    TraceStep(
        kind: kind,
        event: "go.on",
        fired: [TraceFired(source: "s0", index: 2)],
        exited: ["s01", "s0"],
        entered: ["s1"],
        configuration: ["s1"],
        outcome: "pass"
    )
}

@Test func eachKindWritesTheFieldsTheFormatAsksOfIt() {
    #expect(step(.start).toJSON() == #"{"kind": "start", "entered": ["s1"], "configuration": ["s1"]}"#)
    #expect(
        step(.eventless).toJSON()
            == #"{"kind": "eventless", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}"#
    )
    #expect(
        step(.internal).toJSON()
            == #"{"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}"#
    )
    #expect(
        step(.external).toJSON()
            == #"{"kind": "external", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}"#
    )
    #expect(step(.stop).toJSON() == #"{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}"#)
    var unended = step(.stop)
    unended.outcome = nil
    #expect(unended.toJSON() == #"{"kind": "stop", "exited": ["s01", "s0"], "outcome": null}"#)
}

@Test func aTraceIsWrittenWithItsVersionAndLaneOneStepToALine() {
    var trace = RunTrace()
    #expect(trace.toJSON() == "{\n  \"version\": 1,\n  \"lane\": \"swift\",\n  \"steps\": []\n}\n")
    trace.push(step(.start))
    trace.push(step(.stop))
    #expect(
        trace.toJSON()
            == "{\n  \"version\": 1,\n  \"lane\": \"swift\",\n  \"steps\": [\n    "
            + #"{"kind": "start", "entered": ["s1"], "configuration": ["s1"]}"#
            + ",\n    "
            + #"{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}"#
            + "\n  ]\n}\n"
    )
    #expect(trace.steps.count == 2)
}

@Test func aNameIsWrittenAsAJsonString() {
    let quoted = TraceStep(kind: .start, entered: [#"a"b\c"#])
    #expect(quoted.toJSON() == #"{"kind": "start", "entered": ["a\"b\\c"], "configuration": []}"#)
}

/// What a recorder hands back once a step closes, told to its port as the engine tells it.
private func closeAndTell(_ recorder: inout TraceRecorder, configuration: [String], outcome: String?) {
    if let told = recorder.close(configuration: configuration, outcome: outcome) {
        told.port(told.step)
    }
}

/// A recorder tells its port each step once the step ends, with what was added between, and the
/// configuration only of a step that leaves one.
@Test func aRecorderTellsItsPortEachStepOnceItEnds() {
    var told: [TraceStep] = []
    var recorder = TraceRecorder()
    recorder.set { told.append($0) }

    recorder.begin(.internal, event: "e")
    #expect(recorder.isOpen)
    recorder.fired(source: "s0", index: 1)
    recorder.exited("s0")
    recorder.entered("s1")
    #expect(told.isEmpty, "told only once it ends")
    closeAndTell(&recorder, configuration: ["s1"], outcome: "ignored")
    #expect(!recorder.isOpen)

    recorder.begin(.stop, event: nil)
    recorder.exited("s1")
    closeAndTell(&recorder, configuration: ["ignored"], outcome: "pass")

    #expect(
        told == [
            TraceStep(
                kind: .internal,
                event: "e",
                fired: [TraceFired(source: "s0", index: 1)],
                exited: ["s0"],
                entered: ["s1"],
                configuration: ["s1"]
            ),
            TraceStep(kind: .stop, exited: ["s1"], outcome: "pass"),
        ]
    )
}

/// With no port set nothing is opened, no name is asked for, and what the engine adds is kept
/// nowhere; a cleared port is told nothing more.
@Test func aRecorderWithNoPortKeepsNothing() {
    var asked = 0
    func named(_ name: String) -> String {
        asked += 1
        return name
    }
    var recorder = TraceRecorder()
    recorder.begin(.start, event: named("e"))
    #expect(!recorder.isOpen)
    recorder.entered(named("s0"))
    recorder.fired(source: named("s0"), index: 0)
    #expect(recorder.close(configuration: ["s0"], outcome: nil).map { $0.step } == nil)
    #expect(asked == 0, "a recorder with no port asked for \(asked) name(s)")

    var told = 0
    recorder.set { _ in told += 1 }
    recorder.begin(.start, event: nil)
    recorder.clear()
    #expect(!recorder.isOpen)
    closeAndTell(&recorder, configuration: ["s0"], outcome: nil)
    #expect(told == 0)
}
