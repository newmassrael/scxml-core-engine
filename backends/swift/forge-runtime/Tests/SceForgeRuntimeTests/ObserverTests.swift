// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A threshold has two states, and each call says whether it changed one. A queue keeps what was
// pushed in the order it was pushed.

@Test func aThresholdIsEnteredOnceWhereItsConditionHolds() {
    let state = ThresholdState()
    #expect(!state.active)
    #expect(!state.enterIf(false))
    #expect(state.enterIf(true))
    #expect(state.active)
    // Already entered: the same condition is no transition.
    #expect(!state.enterIf(true))
}

@Test func aThresholdIsLeftOnlyWhileItIsEntered() {
    let state = ThresholdState()
    // Not entered: nothing to leave.
    #expect(!state.leaveIf(true))
    _ = state.enterIf(true)
    #expect(!state.leaveIf(false))
    #expect(state.leaveIf(true))
    #expect(!state.active)
    #expect(!state.leaveIf(true))
}

@Test func aThresholdHoldsAcrossTheBandBetweenItsTwoConditions() {
    // Entered above 110, left below 100: 105 is in the band and changes nothing either way.
    let state = ThresholdState()
    func step(_ temperature: Double) -> (entered: Bool, left: Bool) {
        (state.enterIf(temperature > 110), state.leaveIf(temperature < 100))
    }
    #expect(step(105) == (false, false))
    #expect(step(115) == (true, false))
    #expect(step(105) == (false, false))
    #expect(state.active)
    #expect(step(95) == (false, true))
    #expect(!state.active)
}

@Test func resettingAThresholdLeavesItWithoutAnEvent() {
    let state = ThresholdState()
    _ = state.enterIf(true)
    state.reset()
    #expect(!state.active)
    #expect(state.enterIf(true))
}

private enum Tag { case warning, critical }

@Test func aQueueKeepsItsEventsInTheOrderTheyWerePushed() {
    var queue = EventQueue<Tag>()
    #expect(queue.isEmpty)
    // A mutating call cannot stand inside `#expect`, which reads its argument as a value.
    let first = queue.push(.warning)
    let second = queue.push(.critical)
    #expect(first && second)
    #expect(queue.size == 2)
    #expect(queue[0] == .warning)
    #expect(queue[1] == .critical)
    #expect(queue.asArray() == [.warning, .critical])
}

@Test func clearingAQueueDropsEveryEvent() {
    var queue = EventQueue<Tag>()
    _ = queue.push(.warning)
    queue.clear()
    #expect(queue.isEmpty)
    #expect(queue.size == 0)
}

@Test func aQueueIsAValueSoACopyIsNotMovedByTheOriginal() {
    var queue = EventQueue<Tag>()
    _ = queue.push(.warning)
    let copy = queue
    _ = queue.push(.critical)
    #expect(copy.size == 1)
    #expect(queue.size == 2)
}
