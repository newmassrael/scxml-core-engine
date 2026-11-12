// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1 + 5.10: a computed event name is matched like any other. Swift AOT path.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

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
}
