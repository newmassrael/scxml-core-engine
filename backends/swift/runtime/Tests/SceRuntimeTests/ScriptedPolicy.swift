// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

/// The states of the flat machine the engine tests drive: three siblings under the
/// `<scxml>` element, the last a top-level `<final>`.
enum FlatState: Int, Hashable, CaseIterable {
    case idle, busy, done

    var name: String {
        ["idle", "busy", "done"][rawValue]
    }
}

/// The events of the flat machine. `wildcard` is the member `event="*"` stands for,
/// which the exact table keeps under the name `*`.
enum FlatEvent: Hashable, CaseIterable {
    case null, go, again, ping, request, requestNew, errorExecution, errorCommunication, wildcard
    case doneInvokeProbe, errorInvokeProbe

    var name: String {
        switch self {
        case .doneInvokeProbe: return "done.invoke.probe"
        case .errorInvokeProbe: return "error.invoke.probe"
        case .null: return ""
        case .go: return "go"
        case .again: return "again"
        case .ping: return "ping"
        case .request: return "request"
        case .requestNew: return "request.new"
        case .errorExecution: return "error.execution"
        case .errorCommunication: return "error.communication"
        case .wildcard: return "*"
        }
    }
}

/// One transition of the flat machine: the event it answers, where it goes (nil for a
/// targetless one), whether it is internal, a guard, and the content it runs.
struct FlatTransition {
    var event: FlatEvent
    var target: FlatState?
    var guardHolds: () -> Bool = { true }
    var content: ((Engine<ScriptedPolicy>) -> Void)?
}

/// A policy for a flat machine whose behavior the test scripts: what each state's
/// `<onentry>` and `<onexit>` do, and which transitions it has. Everything the
/// document would otherwise fix is a closure the test owns.
final class ScriptedPolicy: InvocablePolicy {
    typealias State = FlatState
    typealias Event = FlatEvent

    /// What a parent gives the session it invokes (`InvokedSession`).
    var parentEventQueue: ParentEventQueue?
    var sessionID = ""

    var transitions: [FlatState: [FlatTransition]] = [:]
    var onEntry: [FlatState: (Engine<ScriptedPolicy>) -> Void] = [:]
    var onExit: [FlatState: (Engine<ScriptedPolicy>) -> Void] = [:]
    var active: [FlatState] = []
    var log: [String] = []

    /// What the parent-delivery hooks answer, set by a test that plays a child.
    var parentDeliveries: [(name: String, data: String)] = []
    var hasParent = false

    /// The ids of the `<invoke>`s a host runs, for a test that plays a document with some.
    var hostInvokeIDs: [String] = []

    var invocationsRunning: Set<String> = []
    var invocationDeliveries: [(invokeID: String, name: String)] = []
    var sessionsRunning: Set<String> = []
    var sessionDeliveries: [(sessionID: String, name: String)] = []
    var settledChildren = 0

    func initialState() -> FlatState { .idle }

    func isFinalState(_ state: FlatState) -> Bool { state == .done }

    func getParent(_ state: FlatState) -> FlatState? { nil }

    func isCompoundState(_ state: FlatState) -> Bool { false }

    func isParallelState(_ state: FlatState) -> Bool { false }

    func getChildStates(_ state: FlatState) -> [FlatState] { [] }

    func getInitialTargets(_ state: FlatState) -> [EntryTarget<FlatState>] { [] }

    func getDocumentInitialTargets() -> [EntryTarget<FlatState>] { [.state(.idle)] }

    func getHistoryParent(_ history: HistoryID) -> FlatState {
        preconditionFailure("the flat machine declares no <history>")
    }

    func getHistoryDefaultTargets(_ history: HistoryID) -> [EntryTarget<FlatState>] {
        preconditionFailure("the flat machine declares no <history>")
    }

    func historyValue(_ history: HistoryID) -> [FlatState]? { nil }

    func getDocumentOrder(_ state: FlatState) -> Int { state.rawValue }

