// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D: what a microstep selects, exits and enters, asked of
// Microstep.swift over a document written out by hand.
//
// Each case states the answer the appendix computes, worked from the pseudo-code,
// and asks the transcription for it. The cases, the document and the answers are
// those of backends/rust/runtime/tests/microstep_algorithms.rs,
// backends/go/runtime/microstep_test.go and
// backends/python/tests/microstep/test_microstep_algorithms.py, and through them
// of the C++ engines' tests/states/EntrySetAlgorithmsTest.cpp and
// tests/states/CompletionAlgorithmsTest.cpp, so the transcriptions answer one set
// of questions. Three answers are the reason the entry procedures exist and are
// pinned by name:
//
//   - a target SET enters every target, and a <parallel> region no target
//     descends into still gets its default;
//   - a compound state entered only as an ANCESTOR of a deeper target is not in
//     statesForDefaultEntry, so its initial transition content does not run;
//   - a history that recorded several regions restores all of them.
//
//   <scxml initial="s">
//     <state id="s" initial="p">
//       <parallel id="p">
//         <state id="r1" initial="a1"> a1 a2 </state>
//         <state id="r2" initial="b1"> b1 b2
//           <history id="hb" type="shallow"> -> b2 </history> </state>
//         <state id="r3"> c1 c2 </state>                    (no initial: c1)
//       </parallel>
//       <state id="q" initial="q1"> q1 <final id="qf"/> </state>
//       <history id="hs" type="deep"> -> "a2 b2" </history>
//       <state id="m" initial="m1a m2b">
//         <parallel id="mp">
//           <state id="m1"> m1a m1b </state>
//           <state id="m2"> m2a m2b </state>
//         </parallel>
//       </state>
//     </state>
//     <state id="t"/>
//   </scxml>

import Testing

@testable import SceRuntime

/// Declared in document order, so a value is the document position.
private enum St: Int, Hashable, CustomStringConvertible {
    case s, p, r1, a1, a2, r2, b1, b2, r3, c1, c2
    case q, q1, qf, m, mp, m1, m1a, m1b, m2, m2a, m2b, t

    var description: String {
        let names = [
            "S", "P", "R1", "A1", "A2", "R2", "B1", "B2", "R3", "C1", "C2",
            "Q", "Q1", "Qf", "M", "Mp", "M1", "M1a", "M1b", "M2", "M2a", "M2b", "T",
        ]
        return names[rawValue]
    }
}

private let hb = HistoryID(0)
private let hs = HistoryID(1)

private func historyName(_ history: HistoryID) -> String {
    history == hb ? "Hb" : "Hs"
}

private typealias Target = EntryTarget<St>

private func to(_ state: St) -> Target { .state(state) }

private func toHistory(_ history: HistoryID) -> Target { .history(history) }

/// The events the transition table below answers; `null` is the eventless
/// selection's "no event".
private enum Ev {
    case e, f, g, h, k, r, null
}

private struct Tr {
    var event: Ev
    var targets: [Target] = []
    var isInternal = false
}

/// The transitions, per source, in document order. Every one has content:
///
///   a1: E -> a2, F -> a2, H (targetless), K -> a2, R -> a1 (a self-transition)
///   b1: E -> b2, G -> b2, K -> b2, R -> b2
///   p:  F -> t,  G -> t,  H -> t,  K (targetless)
private let chartTransitions: [St: [Tr]] = [
    .a1: [
        Tr(event: .e, targets: [to(.a2)]),
        Tr(event: .f, targets: [to(.a2)]),
        Tr(event: .h),
        Tr(event: .k, targets: [to(.a2)]),
        Tr(event: .r, targets: [to(.a1)]),
    ],
    .b1: [
        Tr(event: .e, targets: [to(.b2)]),
        Tr(event: .g, targets: [to(.b2)]),
        Tr(event: .k, targets: [to(.b2)]),
        Tr(event: .r, targets: [to(.b2)]),
    ],
    .p: [
        Tr(event: .f, targets: [to(.t)]),
        Tr(event: .g, targets: [to(.t)]),
        Tr(event: .h, targets: [to(.t)]),
        Tr(event: .k),
    ],
]

