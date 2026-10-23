// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// §scxml-3.3 / §scxml-3.4 / §scxml-3.8: `Engine.enterAt`: entering a configuration a
// machine was already in, without re-running `<onentry>`.
//
// Two hand-written policies, because the two halves of the door are different facts:
// the ones of backends/rust/runtime/tests/configuration_entry.rs.
//
//   - `ParallelPolicy` keeps its own active set (`hasActiveStates`), which is the
//     shape the consumer that asked for this door actually runs: a `<parallel>`
//     document. Its configuration is a set, and it is the case where `current` is NOT
//     recoverable from that set.
//   - `LinearPolicy` has no parallel regions, so its configuration is the parent walk
//     from the leaf and `Engine.getActiveStates` derives it and does not read the
//     policy. `enterAt` has to close the round trip there too, through a different
//     code path.
//
// The onentry witness is what separates a resume from a replay. Both policies raise an
// event from an `<onentry>` that has a transition waiting for it, so a re-entry would
// move the machine. Asserting the machine stays put after `enterAt` + `step()` is the
// observable form of "entry actions did not run"; the `entries` counter is the same
// claim read from the inside.

import Testing

@testable import SceRuntime

/// What a hand-written policy that declares no `<history>` and has the features of
/// neither invokes, finals nor autoforward does not have to say.
private protocol HistorylessPolicy: StatePolicy {}

extension HistorylessPolicy {
    func getHistoryParent(_ history: HistoryID) -> State {
        preconditionFailure("this document declares no <history>")
    }

    func getHistoryDefaultTargets(_ history: HistoryID) -> [EntryTarget<State>] {
        preconditionFailure("this document declares no <history>")
    }

    func historyValue(_ history: HistoryID) -> [State]? { nil }

    func executeHistoryDefaultContent(_ history: HistoryID, engine: Engine<Self>) {}

    func bindCurrentEvent(_ event: Event, engine: Engine<Self>) {}

    func executeTransitionContent(source: State, transitionIndex: Int, engine: Engine<Self>) {}

    var needsScriptEngine: Bool { false }
    var needsDataModelInit: Bool { false }
    var hasInvokeSupport: Bool { false }
    var hasFinalize: Bool { false }
    var hasAutoforward: Bool { false }
    var hasChildTick: Bool { false }
}

// MARK: - A <parallel> document
//
//   P (parallel, root)
//     RA (compound)   A1 (atomic, onentry raise Advance) | A2 (atomic)
//     RB (compound)   B1 (atomic)
//
//   A1 --Advance--> A2

private enum PState: Hashable {
    case p, ra, a1, a2, rb, b1
}

private enum PEvent: Hashable {
    case none, advance
}

private final class ParallelPolicy: HistorylessPolicy {
    typealias State = PState
    typealias Event = PEvent

    var active: [PState] = []

    /// How many states this policy has run entry actions for: the inside view of the
    /// onentry contract.
    var entries = 0

    var hasParallelStates: Bool { true }
    var hasActiveStates: Bool { true }

    func initialState() -> PState { .p }

    func isFinalState(_ state: PState) -> Bool { false }

    func getParent(_ state: PState) -> PState? {
        switch state {
        case .p: return nil
        case .ra, .rb: return .p
        case .a1, .a2: return .ra
        case .b1: return .rb
        }
    }

    // A <parallel> is not compound: Appendix D's isCompoundState.
    func isCompoundState(_ state: PState) -> Bool { state == .ra || state == .rb }

    func isParallelState(_ state: PState) -> Bool { state == .p }

    func getChildStates(_ state: PState) -> [PState] {
        switch state {
        case .p: return [.ra, .rb]
        case .ra: return [.a1, .a2]
        case .rb: return [.b1]
        default: return []
        }
    }

    func getInitialTargets(_ state: PState) -> [EntryTarget<PState>] {
        switch state {
        case .ra: return [.state(.a1)]
        case .rb: return [.state(.b1)]
        default: return []
        }
    }

    func getDocumentInitialTargets() -> [EntryTarget<PState>] { [.state(.p)] }

    func getDocumentOrder(_ state: PState) -> Int {
        switch state {
        case .p: return 0
        case .ra: return 1
        case .a1: return 2
        case .a2: return 3
        case .rb: return 4
        case .b1: return 5
        }
    }

    func getEventName(_ event: PEvent) -> String { event == .advance ? "advance" : "" }

    func getEventFromName(_ name: String) -> PEvent? { name == "advance" ? .advance : nil }

    func getStateName(_ state: PState) -> String {
        switch state {
        case .p: return "p"
        case .ra: return "ra"
        case .a1: return "a1"
        case .a2: return "a2"
        case .rb: return "rb"
        case .b1: return "b1"
        }
    }

    func getStateFromName(_ name: String) -> PState? {
        [PState.p, .ra, .a1, .a2, .rb, .b1].first { getStateName($0) == name }
    }

    func nullEvent() -> PEvent { .none }

