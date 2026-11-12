// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// docs/adr/0096, the engine's half: a start whose script session cannot be established is refused
// before anything is entered, whichever door the host started it through, and the host is handed
// the failure in the words every backend writes. The policy is a scripted one whose session
// answers what the test says; the generated machine over a real Lua state is held in
// `AMachineWhoseScriptSessionFailsDoesNotStartTests` of the tests package.

private let failure = ScriptSessionFailure(stage: .library, message: "the semantics chunk did not load")

@Test func aFailureNamesItsStepInTheWordsEveryBackendWrites() {
    #expect(ScriptSessionStage.allCases.map(\.description) == ["create", "library", "system-variables"])
    #expect(
        failure.description
            == "the script session could not be established at step 'library': the semantics chunk did not load")
    #expect(
        failure.childStartRefusal(invokeID: "chatty")
            == "<invoke id='chatty'> did not start: its child's script session failed at step 'library'")
    #expect(RootStartRefusal.scriptSession(failure).description == failure.description)
    #expect(RootStartRefusal.scriptSession(failure) != .needsParent)
    #expect(ConfigurationRejection.scriptSessionUnavailable(failure).description == failure.description)
    #expect(ConfigurationRejection.scriptSessionUnavailable(failure) != .accepted)
}

/// What a refused start leaves: no state entered, no `<onentry>` run, no run marked running.
private func expectNothingStarted(_ engine: Engine<ScriptedPolicy>, _ policy: ScriptedPolicy) {
    #expect(!engine.isRunning, "a refused start marks no run running")
    #expect(engine.getActiveStates().isEmpty, "a refused start enters no state")
    #expect(engine.terminalState == nil, "a refused start reaches no final state")
    #expect(policy.log.isEmpty, "a refused start runs no executable content: \(policy.log)")
}

@Test func aStartWhoseSessionCannotBeEstablishedThrowsTheFailureAndEntersNothing() {
    let (engine, policy, _) = makeEngine { $0.sessionFailure = failure }
    #expect(throws: failure) { try engine.initialize() }
    #expect(policy.sessionsAsked == 1, "the start asked for its session once")
    expectNothingStarted(engine, policy)

    // The machine was not started, so an event the host hands it is not taken.
    engine.processEvent(.go)
    #expect(engine.unseenExternalEvents == 1)
    expectNothingStarted(engine, policy)

    // A session that failed is not taken as made: the next start asks again, and is refused again.
    #expect(throws: failure) { try engine.initialize() }
    #expect(policy.sessionsAsked == 2)
    expectNothingStarted(engine, policy)
}

@Test func aStartWhoseSessionIsEstablishedRunsAsBefore() throws {
    let (engine, policy, _) = makeEngine { _ in }
    try engine.initialize()
    #expect(policy.sessionsAsked == 1, "every start establishes the session, once")
    #expect(engine.isRunning)
    #expect(policy.log == ["enter idle"])
}

@Test func theCheckedRootStartRefusesForTheParentFirstAndThenForTheSession() {
    let (forParent, parentPolicy, _) = makeEngine {
        $0.needsParent = true
        $0.sessionFailure = failure
    }
    #expect(throws: RootStartRefusal.needsParent) { try forParent.initializeAsRoot() }
    #expect(parentPolicy.sessionsAsked == 0, "a machine refused for its parent establishes nothing")
    expectNothingStarted(forParent, parentPolicy)

    let (forSession, sessionPolicy, _) = makeEngine { $0.sessionFailure = failure }
    #expect(throws: RootStartRefusal.scriptSession(failure)) { try forSession.initializeAsRoot() }
    expectNothingStarted(forSession, sessionPolicy)
}

@Test func aResumedStartWhoseSessionCannotBeEstablishedIsRefusedWithTheFailure() {
    let (engine, policy, _) = makeEngine { $0.sessionFailure = failure }
    #expect(engine.enterAt(configuration: [.busy], current: .busy) == .scriptSessionUnavailable(failure))
    #expect(engine.getCurrentState() == .idle, "the refused configuration is not entered")
    expectNothingStarted(engine, policy)

    // A configuration that is not one of the document is refused for that, before a session is
    // asked for.
    #expect(engine.enterAt(configuration: [], current: .busy) == .empty)
    #expect(policy.sessionsAsked == 1)
}

@Test func aChildWhoseStartIsRefusedIsAFailedInvokeOfItsParent() throws {
    var raised: [String] = []
    let (parent, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(
                event: .errorExecution, target: nil,
                content: { engine in raised.append(engine.policy.eventData) })
        ]
    }
    try parent.initialize()

    let childPolicy = ScriptedPolicy()
    childPolicy.sessionFailure = failure
    let child = InvokedSession(policy: childPolicy, sessionID: "parent.probe", queue: ParentEventQueue())
    #expect(!startChildSession(child, invokeID: "probe", engine: parent), "the refused child did not start")
    #expect(!child.engine.isRunning)
    #expect(childPolicy.log.isEmpty, "the refused child ran nothing")

    parent.step()
    #expect(raised == [failure.childStartRefusal(invokeID: "probe")], "the parent raised error.execution")

    let startedPolicy = ScriptedPolicy()
    let started = InvokedSession(policy: startedPolicy, sessionID: "parent.other", queue: ParentEventQueue())
    #expect(startChildSession(started, invokeID: "other", engine: parent), "a child whose session is there starts")
    #expect(started.engine.isRunning)
    parent.step()
    #expect(raised.count == 1, "a child that started raises nothing")
}