private let chartParent: [St: St] = [
    .p: .s, .q: .s, .m: .s,
    .r1: .p, .r2: .p, .r3: .p,
    .a1: .r1, .a2: .r1,
    .b1: .r2, .b2: .r2,
    .c1: .r3, .c2: .r3,
    .q1: .q, .qf: .q,
    .mp: .m,
    .m1: .mp, .m2: .mp,
    .m1a: .m1, .m1b: .m1,
    .m2a: .m2, .m2b: .m2,
]

private let chartChildren: [St: [St]] = [
    .s: [.p, .q, .m],
    .p: [.r1, .r2, .r3],
    .r1: [.a1, .a2],
    .r2: [.b1, .b2],
    .r3: [.c1, .c2],
    .q: [.q1, .qf],
    .m: [.mp],
    .mp: [.m1, .m2],
    .m1: [.m1a, .m1b],
    .m2: [.m2a, .m2b],
]

private let chartInitial: [St: [Target]] = [
    .s: [to(.p)],
    .r1: [to(.a1)],
    .r2: [to(.b1)],
    .r3: [to(.c1)],
    .q: [to(.q1)],
    .m: [to(.m1a), to(.m2b)],
    .m1: [to(.m1a)],
    .m2: [to(.m2a)],
]

private let initialConfiguration: [St] = [.s, .p, .r1, .a1, .r2, .b1, .r3, .c1]

/// The document above, a configuration, and a record of what the microstep asked
/// the machine to do.
private final class Chart: Run {
    typealias State = St
    typealias Event = Ev

    var recorded: [HistoryID: [St]] = [:]
    var activeConfiguration: [St] = []
    var log: [String] = []
    var exitsSaw: [[St]] = []

    static func atInitialConfiguration() -> Chart {
        let chart = Chart()
        chart.activeConfiguration = initialConfiguration
        return chart
    }

    // Document

    func parentOf(_ state: St) -> St? { chartParent[state] }

    func isCompound(_ state: St) -> Bool {
        [.s, .r1, .r2, .r3, .q, .m, .m1, .m2].contains(state)
    }

    func isParallel(_ state: St) -> Bool { state == .p || state == .mp }

    func isFinal(_ state: St) -> Bool { state == .qf }

    func childStates(_ state: St) -> [St] { chartChildren[state] ?? [] }

    func initialTargets(_ state: St) -> [Target] { chartInitial[state] ?? [] }

    func historyParent(_ history: HistoryID) -> St { history == hb ? .r2 : .s }

    func historyValue(_ history: HistoryID) -> [St]? { recorded[history] }

    func historyDefaultTargets(_ history: HistoryID) -> [Target] {
        history == hb ? [to(.b2)] : [to(.a2), to(.b2)]
    }

    func documentOrder(_ state: St) -> Int { state.rawValue }

    // Run

    func configuration() -> [St] { activeConfiguration }

    func firstEnabledTransition(_ state: St, event: Ev) -> EnabledTransition<St>? {
        for (index, transition) in (chartTransitions[state] ?? []).enumerated() where transition.event == event {
            return EnabledTransition(
                source: state,
                targets: transition.targets,
                transitionIndex: index,
                hasActions: true,
                isInternal: transition.isInternal
            )
        }
        return nil
    }

    func exitState(_ state: St, configurationBeforeExit: [St]) {
        exitsSaw.append(configurationBeforeExit)
        activeConfiguration.removeAll { $0 == state }
        log.append("exit \(state)")
    }

    func executeTransitionContent(_ transition: EnabledTransition<St>) {
        log.append("content \(transition.source)#\(transition.transitionIndex)")
    }

    func enterState(_ state: St, isDefaultEntry: Bool) {
        activeConfiguration.append(state)
        log.append("enter \(state)\(isDefaultEntry ? " (default)" : "")")
    }

    func executeHistoryDefaultContent(_ history: HistoryID) {
        log.append("history default \(historyName(history))")
    }
}