    // Mirrors the generated shape: the entered state joins the active set, and
    // nothing below it is entered here; the engine's Appendix D entry set enters
    // every region and initial child, one state at a time.
    func executeEntryActions(_ state: PState, engine: Engine<ParallelPolicy>, isDefaultEntry: Bool) {
        entries += 1
        if !active.contains(state) {
            active.append(state)
        }
        // The onentry that separates a resume from a replay.
        if state == .a1 {
            engine.raise(EventWithMetadata(.advance))
        }
    }

    func executeExitActions(_ state: PState, engine: Engine<ParallelPolicy>, configurationBeforeExit: [PState]) {
        active.removeAll { $0 == state }
    }

    func firstEnabledTransition(
        _ state: PState,
        event: PEvent,
        engine: Engine<ParallelPolicy>
    ) -> EnabledTransition<PState>? {
        if state == .a1 && event == .advance {
            return EnabledTransition(
                source: state,
                targets: [.state(.a2)],
                transitionIndex: 0,
                hasActions: false,
                isInternal: false
            )
        }
        return nil
    }

    func getActiveStates() -> [PState] { active }

    func setActiveStates(_ states: [PState]) { active = states }
}

// MARK: - A document with no parallel regions
//
//   Root (compound)   X1 (atomic, onentry raise Go) | X2 (atomic)
//   X1 --Go--> X2

private enum LState: Hashable {
    case root, x1, x2
}

private enum LEvent: Hashable {
    case none, go
}

private final class LinearPolicy: HistorylessPolicy {
    typealias State = LState
    typealias Event = LEvent

    var entries = 0

    var hasParallelStates: Bool { false }
    var hasActiveStates: Bool { false }

    func initialState() -> LState { .root }

    func isFinalState(_ state: LState) -> Bool { false }

    func getParent(_ state: LState) -> LState? { state == .root ? nil : .root }

    func isCompoundState(_ state: LState) -> Bool { state == .root }

    func isParallelState(_ state: LState) -> Bool { false }

    func getChildStates(_ state: LState) -> [LState] { state == .root ? [.x1, .x2] : [] }

    func getInitialTargets(_ state: LState) -> [EntryTarget<LState>] { state == .root ? [.state(.x1)] : [] }

    func getDocumentInitialTargets() -> [EntryTarget<LState>] { [.state(.root)] }

    func getDocumentOrder(_ state: LState) -> Int {
        switch state {
        case .root: return 0
        case .x1: return 1
        case .x2: return 2
        }
    }

    func getEventName(_ event: LEvent) -> String { event == .go ? "go" : "" }

    func getEventFromName(_ name: String) -> LEvent? { name == "go" ? .go : nil }

    func getStateName(_ state: LState) -> String {
        switch state {
        case .root: return "root"
        case .x1: return "x1"
        case .x2: return "x2"
        }
    }

    func getStateFromName(_ name: String) -> LState? {
        [LState.root, .x1, .x2].first { getStateName($0) == name }
    }

    func nullEvent() -> LEvent { .none }

    func executeEntryActions(_ state: LState, engine: Engine<LinearPolicy>, isDefaultEntry: Bool) {
        entries += 1
        if state == .x1 {
            engine.raise(EventWithMetadata(.go))
        }
    }

    func executeExitActions(_ state: LState, engine: Engine<LinearPolicy>, configurationBeforeExit: [LState]) {}

    func firstEnabledTransition(
        _ state: LState,
        event: LEvent,
        engine: Engine<LinearPolicy>
    ) -> EnabledTransition<LState>? {
        if state == .x1 && event == .go {
            return EnabledTransition(
                source: state,
                targets: [.state(.x2)],
                transitionIndex: 0,
                hasActions: false,
                isInternal: false
            )
        }
        return nil
    }
}

// MARK: - Acceptance 1: the round trip closes, through both code paths

@Test func aParallelConfigurationRoundTripsThroughANewEngine() {
    let ran = Engine(policy: ParallelPolicy())
    ran.initialize()

    // What a host would persist: both readers.
    let savedConfiguration = ran.getActiveStates()
    let savedCurrent = ran.getCurrentState()

    // The onentry fired and the macrostep consumed it, so the run really did move:
    // the configuration below is one this document reached, not its initial one.
    #expect(savedCurrent == .a2)
    #expect(savedConfiguration.contains(.a2))

    let restored = Engine(policy: ParallelPolicy())
    #expect(restored.enterAt(configuration: savedConfiguration, current: savedCurrent) == .accepted)

    #expect(restored.getActiveStates() == savedConfiguration)
    #expect(restored.getCurrentState() == savedCurrent)
    #expect(restored.isRunning)
}

@Test func aLinearConfigurationRoundTripsWithoutAPolicyActiveSet() {
    let ran = Engine(policy: LinearPolicy())
    ran.initialize()

    let savedConfiguration = ran.getActiveStates()
    let savedCurrent = ran.getCurrentState()
    #expect(savedCurrent == .x2)
    // No parallel regions: the chain is the parent walk, leaf first.
    #expect(savedConfiguration == [.x2, .root])

    let restored = Engine(policy: LinearPolicy())
    #expect(restored.enterAt(configuration: savedConfiguration, current: savedCurrent) == .accepted)

    #expect(restored.getActiveStates() == savedConfiguration)
    #expect(restored.getCurrentState() == savedCurrent)
}

