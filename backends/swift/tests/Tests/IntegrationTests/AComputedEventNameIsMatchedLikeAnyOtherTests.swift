// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1 + 5.10: a computed event name is matched like any other. Swift AOT path.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/materialize.sh swift-integration

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AComputedEventNameIsMatchedLikeAnyOther {
    @Test func aComputedNameIsMatchedByTokenAndToldWhole() throws {
        let engine = try initializedOnVirtualTime(AComputedEventNameIsMatchedLikeAnyOtherPolicy())
        _ = driveOnVirtualTime(engine)

        #expect(
            engine.terminalState == .pass,
            "a computed name was dropped, or matched but told shorter than the name it was sent under (`request.new`, `other.thing`, `request.again`, `last.one`); ended in \(String(describing: engine.terminalState))"
        )
    }

    /// The document declares its transition table, and a run that reaches `pass` takes each of
    /// its six transitions once, in document order: the transition observer is told exactly
    /// those rows, each by the `(from, index)` its row of the table carries (docs/adr/0089).
    @Test func theObserverIsToldEachRowOfTheTableTheRunTakes() throws {
        var seen: [TransitionTaken<AComputedEventNameIsMatchedLikeAnyOtherState>] = []
        let engine = Engine(policy: AComputedEventNameIsMatchedLikeAnyOtherPolicy())
        engine.setClock(ManualClock())
        engine.setTransitionObserver { seen.append($0) }
        try engine.initialize()
        _ = driveOnVirtualTime(engine)

        #expect(engine.terminalState == .pass)
        let taken: [AComputedEventNameIsMatchedLikeAnyOtherState] = [.s1, .s2, .s3, .s4, .s5, .s6]
        let expected = taken.map { TransitionTaken(source: $0, index: 0) }
        #expect(seen == expected, "the rows taken, in the order the run took them")
        let rows = Set(AComputedEventNameIsMatchedLikeAnyOtherTransition.all.map {
            TransitionTaken(source: $0.from, index: $0.index)
        })
        #expect(rows.isSuperset(of: seen), "every pair the observer is told is a row of the table: \(seen)")
        #expect(rows == Set(seen), "the run takes every row of the table: \(rows)")
    }

    /// An observer set and then cleared is told nothing, and the run still reaches `pass`, as
    /// one that never had an observer does.
    @Test func aClearedObserverIsToldNothingAndTheRunIsTheSame() throws {
        var seen: [TransitionTaken<AComputedEventNameIsMatchedLikeAnyOtherState>] = []
        let engine = Engine(policy: AComputedEventNameIsMatchedLikeAnyOtherPolicy())
        engine.setClock(ManualClock())
        engine.setTransitionObserver { seen.append($0) }
        engine.setTransitionObserver(nil)
        try engine.initialize()
        _ = driveOnVirtualTime(engine)

        #expect(engine.terminalState == .pass)
        #expect(seen.isEmpty, "a cleared observer was told \(seen)")
    }

    /// An event step of the trace port names its event by the name `_event.name` holds while it is
    /// taken, which the run trace format defines the field as (docs/adr/0115, D4): the whole name it
    /// was sent under, and not the event of the document a door resolved it to (`request`, or the
    /// wildcard). The document sends six events whose names it computes, three to its external queue
    /// and three to its internal one, as the Rust lane's
    /// `an_event_step_names_the_event_by_the_name_it_was_sent_under` holds of the same document.
    @Test func anEventStepNamesTheEventByTheNameItWasSentUnder() throws {
        var trace = RunTrace()
        let engine = Engine(policy: AComputedEventNameIsMatchedLikeAnyOtherPolicy())
        engine.setClock(ManualClock())
        engine.setTracePort { trace.push($0) }
        try engine.initialize()
        _ = driveOnVirtualTime(engine)

        #expect(engine.terminalState == .pass)
        #expect(trace.steps.first?.kind == .start, "the run begins with its start")
        #expect(trace.steps.last?.kind == .stop, "the run ends with the exit of the interpreter")
        #expect(trace.steps.last?.exited == ["pass"])
        #expect(trace.steps.last?.outcome == "pass")
        let events = trace.steps
            .filter { $0.kind == .internal || $0.kind == .external }
            .map { "\($0.kind.name) \($0.event ?? "")" }
        #expect(
            events == [
                "external request.new",
                "external other.thing",
                "internal request.again",
                "internal last.one",
                "internal request.third",
                "external last.two",
            ],
            "each event step names the name its event was sent under: \(events)"
        )
    }

    /// A trace port set and then cleared is told nothing, and the run still reaches `pass`, as one
    /// that never had a port does (docs/adr/0115, D4).
    @Test func aClearedTracePortIsToldNothingAndTheRunIsTheSame() throws {
        var told: [TraceStep] = []
        let engine = Engine(policy: AComputedEventNameIsMatchedLikeAnyOtherPolicy())
        engine.setClock(ManualClock())
        engine.setTracePort { told.append($0) }
        engine.clearTracePort()
        try engine.initialize()
        _ = driveOnVirtualTime(engine)

        #expect(engine.terminalState == .pass)
        #expect(told.isEmpty, "a cleared trace port was told \(told)")
    }
}