private func entering(_ source: St, _ targets: Target...) -> EntryTransition<St> {
    EntryTransition(source: source, targets: targets)
}

private func enteringInternal(_ source: St, _ targets: Target...) -> EntryTransition<St> {
    EntryTransition(source: source, targets: targets, isInternal: true)
}

private func fromDocument(_ targets: Target...) -> EntryTransition<St> {
    EntryTransition(source: nil, targets: targets)
}

private func enabled(_ source: St, _ index: Int) -> EnabledTransition<St> {
    let transition = chartTransitions[source]![index]
    return EnabledTransition(
        source: source,
        targets: transition.targets,
        transitionIndex: index,
        hasActions: true,
        isInternal: transition.isInternal
    )
}

private struct Selected: Equatable {
    var source: St
    var index: Int
}

private func sources(_ transitions: [EnabledTransition<St>]) -> [Selected] {
    transitions.map { Selected(source: $0.source, index: $0.transitionIndex) }
}

private func sortedStates(_ states: [St]) -> [St] {
    states.sorted { $0.rawValue < $1.rawValue }
}

// MARK: - computeEntrySet

@Test func theInitialConfigurationEntersEveryRegionByDefault() {
    let entry = computeEntrySet(Chart(), transitions: [fromDocument(to(.s))])
    #expect(entry.statesToEnter == initialConfiguration)
    // A <parallel> is not compound, so it has no initial transition to run.
    #expect(sortedStates(entry.statesForDefaultEntry) == [.s, .r1, .r2, .r3])
    #expect(entry.defaultHistoryContent.isEmpty, "no history was taken, but content is owed")
}

@Test func aTargetSetEntersEveryTargetAndDefaultsTheRegionNoTargetReaches() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.t, to(.a2), to(.b2))])
    #expect(entry.statesToEnter == [.s, .p, .r1, .a2, .r2, .b2, .r3, .c1])
    // s, r1 and r2 are entered as ANCESTORS of a named target, so their initial
    // transitions do not run; only r3, reached by nobody, defaults.
    #expect(entry.statesForDefaultEntry == [.r3])
    #expect(!entry.isDefaultEntry(.s) && entry.isDefaultEntry(.r3))
}

@Test func anUnrecordedShallowHistoryTakesItsDefaultAndOwesItsContentToItsParent() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.a1, toHistory(hb))])
    // The domain is s: the history stands for b2, and the least compound ancestor
    // of a1 and b2 walks past the <parallel>.
    #expect(entry.statesToEnter == [.p, .r1, .a1, .r2, .b2, .r3, .c1])
    #expect(sortedStates(entry.statesForDefaultEntry) == [.r1, .r3])
    #expect(entry.defaultHistoryContentOf(.r2) == hb, "r2 must owe hb's default content")
    #expect(entry.defaultHistoryContentOf(.p) == nil, "p owes no history content")
}

@Test func aRecordedShallowHistoryRestoresWhatItRecordedAndOwesNoContent() {
    let chart = Chart()
    chart.recorded[hb] = [.b1]
    let entry = computeEntrySet(chart, transitions: [entering(.a1, toHistory(hb))])
    #expect(entry.statesToEnter == [.p, .r1, .a1, .r2, .b1, .r3, .c1])
    #expect(entry.defaultHistoryContent.isEmpty, "a recorded history owes no default content")
}

@Test func aDeepHistoryThatRecordedEveryRegionRestoresAllOfThem() {
    let chart = Chart()
    chart.recorded[hs] = [.a2, .b1, .c2]
    let entry = computeEntrySet(chart, transitions: [entering(.t, toHistory(hs))])
    #expect(entry.statesToEnter == [.s, .p, .r1, .a2, .r2, .b1, .r3, .c2])
    // Nothing is entered by default: every compound state on the way is an
    // ancestor of a recorded state.
    #expect(entry.statesForDefaultEntry.isEmpty)
    #expect(entry.defaultHistoryContent.isEmpty, "a recorded history owes no default content")
}

