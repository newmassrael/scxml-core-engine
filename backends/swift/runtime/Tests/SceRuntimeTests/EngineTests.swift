// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// The engine, driven through a policy the test scripts: what a macrostep takes and
// leaves, what the scheduler holds, and what the engine counts of what it declined.

// MARK: - Macrostep and queues

@Test func anEntryThatRaisesAnEventIsTakenInTheSameMacrostep() {
    let (engine, policy, _) = makeEngine { policy in
        policy.onEntry[.idle] = { $0.raise(EventWithMetadata(.go)) }
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .busy)]
    }
    engine.initialize()
    #expect(engine.getCurrentState() == .busy)
    #expect(policy.log == ["enter idle", "exit idle", "enter busy"])
    #expect(engine.isRunning)
}

@Test func anInternalEventIsTakenBeforeAnExternalOneThatWasQueuedFirst() {
    let (engine, policy, _) = makeEngine { policy in
        policy.onEntry[.idle] = { $0.raise(EventWithMetadata(.ping)) }
        policy.transitions[.idle] = [
            FlatTransition(event: .go, target: .busy),
            FlatTransition(event: .ping, target: nil, content: { _ in }),
        ]
    }
    engine.raiseExternal(.go)
    engine.initialize()
    // §scxml-C-1: the internal ping runs its targetless transition first; only then
    // is the external go taken.
    #expect(policy.log == ["enter idle", "content idle#1", "exit idle", "enter busy"])
}

@Test func aTargetlessTransitionRunsItsContentAndLeavesTheConfigurationAlone() {
    let (engine, policy, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .ping, target: nil, content: { _ in })]
    }
    engine.initialize()
    engine.processEvent(.ping)
    #expect(engine.getCurrentState() == .idle)
    #expect(policy.log == ["enter idle", "content idle#0"])
    // It was taken, so it is no discard.
    #expect(engine.discardedExternalEvents == 0)
}

@Test func anEventNoTransitionMatchesIsDiscardedAndCounted() {
    let (engine, _, _) = makeEngine { _ in }
    engine.initialize()
    engine.processEvent(.ping)
    engine.processEvent(.again)
    #expect(engine.discardedExternalEvents == 2)
    #expect(engine.lastDiscardedEvent == .again)
}

@Test func anEventHandedToAMachineThatHasEndedIsCountedAsUnseen() {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .done)]
    }
    engine.initialize()
    engine.processEvent(.go)
    #expect(engine.isInFinalState)
    #expect(engine.terminalState == .done)
    engine.processEvent(.ping)
    engine.processEvent(.again)
    #expect(engine.unseenExternalEvents == 2)
    #expect(engine.lastUnseenEvent == .again)
    #expect(engine.discardedExternalEvents == 0)
}

@Test func eventsQueuedWhenTheRunEndsAreCountedOnceAndDrained() {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .done)]
    }
    engine.initialize()
    engine.raiseExternal(.go)
    engine.raiseExternal(.ping)
    engine.raiseExternal(.again)
    engine.step()
    #expect(engine.unseenExternalEvents == 2)
    // A host that keeps stepping a halted machine does not re-count the same queue.
    engine.step()
    engine.step()
    #expect(engine.unseenExternalEvents == 2)
}

@Test func theCompletionCallbackFiresWhenTheRunEntersATopLevelFinal() {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .done)]
    }
    var completions = 0
    engine.setCompletionCallback { completions += 1 }
    engine.initialize()
    #expect(completions == 0)
    engine.processEvent(.go)
    #expect(completions == 1)
    #expect(engine.getActiveStates().isEmpty, "§scxml-D-exitInterpreter leaves the configuration empty")
}

@Test func aStoppedMachineExitsEveryStateItWasInAndTakesNoMoreEvents() {
    let (engine, policy, _) = makeEngine { _ in }
    engine.initialize()
    engine.stop()
    #expect(policy.log == ["enter idle", "exit idle"])
    #expect(!engine.isRunning)
    #expect(engine.getActiveStates().isEmpty)
    engine.processEvent(.go)
    #expect(engine.unseenExternalEvents == 1)
    // A second stop exits nothing again.
    engine.stop()
    #expect(policy.log == ["enter idle", "exit idle"])
}

@Test func eventlessTransitionsAreTakenAfterEveryMicrostep() {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .null, target: .busy)]
        policy.transitions[.busy] = [FlatTransition(event: .null, target: .done)]
    }
    engine.initialize()
    #expect(engine.getCurrentState() == .done)
    #expect(engine.isInFinalState)
}

