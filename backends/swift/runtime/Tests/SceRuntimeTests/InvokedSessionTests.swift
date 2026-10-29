// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// An invoked child session, driven the way a parent drives it: through `ChildEngine`,
// by name, with the queue and the id the parent gave it.

private func makeSession(
    _ script: (ScriptedPolicy) -> Void = { _ in }
) -> (InvokedSession<ScriptedPolicy>, ScriptedPolicy, ParentEventQueue) {
    let policy = ScriptedPolicy()
    policy.transitions[.idle] = [FlatTransition(event: .go, target: .busy)]
    policy.transitions[.busy] = [FlatTransition(event: .again, target: .done)]
    script(policy)
    let queue = ParentEventQueue()
    let session = InvokedSession(policy: policy, sessionID: "parent.child", queue: queue)
    return (session, policy, queue)
}

@Test func aSessionIsGivenTheIdAndTheQueueItsParentDerived() {
    let (session, policy, queue) = makeSession()
    #expect(policy.sessionID == "parent.child")
    #expect(session.parentEventQueue === queue)
    #expect(policy.parentEventQueue === queue)
}

@Test func aSessionTakesAnEventDeliveredByNameWhenItSteps() {
    let (session, _, _) = makeSession()
    session.initialize()
    #expect(session.engine.getCurrentState() == .idle)
    session.raiseExternalByName("go", eventData: "")
    session.step()
    #expect(session.engine.getCurrentState() == .busy)
}

@Test func aSessionThatReachesItsFinalStateSaysSoOnce() {
    let (session, _, _) = makeSession()
    var completions = 0
    session.setCompletionCallback { completions += 1 }
    session.initialize()
    #expect(!session.isInFinalState)
    session.raiseExternalByName("go", eventData: "")
    session.step()
    session.raiseExternalByName("again", eventData: "")
    session.step()
    #expect(session.isInFinalState)
    #expect(completions == 1)
}

@Test func aSessionReportsTheDonedataItsFinalStateStashed() {
    let (session, _, _) = makeSession()
    session.initialize()
    #expect(session.donedataAtFinal.isEmpty)
    session.engine.stashDonedataAtFinal("{\"n\":1}")
    #expect(session.donedataAtFinal == "{\"n\":1}")
}

@Test func stoppingASessionRunsTheExitOfEveryStateItIsIn() {
    let (session, policy, _) = makeSession()
    session.initialize()
    session.raiseExternalByName("go", eventData: "")
    session.step()
    policy.log.removeAll()
    session.stop()
    // §scxml-D-exitInterpreter: a run its host stops is exited as one that ended.
    #expect(policy.log == ["exit busy"])
}

@Test func aSessionAnswersTheDeadlineOfItsOwnScheduler() {
    let (session, _, _) = makeSession()
    session.initialize()
    #expect(session.timeUntilNextScheduled() == nil)
}