@Test func anUnrecordedDeepHistoryWithAMultiStateDefaultEntersTheWholeSet() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.t, toHistory(hs))])
    #expect(entry.statesToEnter == [.s, .p, .r1, .a2, .r2, .b2, .r3, .c1])
    #expect(entry.statesForDefaultEntry == [.r3])
    #expect(entry.defaultHistoryContentOf(.s) == hs, "s must owe hs's default content")
}

@Test func anInternalTransitionToADescendantLeavesItsSourceOutOfTheSet() {
    let entry = computeEntrySet(Chart(), transitions: [enteringInternal(.s, to(.q1))])
    #expect(entry.statesToEnter == [.q, .q1])
    #expect(entry.statesForDefaultEntry.isEmpty)
}

@Test func anExternalTransitionToADescendantReentersItsSource() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.s, to(.q1))])
    #expect(entry.statesToEnter == [.s, .q, .q1])
    #expect(entry.statesForDefaultEntry.isEmpty)
}

@Test func anExternalTransitionOnARegionRootReentersEverySiblingRegion() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.r1, to(.a2))])
    // A <parallel> is never a domain, so the domain is s, and the sibling regions
    // are exited and entered again at their defaults.
    #expect(entry.statesToEnter == [.p, .r1, .a2, .r2, .b1, .r3, .c1])
    #expect(sortedStates(entry.statesForDefaultEntry) == [.r2, .r3])
}

@Test func anInternalTransitionOnARegionRootStaysInsideItsRegion() {
    let entry = computeEntrySet(Chart(), transitions: [enteringInternal(.r1, to(.a2))])
    #expect(entry.statesToEnter == [.a2])
}

@Test func transitionsOfOneMicrostepEnterInDocumentOrder() {
    // Selected second-region first, as a set is free to be; entry order is
    // document order all the same.
    let entry = computeEntrySet(
        Chart(),
        transitions: [entering(.b1, to(.b2)), entering(.a1, to(.a2))]
    )
    #expect(entry.statesToEnter == [.a2, .b2])
}

@Test func aDeepMultiTargetInitialDefaultsOnlyTheStateThatNamesIt() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.t, to(.m))])
    #expect(entry.statesToEnter == [.s, .m, .mp, .m1, .m1a, .m2, .m2b])
    // m is the target and defaults; m1 and m2 are ancestors of its initial
    // targets, and s an ancestor of the target itself.
    #expect(entry.statesForDefaultEntry == [.m])
}

@Test func aTargetlessTransitionEntersNothing() {
    let entry = computeEntrySet(Chart(), transitions: [entering(.a1)])
    #expect(entry.statesToEnter.isEmpty)
    #expect(entry.statesForDefaultEntry.isEmpty)
}

@Test func theDomainIsAskedOfTheStatesAHistoryStandsFor() {
    let chart = Chart()
    // Unrecorded, hs stands for "a2 b2", whose least compound ancestor with t is
    // the document itself.
    let unrecorded = effectiveTargetStates(chart, targets: [toHistory(hs)])
    #expect(unrecorded == [.a2, .b2])
    #expect(
        transitionDomain(chart, transition: entering(.t), effectiveTargets: unrecorded) == nil,
        "the domain of t -> hs is the <scxml> element"
    )

    // Recorded inside q, an internal transition on q keeps q as its domain.
    chart.recorded[hs] = [.q1]
    let recorded = effectiveTargetStates(chart, targets: [toHistory(hs)])
    #expect(recorded == [.q1])
    #expect(
        transitionDomain(chart, transition: enteringInternal(.q), effectiveTargets: recorded) == .q,
        "the domain of an internal q -> hs is q"
    )
}

// MARK: - isDescendant

@Test(arguments: [(St.a1, St.r1), (.a1, .p), (.m2b, .s)])
private func aChildAndAGrandchildAreDescendants(state: St, ancestor: St) {
    #expect(isDescendant(Chart(), state, of: ancestor), "\(state) lies below \(ancestor)")
}

@Test func aStateIsNotItsOwnDescendant() {
    // Appendix D's isDescendant is strict.
    #expect(!isDescendant(Chart(), St.r1, of: .r1))
}