@Test func aGuardThatIsFalseLeavesTheTransitionUntaken() {
    var open = false
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .busy, guardHolds: { open })]
    }
    engine.initialize()
    engine.processEvent(.go)
    #expect(engine.getCurrentState() == .idle)
    open = true
    engine.processEvent(.go)
    #expect(engine.getCurrentState() == .busy)
}

// MARK: - The ceilings

@Test func aCyclicEventlessChainIsStoppedAndCountedAndTheMachineStaysReachable() {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(event: .null, target: .busy),
            FlatTransition(event: .go, target: .done),
        ]
        policy.transitions[.busy] = [FlatTransition(event: .null, target: .idle)]
    }
    engine.initialize()
    #expect(engine.truncatedMacrosteps == 1)
    #expect(engine.lastTruncatedMacrostepState != nil)
    #expect(engine.isRunning)
    // The event that rescues it is on the external queue and is still taken.
    engine.raiseExternal(.go)
    engine.step()
    // Each event the host gives it is a macrostep of its own with a budget of its
    // own: the chain is refused again and counted again.
    #expect(engine.truncatedMacrosteps >= 1)
}

@Test func aRaiseThatAnswersItselfIsStoppedAtTheMicrostepCeiling() {
    let (engine, _, _) = makeEngine { policy in
        policy.onEntry[.idle] = { $0.raise(EventWithMetadata(.again)) }
        policy.transitions[.idle] = [
            FlatTransition(
                event: .again,
                target: .idle,
                content: { $0.raise(EventWithMetadata(.again)) }
            )
        ]
    }
    engine.initialize()
    #expect(engine.truncatedMacrosteps == 1)
    #expect(engine.isRunning)
}

@Test func aMachineThatSendsItselfTheNextExternalEventHandsTheCallBackAtTheBudget() throws {
    let (engine, _, _) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(
                event: .again,
                target: nil,
                content: { $0.raiseExternal(.again) }
            )
        ]
    }
    try engine.setMaxExternalEventsPerCall(5)
    engine.initialize()
    engine.raiseExternal(.again)
    engine.step()
    #expect(engine.truncatedEventChains == 1)
    #expect(engine.lastTruncatedEvent == .again)
    // The events left queued stay queued: the next call gets another budget.
    engine.step()
    #expect(engine.truncatedEventChains == 2)
}

@Test func aBudgetThatTakesNoEventIsRefusedAndChangesNothing() throws {
    let (engine, _, _) = makeEngine { _ in }
    #expect(engine.maxExternalEventsPerCall == 10000)
    #expect(throws: EngineError.self) { try engine.setMaxExternalEventsPerCall(0) }
    #expect(throws: EngineError.self) { try engine.setMaxExternalEventsPerCall(-3) }
    #expect(throws: EngineError.self) { try engine.setMaxExternalEventsPerCall(Int(UInt32.max) + 1) }
    #expect(engine.maxExternalEventsPerCall == 10000)
    try engine.setMaxExternalEventsPerCall(7)
    #expect(engine.maxExternalEventsPerCall == 7)
}

@Test func anErrorHandlerThatKeepsFailingTheSameWayIsCutAtTheCascadeCeiling() {
    let (engine, policy, _) = makeEngine { policy in
        policy.onEntry[.idle] = { engine in
            engine.raise(
                .platformError(.errorExecution, message: "first failure")
            )
        }
        policy.transitions[.idle] = [
            FlatTransition(
                event: .errorExecution,
                target: nil,
                content: { engine in
                    engine.raise(.platformError(.errorExecution, message: "the handler failed too"))
                }
            )
        ]
    }
    engine.initialize()
    #expect(engine.errorCascadeEvents >= 1)
    #expect(engine.lastErrorCascadeEvent == .errorExecution)
    // The chain stopped at the cascade ceiling and not at the microstep one.
    #expect(engine.truncatedMacrosteps == 0)
    #expect(policy.log.count < 400)
}

@Test func anErrorNoTransitionAnswersIsCountedAsUnhandled() {
    let (engine, _, _) = makeEngine { policy in
        policy.onEntry[.idle] = { $0.raise(.platformError(.errorCommunication, message: "nobody there")) }
    }
    engine.initialize()
    #expect(engine.unhandledErrorEvents == 1)
    #expect(engine.lastUnhandledError == .errorCommunication)
}