    func getEventName(_ event: FlatEvent) -> String { event.name }

    func getEventFromName(_ name: String) -> FlatEvent? {
        FlatEvent.allCases.first { $0 != .null && $0.name == name }
    }

    func getStateName(_ state: FlatState) -> String { state.name }

    func getStateFromName(_ name: String) -> FlatState? {
        FlatState.allCases.first { $0.name == name }
    }

    func nullEvent() -> FlatEvent { .null }

    func bindCurrentEvent(_ event: FlatEvent, engine: Engine<ScriptedPolicy>) {}

    func firstEnabledTransition(
        _ state: FlatState,
        event: FlatEvent,
        engine: Engine<ScriptedPolicy>
    ) -> EnabledTransition<FlatState>? {
        for (index, transition) in (transitions[state] ?? []).enumerated()
        where transition.event == event && transition.guardHolds() {
            return EnabledTransition(
                source: state,
                targets: transition.target.map { [.state($0)] } ?? [],
                transitionIndex: index,
                hasActions: transition.content != nil,
                isInternal: false
            )
        }
        return nil
    }

    func executeTransitionContent(source: FlatState, transitionIndex: Int, engine: Engine<ScriptedPolicy>) {
        log.append("content \(source.name)#\(transitionIndex)")
        transitions[source]?[transitionIndex].content?(engine)
    }

    func executeEntryActions(_ state: FlatState, engine: Engine<ScriptedPolicy>, isDefaultEntry: Bool) {
        if !active.contains(state) {
            active.append(state)
        }
        log.append("enter \(state.name)")
        onEntry[state]?(engine)
    }

    func executeExitActions(_ state: FlatState, engine: Engine<ScriptedPolicy>, configurationBeforeExit: [FlatState]) {
        log.append("exit \(state.name)")
        onExit[state]?(engine)
        active.removeAll { $0 == state }
    }

    func executeHistoryDefaultContent(_ history: HistoryID, engine: Engine<ScriptedPolicy>) {}

    var hasParallelStates: Bool { false }
    var needsScriptEngine: Bool { false }
    var needsDataModelInit: Bool { false }
    var hasInvokeSupport: Bool { false }
    var hasFinalize: Bool { false }
    var hasAutoforward: Bool { false }
    var hasActiveStates: Bool { true }
    var hasChildTick: Bool { false }

    func getActiveStates() -> [FlatState] { active }

    func setActiveStates(_ states: [FlatState]) { active = states }

    var hasParentSession: Bool { hasParent }

    func deliverToParent(eventName: String, eventData: String) -> Bool {
        if !hasParent {
            return false
        }
        parentDeliveries.append((name: eventName, data: eventData))
        return true
    }

    func isInvocationRunning(_ invokeID: String) -> Bool { invocationsRunning.contains(invokeID) }

    func deliverToInvocation(invokeID: String, eventName: String, eventData: String, sendID: String) -> Bool {
        invocationDeliveries.append((invokeID: invokeID, name: eventName))
        return invocationsRunning.contains(invokeID)
    }

    func isChildSessionRunning(_ childSessionID: String) -> Bool { sessionsRunning.contains(childSessionID) }

    func deliverToChildSession(childSessionID: String, eventName: String, eventData: String) -> Bool {
        sessionDeliveries.append((sessionID: childSessionID, name: eventName))
        return sessionsRunning.contains(childSessionID)
    }

    func settleChildren(engine: Engine<ScriptedPolicy>) {
        settledChildren += 1
    }
}

/// An engine of a scripted policy, on a manual clock the test owns.
func makeEngine(_ script: (ScriptedPolicy) -> Void) -> (Engine<ScriptedPolicy>, ScriptedPolicy, ManualClock) {
    let policy = ScriptedPolicy()
    script(policy)
    let engine = Engine(policy: policy)
    let clock = ManualClock()
    engine.setClock(clock)
    return (engine, policy, clock)
}