@Test(arguments: [(St.a1, St.r2), (.q1, .p), (.t, .s)])
private func statesOnUnrelatedBranchesAreNotDescendants(state: St, ancestor: St) {
    #expect(!isDescendant(Chart(), state, of: ancestor), "\(state) does not lie below \(ancestor)")
}

// MARK: - Domains and exit sets

@Test func aSelfTransitionExitsOnlyItsSource() {
    let chart = Chart.atInitialConfiguration()
    // §scxml-D-findLCCA chooses among the PROPER ancestors, so the domain of
    // a1 -> a1 is r1, and the exit set is a1 alone. A source that were its own
    // domain would exit nothing, and the self-transition would not leave and
    // re-enter its state at all.
    #expect(computeExitSet(chart, transition: enabled(.a1, 4), configuration: chart.activeConfiguration) == [.a1])
}

@Test func anExternalTransitionLeavingAParallelExitsEveryRegion() {
    let chart = Chart.atInitialConfiguration()
    // p -> t: the domain is the <scxml> element, so the whole configuration goes,
    // the sibling regions of p included, which no walk up from the source alone
    // could name.
    let exitSet = computeExitSet(chart, transition: enabled(.p, 0), configuration: chart.activeConfiguration)
    #expect(sortedStates(exitSet) == sortedStates(chart.activeConfiguration))
}

@Test func anInternalTransitionFromARegionRootExitsOnlyInsideTheRegion() {
    let chart = Chart.atInitialConfiguration()
    let internalTransition = EnabledTransition(
        source: St.r1,
        targets: [to(.a2)],
        transitionIndex: 0,
        hasActions: false,
        isInternal: true
    )
    var externalTransition = internalTransition
    externalTransition.isInternal = false
    #expect(
        computeExitSet(chart, transition: internalTransition, configuration: chart.activeConfiguration) == [.a1]
    )
    // Written external, the same transition's domain is s (a <parallel> is never a
    // domain), so every region goes with it.
    #expect(
        sortedStates(
            computeExitSet(chart, transition: externalTransition, configuration: chart.activeConfiguration)
        ) == [.p, .r1, .a1, .r2, .b1, .r3, .c1]
    )
}

@Test func aTargetlessTransitionExitsNothing() {
    let chart = Chart.atInitialConfiguration()
    #expect(computeExitSet(chart, transition: enabled(.a1, 2), configuration: chart.activeConfiguration).isEmpty)
}

@Test func theStatesToExitComeOutInReverseDocumentOrder() {
    let chart = Chart.atInitialConfiguration()
    #expect(
        computeStatesToExit(chart, transitions: [enabled(.p, 0)], configuration: chart.activeConfiguration)
            == [.c1, .r3, .b1, .r2, .a1, .r1, .p, .s]
    )
}

// MARK: - selectTransitions / removeConflictingTransitions

@Test func everyRegionTakesItsOwnTransition() {
    // §scxml-3.4: two regions' transitions have domains in disjoint subtrees, so
    // their exit sets are disjoint and both survive.
    let chart = Chart.atInitialConfiguration()
    #expect(sources(selectTransitions(chart, event: .e)) == [Selected(source: .a1, index: 0), Selected(source: .b1, index: 0)])
}

@Test func aTransitionSelectedFirstPreemptsALaterOneThatIsNotItsDescendant() {
    // a1 selects a1 -> a2; b1 walks up to p -> t, which exits a1 too, and p does
    // not descend from a1: it is preempted.
    let chart = Chart.atInitialConfiguration()
    #expect(sources(selectTransitions(chart, event: .f)) == [Selected(source: .a1, index: 1)])
}

@Test func aDescendantSourcePreemptsAnAncestorSelectedBeforeIt() {
    // a1 walks up to p -> t first; b1 then selects b1 -> b2, which conflicts with
    // it and descends from p, so it wins. c1 reaches p -> t again: one transition,
    // already in the set.
    let chart = Chart.atInitialConfiguration()
    #expect(sources(selectTransitions(chart, event: .g)) == [Selected(source: .b1, index: 1)])
}