@Test func aRaisedEventThatMatchesNothingIsTheDocumentsOwnBusinessAndIsNotCounted() {
    let (engine, _, _) = makeEngine { policy in
        policy.onEntry[.idle] = { $0.raise(EventWithMetadata(.ping)) }
    }
    engine.initialize()
    #expect(engine.unhandledErrorEvents == 0)
    #expect(engine.discardedExternalEvents == 0)
}

// MARK: - Names

@Test func anEventArrivingByNameIsDeliveredAsTheLongestPrefixTheDocumentWrites() {
    let (engine, _, _) = makeEngine { _ in }
    #expect(engine.resolveEventByName("request.new") == .requestNew)
    #expect(engine.resolveEventByName("request") == .request)
    // Not written, so cut back to the longest token prefix that is.
    #expect(engine.resolveEventByName("request.new.later") == .requestNew)
    #expect(engine.resolveEventByName("request.other") == .request)
    // Token boundaries hold: no event of the document is a prefix of this name, so
    // it falls through to the wildcard member.
    #expect(engine.resolveEventByName("requests") == .wildcard)
    #expect(engine.resolveEventByName("nothing.like.it") == .wildcard)
}

@Test func theArrivalNameIsKeptOnlyWhenTheMemberIsNotCalledThat() {
    let (engine, _, _) = makeEngine { _ in }
    let kept = engine.arrivingEvent(name: "request.other", metadata: EventMetadata())
    #expect(kept?.event == .request)
    #expect(kept?.metadata.name == "request.other")
    let exact = engine.arrivingEvent(name: "request", metadata: EventMetadata(name: "stale"))
    #expect(exact?.metadata.name == "", "a name matched against another machine's says nothing here")
}

@Test func aNameNoTransitionCouldMatchIsDropped() {
    // A policy with no wildcard member: remove it from the table by asking a name the
    // table cannot resolve once the wildcard is not there.
    let (engine, policy, _) = makeEngine { _ in }
    #expect(policy.getEventFromName("*") == .wildcard)
    engine.raiseExternalByName("unknown.name")
    // With the wildcard member present it is delivered as that member.
    #expect(engine.arrivingEvent(name: "unknown.name", metadata: EventMetadata())?.event == .wildcard)
}

@Test func anExternalEventByNameCarriesTheNameItArrivedUnder() {
    let (engine, policy, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .requestNew, target: nil, content: { _ in })]
    }
    engine.initialize()
    engine.raiseExternalByName("request.new.later", eventData: "{}")
    engine.step()
    #expect(policy.log.last == "content idle#0")
}

// MARK: - The scheduler

@Test func aDelayedSendComesDueWhenTheHostMovesTheClockPastIt() {
    let (engine, policy, clock) = makeEngine { policy in
        policy.onEntry[.idle] = { engine in
            engine.scheduleEvent(.go, delay: .milliseconds(100), sendID: "s1", eventData: "", origin: "")
        }
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .busy)]
    }
    engine.initialize()
    #expect(engine.timeUntilNextScheduled() == .milliseconds(100))
    engine.advanceTimeMs(99)
    #expect(engine.getCurrentState() == .idle)
    #expect(engine.timeUntilNextScheduled() == .milliseconds(1))
    engine.advanceTimeMs(1)
    #expect(engine.getCurrentState() == .busy)
    #expect(clock.elapsedMs() == 100)
    #expect(engine.timeUntilNextScheduled() == nil)
    #expect(policy.log.last == "enter busy")
}

@Test func aCancelledSendIsNeverDelivered() {
    let (engine, _, _) = makeEngine { policy in
        policy.onEntry[.idle] = { engine in
            engine.scheduleEvent(.go, delay: .milliseconds(50), sendID: "late", eventData: "", origin: "")
        }
        policy.transitions[.idle] = [
            FlatTransition(event: .go, target: .busy),
            FlatTransition(event: .ping, target: nil, content: { _ in _ = 0 }),
        ]
    }
    engine.initialize()
    #expect(engine.cancelEvent(sendID: "late"))
    #expect(!engine.cancelEvent(sendID: "late"))
    engine.advanceTimeMs(1000)
    #expect(engine.getCurrentState() == .idle)
}

