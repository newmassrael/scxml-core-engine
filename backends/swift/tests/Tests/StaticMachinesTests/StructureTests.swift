// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import StaticMachines
import Testing

// What the generated machines of `sce-build/tests/fixtures/swift_statecharts` do
// under their `Engine`: the structure of a statechart (hierarchy, regions, history,
// final states), its transitions, `<raise>` and the In() guard. Each expectation is
// worked from the document, not read off a run.

private func started<Policy: StatePolicy>(_ policy: Policy) -> Engine<Policy> {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    return engine
}

// MARK: - structure_light

@Test func theLampIsOffUntilItIsPowered() {
    let engine = started(StructureLightPolicy())
    #expect(engine.getCurrentState() == .off)
    #expect(engine.getActiveStates() == [.off])
    #expect(engine.isRunning)
}

@Test func poweringTheLampEntersGreenWhoseEntryRaisesWarmAndThatMovesItToYellow() {
    let engine = started(StructureLightPolicy())
    engine.processEvent(.power)
    // `on` is compound and enters at its initial `green`; green's <onentry> raises
    // `warm`, and `warm` is taken in the same macrostep because In('green') holds
    // while green is still active.
    #expect(engine.getCurrentState() == .yellow)
    #expect(Set(engine.getActiveStates()) == [.on, .yellow])
}

@Test func theTimerWalksTheLampAroundAndGreenIsOnlyEverPassedThrough() {
    let engine = started(StructureLightPolicy())
    engine.processEvent(.power)
    engine.processEvent(.timer)
    #expect(engine.getCurrentState() == .red)
    // red -> green enters green again; its <onentry> raises warm again.
    engine.processEvent(.timer)
    #expect(engine.getCurrentState() == .yellow)
}

@Test func powerLeavesTheWholeOfOnFromWhicheverChildItIsIn() {
    let engine = started(StructureLightPolicy())
    engine.processEvent(.power)
    engine.processEvent(.timer)
    #expect(engine.getCurrentState() == .red)
    engine.processEvent(.power)
    // The transition is on `on`, so it applies in any of its children.
    #expect(engine.getCurrentState() == .off)
    #expect(engine.getActiveStates() == [.off])
}

@Test func haltFromRedEndsTheRunInTheTopLevelFinal() {
    let policy = StructureLightPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    var completions = 0
    engine.setCompletionCallback { completions += 1 }
    engine.initialize()
    engine.processEvent(.power)
    engine.processEvent(.timer)
    engine.processEvent(.halt)
    #expect(engine.isInFinalState)
    #expect(engine.terminalState == .stopped)
    #expect(completions == 1)
    #expect(engine.getActiveStates().isEmpty)
}

@Test func anEventNoActiveStateAnswersIsDiscardedAndCounted() {
    let engine = started(StructureLightPolicy())
    // `halt` is only answered in red.
    engine.processEvent(.halt)
    #expect(engine.getCurrentState() == .off)
    #expect(engine.discardedExternalEvents == 1)
    #expect(engine.lastDiscardedEvent == .halt)
}

@Test func theStateNamesRoundTripThroughTheirReverseLookup() {
    let policy = StructureLightPolicy()
    for state in StructureLightAllStates {
        #expect(policy.getStateFromName(policy.getStateName(state)) == state)
    }
    #expect(policy.getStateFromName("nowhere") == nil)
    #expect(policy.getEventFromName("power") == .power)
    #expect(policy.getEventFromName("nothing") == nil)
}

@Test func theMachineCanBePutBackWhereItWasWithoutRunningAnEntry() {
    let ran = started(StructureLightPolicy())
    ran.processEvent(.power)
    ran.processEvent(.timer)
    let configuration = ran.getActiveStates()
    let current = ran.getCurrentState()

    let policy = StructureLightPolicy()
    let restored = Engine(policy: policy)
    restored.setClock(ManualClock())
    #expect(restored.enterAt(configuration: configuration, current: current) == .accepted)
    #expect(restored.getCurrentState() == .red)
    // Its <onentry> did not run: red --timer--> green would raise warm and move on
    // to yellow, and a resumed machine is in red.
    restored.step()
    #expect(restored.getCurrentState() == .red)
}

@Test func aSessionIdAHostGivesIsKeptAndOneThatIsNoNmtokenIsRefused() throws {
    let policy = StructureLightPolicy()
    try policy.setSessionID("session-9")
    #expect(policy.sessionID == "session-9")
    #expect(throws: SendArgumentError.self) { try policy.setSessionID("not a token") }
    #expect(policy.sessionID == "session-9")
    #expect(isNmtoken(StructureLightPolicy().sessionID), "a drawn id is a token")
}

// MARK: - structure_regions

@Test func startingTheJobEntersBothRegionsAtTheirInitialStates() {
    let engine = started(StructureRegionsPolicy())
    #expect(engine.getActiveStates() == [.idle])
    engine.processEvent(.start)
    #expect(
        Set(engine.getActiveStates())
            == [.job, .work, .left, .leftBusy, .right, .rightBusy]
    )
}

@Test func oneRegionFinishingLeavesTheParallelRunning() {
    let engine = started(StructureRegionsPolicy())
    engine.processEvent(.start)
    engine.processEvent(.leftDone)
    #expect(Set(engine.getActiveStates()) == [.job, .work, .left, .leftDone, .right, .rightBusy])
}

@Test func theParallelCompletesOnlyWhenEveryRegionHasFinishedAndThenTakesDoneState() {
    let engine = started(StructureRegionsPolicy())
    engine.processEvent(.start)
    engine.processEvent(.leftDone)
    engine.processEvent(.rightDone)
    // done.state.work was raised when the second region reached its final, and the
    // transition on it left the <parallel> for `finished`.
    #expect(Set(engine.getActiveStates()) == [.job, .finished])
    #expect(engine.getCurrentState() == .finished)
}

@Test func pausingARunningJobRecordsItsDeepHistoryAndResumingRestoresTheRegions() {
    let engine = started(StructureRegionsPolicy())
    engine.processEvent(.start)
    engine.processEvent(.leftDone)
    engine.processEvent(.pause)
    #expect(engine.getActiveStates() == [.idle])
    // `remembered` is a deep history of `job`: the atomic states that were active
    // when job was left, one per region, are what resume enters.
    engine.processEvent(.resume)
    #expect(Set(engine.getActiveStates()) == [.job, .work, .left, .leftDone, .right, .rightBusy])
}

@Test func resumingBeforeAnythingWasRecordedTakesTheHistorysDefault() {
    let engine = started(StructureRegionsPolicy())
    engine.processEvent(.resume)
    // Nothing recorded yet: the default transition names `work`, which enters at the
    // regions' initial states.
    #expect(
        Set(engine.getActiveStates())
            == [.job, .work, .left, .leftBusy, .right, .rightBusy]
    )
}

@Test func theHistoryIsReadableThroughThePolicyOnceItsParentHasBeenLeft() {
    let policy = StructureRegionsPolicy()
    let engine = started(policy)
    #expect(policy.historyValue(StructureRegionsHistory.remembered) == nil)
    engine.processEvent(.start)
    engine.processEvent(.pause)
    #expect(Set(policy.historyValue(StructureRegionsHistory.remembered) ?? []) == [.leftBusy, .rightBusy])
    #expect(policy.getHistoryParent(StructureRegionsHistory.remembered) == .job)
}