@Test func aTargetlessTransitionIsNeverPreempted() {
    // W3C test 403c: an empty exit set conflicts with nothing, so a1's targetless
    // transition survives the p -> t that exits a1 itself.
    let chart = Chart.atInitialConfiguration()
    #expect(
        sources(selectTransitions(chart, event: .h))
            == [Selected(source: .a1, index: 2), Selected(source: .p, index: 2)]
    )
}

@Test func aTransitionReachedFromTwoRegionsIsSelectedOnce() {
    // In the initial configuration only c1 walks up to p's targetless K; a1 and b1
    // answer K themselves. Move the first two regions to a2 and b2, which answer
    // nothing, and all three atomic states reach it: it is still one element of the
    // set.
    let chart = Chart.atInitialConfiguration()
    #expect(
        sources(selectTransitions(chart, event: .k))
            == [Selected(source: .a1, index: 3), Selected(source: .b1, index: 2), Selected(source: .p, index: 3)]
    )
    chart.activeConfiguration = [.s, .p, .r1, .a2, .r2, .b2, .r3, .c1]
    #expect(sources(selectTransitions(chart, event: .k)) == [Selected(source: .p, index: 3)])
}

@Test func anEventlessSelectionWithNothingEnabledSelectsNothing() {
    #expect(selectTransitions(Chart.atInitialConfiguration(), event: .null).isEmpty)
}

// MARK: - The microstep

@Test func aMicrostepExitsThenRunsContentInSelectionOrderThenEnters() {
    let chart = Chart.atInitialConfiguration()
    microstep(chart, transitions: selectTransitions(chart, event: .k))
    // §scxml-D-executeTransitionContent runs content in SELECTION order: p's K was
    // reached last, from c1, though p comes first in document order.
    #expect(
        chart.log == [
            "exit B1",
            "exit A1",
            "content A1#3",
            "content B1#2",
            "content P#3",
            "enter A2",
            "enter B2",
        ]
    )
}

@Test func everyExitIsHandedTheConfigurationBeforeTheFirstExit() {
    let chart = Chart.atInitialConfiguration()
    let before = chart.activeConfiguration
    // K exits two states, b1 then a1: the second exit must still be handed the
    // configuration from before the first. A microstep with one exit could not tell
    // that from the configuration as it stands at each exit.
    microstep(chart, transitions: selectTransitions(chart, event: .k))
    #expect(chart.exitsSaw == [before, before])
}

@Test func aSelfTransitionLeavesAndReentersItsState() {
    let chart = Chart.atInitialConfiguration()
    let taken = selectTransitions(chart, event: .r)
    #expect(sources(taken) == [Selected(source: .a1, index: 4), Selected(source: .b1, index: 3)])
    microstep(chart, transitions: taken)
    #expect(
        chart.log == [
            "exit B1",
            "exit A1",
            "content A1#4",
            "content B1#3",
            "enter A1",
            "enter B2",
        ]
    )
}

@Test func aHistoryDefaultContentRunsAfterItsParentIsEntered() {
    let chart = Chart.atInitialConfiguration()
    let toHb = EnabledTransition(
        source: St.a1,
        targets: [toHistory(hb)],
        transitionIndex: 0,
        hasActions: false,
        isInternal: false
    )
    microstep(chart, transitions: [toHb])
    // The domain is s, so all of p leaves and comes back; r2 is an ancestor of the
    // default target b2, not entered by default, and the history's default content
    // runs once r2 has been entered. The transition itself has no content, so the
    // machine is not asked to run any.
    #expect(
        chart.log == [
            "exit C1",
            "exit R3",
            "exit B1",
            "exit R2",
            "exit A1",
            "exit R1",
            "exit P",
            "enter P",
            "enter R1 (default)",
            "enter A1",
            "enter R2",
            "history default Hb",
            "enter B2",
            "enter R3 (default)",
            "enter C1",
        ]
    )
}