@Test func twoSendsArmedByOneEntryShareOneInstantAndFireInDeadlineOrder() {
    let (engine, policy, clock) = makeEngine { policy in
        policy.onEntry[.idle] = { engine in
            engine.scheduleEvent(.ping, delay: .milliseconds(20), sendID: "second", eventData: "", origin: "")
            engine.scheduleEvent(.go, delay: .milliseconds(10), sendID: "first", eventData: "", origin: "")
        }
        policy.transitions[.idle] = [
            FlatTransition(event: .go, target: nil, content: { _ in }),
            FlatTransition(event: .ping, target: nil, content: { _ in }),
        ]
    }
    engine.initialize()
    clock.advance(50)
    engine.tick()
    // Earliest deadline first, one macrostep apart.
    #expect(policy.log.suffix(2) == ["content idle#0", "content idle#1"])
}

@Test func aSendWithNoIdIsGivenOneThatCanBeCancelled() {
    let (engine, _, _) = makeEngine { _ in }
    engine.initialize()
    let id = engine.scheduleEvent(.go, delay: .milliseconds(5), sendID: "", eventData: "", origin: "")
    #expect(id == "auto_send_1")
    #expect(engine.cancelEvent(sendID: id))
}

@Test func aDelayedSendKeepsTheOriginAndTheNameItWasSentUnder() {
    let (engine, policy, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .request, target: nil, content: { _ in })]
    }
    engine.initialize()
    engine.sendNamedExternal(
        "request.later",
        delay: .milliseconds(10),
        sendID: "n1",
        eventData: "payload",
        origin: "sce://scxml/sender"
    )
    #expect(engine.hasReadyEvents() == false)
    engine.advanceTimeMs(10)
    #expect(policy.log.last == "content idle#0")
}

@Test func theNextAutoSendIdCountsPerMachineFromOne() {
    let (first, _, _) = makeEngine { _ in }
    let (second, _, _) = makeEngine { _ in }
    #expect(first.nextAutoSendID() == "_auto_send_1")
    #expect(first.nextAutoSendID() == "_auto_send_2")
    #expect(second.nextAutoSendID() == "_auto_send_1")
}

@Test func aClockMayNotBeSwappedOnceTheMachineHasStarted() {
    let (engine, _, clock) = makeEngine { _ in }
    #expect(engine.nowMs() == 0)
    clock.advance(30)
    #expect(engine.nowMs() == 30)
    #expect(engine.clock() as? ManualClock === clock)
}

// MARK: - Sends to a target value

@Test func aSendToTheMachinesOwnExternalQueueIsDeliveredAndATargetNoOneCanAddressIsReported() {
    let (engine, policy, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .busy)]
    }
    engine.initialize()
    let sent = engine.sendToTarget(
        event: .go,
        eventName: "go",
        target: "#_scxml_me",
        ownSessionID: "me",
        delay: .zero,
        sendID: "x",
        eventData: "",
        origin: ""
    )
    #expect(sent == .sent)
    engine.step()
    #expect(engine.getCurrentState() == .busy)

    func outcome(_ target: String) -> TargetSendOutcome {
        engine.sendToTarget(
            event: nil,
            eventName: "go",
            target: target,
            ownSessionID: "me",
            delay: .zero,
            sendID: "",
            eventData: "",
            origin: ""
        )
    }
    #expect(outcome("") == .notReachable)
    #expect(outcome("bogus") == .notSupported)
    #expect(outcome("#peer") == .notSupported, "a Mesh peer is not this processor's to address")
    #expect(outcome("#_parent") == .notReachable, "a session nothing invoked has no parent")
    #expect(outcome("#_gone") == .notReachable)
    #expect(outcome("#_scxml_other") == .notReachable)
    _ = policy
}

@Test func aSendToTheParentReachesItOnlyWhereThereIsOne() {
    let (engine, policy, _) = makeEngine { policy in
        policy.hasParent = true
    }
    engine.initialize()
    let outcome = engine.sendToTarget(
        event: nil,
        eventName: "done.child",
        target: "#_parent",
        ownSessionID: "me",
        delay: .zero,
        sendID: "",
        eventData: "{}",
        origin: ""
    )
    #expect(outcome == .sent)
    #expect(policy.parentDeliveries.map(\.name) == ["done.child"])
    #expect(policy.parentDeliveries.map(\.data) == ["{}"])
}

