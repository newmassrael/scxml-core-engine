// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// The loop runs a procedure from its initial state: it enters a state, handles the event the
// entry answered, takes the transition, and stops at a final state, at a state with no
// transition for the event, or at the bound. The machine below is written the way a generated
// one is, small enough to read in full.

private final class Ping: ProcedureStateMachine {
    enum State { case ask, retry, done, failed }
    enum Event { case noEvent, ok, fail, errorExecution }

    var serviceHandler: ProcedureServiceHandler?
    var doneData: [String: String] = [:]
    var pendingEventData = ""

    /// How many times the retry state was entered, and what `assign` should raise.
    var retries = 0
    var raiseOnAssign = false
    var entered: [State] = []

    func initialState() -> State { .ask }

    func isFinal(_ state: State) -> Bool { state == .done || state == .failed }

    func finalStateName(_ state: State) -> String {
        if state == .done { return "done" }
        if state == .failed { return "failed" }
        return ""
    }

    func executeEntryActions(_ state: State) -> (event: Event, data: String) {
        entered.append(state)
        if state == .ask, let handler = serviceHandler {
            let answer = handler(ProcedureServiceRequest(service: "Ping", addr: "7"))
            return (answer.success ? .ok : .fail, answer.data)
        }
        if state == .retry { retries += 1 }
        if state == .done { doneData["result"] = "success" }
        return (.noEvent, "")
    }

    func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>? {
        if state == .ask {
            if event == .ok { return ProcedureTransition(target: .done, index: 0, hasAssigns: raiseOnAssign) }
            if event == .fail { return ProcedureTransition(target: .retry, index: 1, hasAssigns: false) }
        }
        if state == .retry {
            if event == .noEvent {
                return ProcedureTransition(target: retries < 3 ? .ask : .failed, index: 0, hasAssigns: false)
            }
        }
        return nil
    }

    func executeTransitionActions(_ source: State, _ index: Int) -> Event? {
        raiseOnAssign ? .errorExecution : nil
    }
}

@Test func aProcedureRunsFromItsInitialStateToAFinalOne() {
    let procedure = Ping()
    procedure.serviceHandler = { _ in ProcedureServiceResponse(success: true, data: "pong") }
    let result = procedure.runToCompletion()
    #expect(result == ProcedureRunResult(completed: true, finalState: "done", doneData: ["result": "success"]))
    #expect(procedure.pendingEventData == "pong")
}

@Test func theServiceReceivesTheRequestTheEntryActionBuilt() {
    let procedure = Ping()
    var seen: [ProcedureServiceRequest] = []
    procedure.serviceHandler = { request in
        seen.append(request)
        return ProcedureServiceResponse(success: true)
    }
    _ = procedure.runToCompletion()
    #expect(seen == [ProcedureServiceRequest(service: "Ping", addr: "7")])
    #expect(seen.first?.subfunc == nil, "an attribute the document does not write is absent, not empty")
    #expect(seen.first?.payload == nil)
}

@Test func aFailedServiceTakesTheFailTransitionAndTheProcedureMayTryAgain() {
    let procedure = Ping()
    var calls = 0
    procedure.serviceHandler = { _ in
        calls += 1
        return ProcedureServiceResponse(success: calls == 3)
    }
    let result = procedure.runToCompletion()
    #expect(result.completed)
    #expect(result.finalState == "done")
    #expect(calls == 3)
    #expect(procedure.retries == 2)
}

@Test func aProcedureThatNeverSucceedsEndsInItsFailureState() {
    let procedure = Ping()
    procedure.serviceHandler = { _ in ProcedureServiceResponse(success: false) }
    let result = procedure.runToCompletion()
    #expect(result == ProcedureRunResult(completed: true, finalState: "failed", doneData: [:]))
}

@Test func aProcedureWithNoTransitionForTheEventEndsShortOfAFinalState() {
    // No handler is set, so the entry answers no event, and `ask` takes none.
    let procedure = Ping()
    let result = procedure.runToCompletion()
    #expect(result == ProcedureRunResult(completed: false, finalState: "", doneData: [:]))
}

@Test func anEventRaisedByAnAssignIsHandledInTheSourceStateAndTakesNoTransition() {
    let procedure = Ping()
    procedure.serviceHandler = { _ in ProcedureServiceResponse(success: true) }
    procedure.raiseOnAssign = true
    let result = procedure.runToCompletion()
    // `ask` has no transition for the raised event, so the run ends where it was.
    #expect(!result.completed)
    #expect(procedure.entered == [.ask])
}

@Test func theDataOfAnEntryIsKeptOnlyWhereItIsNotEmpty() {
    let procedure = Ping()
    procedure.pendingEventData = "kept"
    procedure.serviceHandler = { _ in ProcedureServiceResponse(success: true, data: "") }
    _ = procedure.runToCompletion()
    #expect(procedure.pendingEventData == "kept")
}