// MARK: - isInFinalState
//
//   run (parallel)
//     form > filling, formDone (final)
//     checks (parallel)
//       left  > leftPending,  leftDone (final)
//       right > rightPending, rightDone (final)
//
// checks is a <parallel> among the regions of a <parallel>: it has no <final> child
// of its own and is final only because both of its regions are.

private enum Cs: Int, Hashable {
    case run, form, filling, formDone, checks, left, leftPending, leftDone, right, rightPending, rightDone
}

private let completionParent: [Cs: Cs] = [
    .form: .run, .checks: .run,
    .filling: .form, .formDone: .form,
    .left: .checks, .right: .checks,
    .leftPending: .left, .leftDone: .left,
    .rightPending: .right, .rightDone: .right,
]

private let completionChildren: [Cs: [Cs]] = [
    .run: [.form, .checks],
    .form: [.filling, .formDone],
    .checks: [.left, .right],
    .left: [.leftPending, .leftDone],
    .right: [.rightPending, .rightDone],
]

/// The document above; it declares no `<history>`.
private struct Completion: Document {
    typealias State = Cs

    func parentOf(_ state: Cs) -> Cs? { completionParent[state] }

    func isCompound(_ state: Cs) -> Bool { state == .form || state == .left || state == .right }

    func isParallel(_ state: Cs) -> Bool { state == .run || state == .checks }

    func isFinal(_ state: Cs) -> Bool { state == .formDone || state == .leftDone || state == .rightDone }

    func childStates(_ state: Cs) -> [Cs] { completionChildren[state] ?? [] }

    func initialTargets(_ state: Cs) -> [EntryTarget<Cs>] { [] }

    func historyParent(_ history: HistoryID) -> Cs {
        preconditionFailure("the completion document declares no <history>")
    }

    func historyValue(_ history: HistoryID) -> [Cs]? { nil }

    func historyDefaultTargets(_ history: HistoryID) -> [EntryTarget<Cs>] { [] }

    func documentOrder(_ state: Cs) -> Int { state.rawValue }
}

@Test func aCompoundStateIsFinalWhenAFinalChildIsActive() {
    let doc = Completion()
    #expect(
        isInFinalState(
            doc,
            state: .form,
            configuration: [.run, .form, .formDone, .checks, .left, .leftPending, .right, .rightPending]
        ),
        "form is in a final state while formDone is active"
    )
    #expect(
        !isInFinalState(
            doc,
            state: .form,
            configuration: [.run, .form, .filling, .checks, .left, .leftPending, .right, .rightPending]
        ),
        "form is not in a final state while filling is active"
    )
}

@Test func aNestedParallelIsFinalOnlyWhenEveryRegionIs() {
    let doc = Completion()
    #expect(
        isInFinalState(doc, state: .checks, configuration: [.checks, .left, .leftDone, .right, .rightDone]),
        "checks is final once both of its regions are"
    )
    #expect(
        !isInFinalState(doc, state: .checks, configuration: [.checks, .left, .leftDone, .right, .rightPending]),
        "one region still pending leaves the <parallel> short of final"
    )
}

@Test func aParallelRegionOfAParallelCounts() {
    let doc = Completion()
    #expect(
        isInFinalState(
            doc,
            state: .run,
            configuration: [.run, .form, .formDone, .checks, .left, .leftDone, .right, .rightDone]
        ),
        "checks has no <final> child of its own; it is final because both of its regions are"
    )
    #expect(
        !isInFinalState(
            doc,
            state: .run,
            configuration: [.run, .form, .formDone, .checks, .left, .leftDone, .right, .rightPending]
        ),
        "run is not final while one region of checks is pending"
    )
}

@Test func neitherAnAtomicStateNorAFinalElementIsInAFinalState() {
    let doc = Completion()
    let configuration: [Cs] = [.run, .form, .formDone, .checks, .left, .leftPending, .right, .rightPending]
    #expect(
        !isInFinalState(doc, state: .leftPending, configuration: configuration),
        "an atomic state has no <final> child to be active"
    )
    #expect(
        !isInFinalState(doc, state: .formDone, configuration: configuration),
        "being a <final> element is not being IN a final state; the predicate asks about children"
    )
}