@Test func aSendToARunningInvocationOrChildSessionIsDeliveredAndTheChildrenSettled() {
    let (engine, policy, _) = makeEngine { policy in
        policy.invocationsRunning = ["worker"]
        policy.sessionsRunning = ["kid"]
    }
    engine.initialize()
    func send(_ target: String) -> TargetSendOutcome {
        engine.sendToTarget(
            event: nil,
            eventName: "tell",
            target: target,
            ownSessionID: "me",
            delay: .zero,
            sendID: "s",
            eventData: "",
            origin: ""
        )
    }
    #expect(send("#_worker") == .sent)
    #expect(send("#_scxml_kid") == .sent)
    #expect(policy.invocationDeliveries.map(\.invokeID) == ["worker"])
    #expect(policy.sessionDeliveries.map(\.sessionID) == ["kid"])
    #expect(policy.settledChildren == 2)
}

@Test func aDelayedSendToAnInvocationThatEndedInTheMeantimeIsReportedWhenItComesDue() {
    let (engine, policy, _) = makeEngine { policy in
        policy.invocationsRunning = ["worker"]
        policy.transitions[.idle] = [
            FlatTransition(event: .errorCommunication, target: .busy)
        ]
    }
    engine.initialize()
    let outcome = engine.sendToTarget(
        event: nil,
        eventName: "tell",
        target: "#_worker",
        ownSessionID: "me",
        delay: .milliseconds(10),
        sendID: "late",
        eventData: "",
        origin: ""
    )
    #expect(outcome == .sent)
    policy.invocationsRunning = []
    engine.advanceTimeMs(10)
    // §scxml-C-1: error.communication on the internal queue of the sender, taken in
    // the same macrostep.
    #expect(engine.getCurrentState() == .busy)
}

@Test func aSendToTheInternalQueueIsAnInternalEventAndKeepsItsPlaceAheadOfTheExternalQueue() {
    let (engine, policy, _) = makeEngine { policy in
        policy.transitions[.idle] = [
            FlatTransition(event: .go, target: nil, content: { _ in }),
            FlatTransition(event: .ping, target: nil, content: { _ in }),
        ]
    }
    engine.initialize()
    engine.raiseExternal(.ping)
    let outcome = engine.sendToTarget(
        event: .go,
        eventName: "go",
        target: "#_internal",
        ownSessionID: "me",
        delay: .zero,
        sendID: "",
        eventData: "",
        origin: ""
    )
    #expect(outcome == .sent)
    engine.step()
    #expect(policy.log.suffix(2) == ["content idle#0", "content idle#1"])
}

// MARK: - Late binding, the root's refusal

@Test func aStatesDataIsBoundOnItsFirstEntryOfARunAndNeverAgain() {
    let (engine, _, _) = makeEngine { _ in }
    #expect(engine.claimLateBindingFirstEntry(.busy))
    #expect(!engine.claimLateBindingFirstEntry(.busy))
    #expect(engine.claimLateBindingFirstEntry(.idle))
    // A new run binds again.
    engine.initialize()
    #expect(engine.claimLateBindingFirstEntry(.busy))
}

private final class ParentNeedingPolicy {}