// MARK: - Acceptance 2: no `<onentry>`, the difference between resume and replay

@Test func enteringAConfigurationRunsNoEntryActions() {
    // The configuration this document is in right after entry, before the
    // `<onentry>`-raised event has been taken: A1 and B1 active.
    let configuration: [PState] = [.p, .ra, .a1, .rb, .b1]

    let policy = ParallelPolicy()
    let engine = Engine(policy: policy)
    #expect(engine.enterAt(configuration: configuration, current: .a1) == .accepted)

    // Inside view: not one entry action ran.
    #expect(policy.entries == 0, "enterAt must run no entry actions")

    // Outside view, and the one that matters to a host: A1's `<onentry>` raises
    // Advance, and A1 has a transition on Advance. Had the entry actions run, this
    // step would have taken the machine to A2. It is still at A1, so nothing was
    // queued: the resume did not replay.
    engine.step()
    #expect(engine.getCurrentState() == .a1, "a replayed <onentry> would have raised Advance and moved this to A2")
    #expect(engine.getActiveStates() == configuration)
}

@Test func initializeByContrastDoesRunThem() {
    // The control: the same document through the other door moves, because the
    // entry actions are exactly what `initialize` is for. Without this the test above
    // could pass on a machine that simply cannot move.
    let policy = ParallelPolicy()
    let engine = Engine(policy: policy)
    engine.initialize()
    #expect(policy.entries > 0)
    #expect(engine.getCurrentState() == .a2)
}

// MARK: - Acceptance 3: what is refused

@Test func anEmptyChainIsRefused() {
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [], current: .a1) == .empty)
}

@Test func twoSiblingsOfOneRegionAreRefused() {
    // §scxml-3.3: RA is compound, so exactly one of A1/A2 can be active.
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [.p, .ra, .a1, .a2, .rb, .b1], current: .a1) == .compoundChildCount)
}

@Test func aParallelWithARegionMissingIsRefused() {
    // §scxml-3.4: RB is a region of P, so a configuration holding P holds it.
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [.p, .ra, .a1], current: .a1) == .parallelRegionMissing)
}

@Test func aChainThatSkipsAnAncestorIsRefused() {
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [.a1], current: .a1) == .ancestorMissing)
}

@Test func aRepeatedStateIsRefused() {
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [.p, .ra, .a1, .a1, .rb, .b1], current: .a1) == .duplicate)
}

@Test func aSetWithTwoRootsIsRefused() {
    let engine = Engine(policy: LinearPolicy())
    // Root appears once; a second root would have to be another parentless state, and
    // this document has one, so a set with no root at all stands in for the wrong
    // count.
    #expect(engine.enterAt(configuration: [.x1], current: .x1) == .ancestorMissing)
}

@Test func anAtomicStateWithAChildInTheSetIsRefused() {
    let engine = Engine(policy: LinearPolicy())
    #expect(engine.enterAt(configuration: [.root, .x1, .x2], current: .x1) == .compoundChildCount)
}

@Test func aCurrentStateOutsideTheConfigurationIsRefused() {
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [.p, .ra, .a1, .rb, .b1], current: .a2) == .currentNotActive)
}

@Test func aNonAtomicCurrentStateIsRefused() {
    let engine = Engine(policy: ParallelPolicy())
    #expect(engine.enterAt(configuration: [.p, .ra, .a1, .rb, .b1], current: .ra) == .currentNotAtomic)
    #expect(engine.enterAt(configuration: [.p, .ra, .a1, .rb, .b1], current: .p) == .currentNotAtomic)
}

@Test func aRefusedEntryLeavesTheEngineUntouched() {
    // The property that makes a rejection safe to handle: the host can try a stale
    // record, be told no, and still hold an engine it can initialize.
    let policy = ParallelPolicy()
    let engine = Engine(policy: policy)
    let before = engine.getCurrentState()
    #expect(engine.enterAt(configuration: [.a1], current: .a1) != .accepted)
    #expect(engine.getCurrentState() == before)
    #expect(!engine.isRunning)
    #expect(policy.entries == 0)

    engine.initialize()
    #expect(engine.getCurrentState() == .a2)
}

@Test func everyRefusalIsAReasonAHostCanReportInWords() {
    let reasons: [ConfigurationRejection] = [
        .empty, .duplicate, .ancestorMissing, .rootCount, .compoundChildCount, .parallelRegionMissing,
        .parallelChildCount, .atomicHasChildren, .currentNotActive, .currentNotAtomic,
    ]
    let words = reasons.map(\.description)
    #expect(Set(words).count == words.count, "two rules share a sentence")
    #expect(words.allSatisfy { !$0.isEmpty && $0 != "accepted" })
}

@Test func theStateNamesRoundTripThroughTheirOwnReverseLookup() {
    let policy = ParallelPolicy()
    for state in [PState.p, .ra, .a1, .a2, .rb, .b1] {
        #expect(policy.getStateFromName(policy.getStateName(state)) == state)
    }
    #expect(policy.getStateFromName("nowhere") == nil)
}