@Test func aMachineThatSendsToItsParentRefusesToStartAsARoot() {
    final class NeedsParent: StatePolicy {
        typealias State = FlatState
        typealias Event = FlatEvent
        let inner = ScriptedPolicy()
        var needsParent: Bool { true }
        func initialState() -> FlatState { inner.initialState() }
        func isFinalState(_ state: FlatState) -> Bool { inner.isFinalState(state) }
        func getParent(_ state: FlatState) -> FlatState? { nil }
        func isCompoundState(_ state: FlatState) -> Bool { false }
        func isParallelState(_ state: FlatState) -> Bool { false }
        func getChildStates(_ state: FlatState) -> [FlatState] { [] }
        func getInitialTargets(_ state: FlatState) -> [EntryTarget<FlatState>] { [] }
        func getDocumentInitialTargets() -> [EntryTarget<FlatState>] { [.state(.idle)] }
        func getHistoryParent(_ history: HistoryID) -> FlatState { .idle }
        func getHistoryDefaultTargets(_ history: HistoryID) -> [EntryTarget<FlatState>] { [] }
        func getDocumentOrder(_ state: FlatState) -> Int { state.rawValue }
        func getEventName(_ event: FlatEvent) -> String { event.name }
        func getEventFromName(_ name: String) -> FlatEvent? { inner.getEventFromName(name) }
        func getStateName(_ state: FlatState) -> String { state.name }
        func getStateFromName(_ name: String) -> FlatState? { inner.getStateFromName(name) }
        func nullEvent() -> FlatEvent { .null }
        func historyValue(_ history: HistoryID) -> [FlatState]? { nil }
        func bindCurrentEvent(_ event: FlatEvent, engine: Engine<NeedsParent>) {}
        func firstEnabledTransition(
            _ state: FlatState,
            event: FlatEvent,
            engine: Engine<NeedsParent>
        ) -> EnabledTransition<FlatState>? { nil }
        func executeTransitionContent(source: FlatState, transitionIndex: Int, engine: Engine<NeedsParent>) {}
        func executeEntryActions(_ state: FlatState, engine: Engine<NeedsParent>, isDefaultEntry: Bool) {}
        func executeExitActions(_ state: FlatState, engine: Engine<NeedsParent>, configurationBeforeExit: [FlatState]) {}
        func executeHistoryDefaultContent(_ history: HistoryID, engine: Engine<NeedsParent>) {}
        var hasParallelStates: Bool { false }
        var needsScriptEngine: Bool { false }
        var needsDataModelInit: Bool { false }
        var hasInvokeSupport: Bool { false }
        var hasFinalize: Bool { false }
        var hasAutoforward: Bool { false }
        var hasActiveStates: Bool { false }
        var hasChildTick: Bool { false }
    }
    let engine = Engine(policy: NeedsParent())
    #expect(engine.rootStartRefusal() == .needsParent)
    #expect(throws: RootStartRefusal.needsParent) { try engine.initializeAsRoot() }
    #expect(!engine.isRunning, "a refusal starts nothing")
    #expect(RootStartRefusal.needsParent.description == "the machine sends to #_parent and was started with no parent session")

    let (plain, _, _) = makeEngine { _ in }
    #expect(plain.rootStartRefusal() == nil)
    #expect(throws: Never.self) { try plain.initializeAsRoot() }
    #expect(plain.isRunning)
}

// MARK: - Payload readings and the HTTP callback

@Test func onlyAnUndecodablePayloadReadingIsCounted() {
    let (engine, _, _) = makeEngine { _ in }
    for reading in [PayloadReading.absent, .dom, .structured, .text] {
        engine.notePayloadReading(.go, reading: reading)
    }
    #expect(engine.undecodablePayloads == 0)
    engine.notePayloadReading(.ping, reading: .undecodable)
    #expect(engine.undecodablePayloads == 1)
    #expect(engine.lastUndecodablePayload == .ping)
}

@Test func anHttpSendIsHandedToTheCallbackAndItsResponseBecomesAnExternalEvent() {
    let (engine, policy, _) = makeEngine { policy in
        policy.transitions[.idle] = [FlatTransition(event: .requestNew, target: nil, content: { _ in })]
    }
    var seen: [HttpSendRequest] = []
    engine.setHTTPSendCallback { request in
        seen.append(request)
        return HttpSendResponse(eventName: "request.new", eventData: "answer")
    }
    engine.initialize()
    engine.performHTTPSend(
        target: "http://localhost/x",
        eventName: "go",
        content: "body",
        params: ["a": ["1", "2"]],
        sendID: "h1"
    )
    #expect(seen.map(\.target) == ["http://localhost/x"])
    #expect(seen.first?.params["a"] == ["1", "2"])
    engine.step()
    #expect(policy.log.last == "content idle#0")

    // Fire-and-forget: no callback, no effect.
    engine.setHTTPSendCallback(nil)
    engine.performHTTPSend(target: "http://localhost/x", eventName: "go", content: "", params: [:], sendID: "")
}

// MARK: - The donedata stash

@Test func theDonedataAFinalStashedIsWhatAParentReadsBack() {
    let (engine, _, _) = makeEngine { policy in
        policy.onEntry[.done] = { $0.stashDonedataAtFinal(#"{"k":1}"#) }
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .done)]
    }
    engine.initialize()
    #expect(engine.donedataAtFinal == "")
    engine.processEvent(.go)
    #expect(engine.donedataAtFinal == #"{"k":1}"#)
}

@Test func theMachineRunsUntilCompletionWhenAScheduledEventEndsIt() {
    let (engine, _, clock) = makeEngine { policy in
        policy.onEntry[.idle] = { engine in
            engine.scheduleEvent(.go, delay: .milliseconds(5), sendID: "end", eventData: "", origin: "")
        }
        policy.transitions[.idle] = [FlatTransition(event: .go, target: .done)]
    }
    engine.initialize()
    clock.advance(5)
    #expect(engine.runUntilCompletion(timeout: .seconds(5), pollInterval: .milliseconds(1)))
    #expect(engine.isInFinalState)
}
