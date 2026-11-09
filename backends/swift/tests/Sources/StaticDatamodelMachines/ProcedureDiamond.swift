// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: procedure_diamond.scxml:2 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// An event-driven procedure: a class that states the policy of a state machine (its states and
// events, the entry action of each state, the transition each takes on each event) and adopts the
// runtime's `ProcedureStateMachine`, whose `runToCompletion` runs it. It supports <onentry>/<send>,
// event-driven <transition>, <assign> and <donedata>. A pure decision tree, with no events and no
// sends, runs by the transitions that name no event.
//
// A service address is sent as text: an integer in decimal, a string as written.
//
// The states and the events are enumerations nested in the class. A file-level enumeration of one
// name would be declared again by every procedure of the module, which is one SwiftPM target.

import SceForgeRuntime

public final class ProcedureDiamond: ProcedureStateMachine {
    public enum State {
        case classify
        case highPath
        case midPath
        case lowPath
        case accept
        case reject
    }

    public enum Event {
        case noEvent
        case errorExecution
        case fail
        case ok
    }

    public var serviceHandler: ProcedureServiceHandler?
    public var doneData: [String: String] = [:]
    public var pendingEventData = ""

    private var sensorValue: UInt16 = 0
    private var mode: String = ""

    public init() {}

    public func setSensorValue(_ value: UInt16) {
        self.sensorValue = value
    }

    public func setMode(_ value: String) {
        self.mode = value
    }

    public func initialState() -> State { .classify }

    public func isFinal(_ state: State) -> Bool {
        if state == .accept { return true }
        if state == .reject { return true }
        return false
    }

    public func finalStateName(_ state: State) -> String {
        if state == .accept { return "accept" }
        if state == .reject { return "reject" }
        return ""
    }

    public func executeEntryActions(_ state: State) -> (event: Event, data: String) {
        return (.noEvent, "")
    }

    public func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>? {
        if state == .classify {
            if event == .noEvent {
                if sensorValue > 1000 {
                    return ProcedureTransition(target: .highPath, index: 0, hasAssigns: false)
                }
            }
            if event == .noEvent {
                if sensorValue > 500 {
                    return ProcedureTransition(target: .midPath, index: 1, hasAssigns: false)
                }
            }
            if event == .noEvent {
                return ProcedureTransition(target: .lowPath, index: 2, hasAssigns: false)
            }
        }
        if state == .highPath {
            if event == .noEvent {
                if mode.utf8.elementsEqual("strict".utf8) {
                    return ProcedureTransition(target: .reject, index: 0, hasAssigns: false)
                }
            }
            if event == .noEvent {
                return ProcedureTransition(target: .accept, index: 1, hasAssigns: false)
            }
        }
        if state == .midPath {
            if event == .noEvent {
                return ProcedureTransition(target: .accept, index: 0, hasAssigns: false)
            }
        }
        if state == .lowPath {
            if event == .noEvent {
                return ProcedureTransition(target: .accept, index: 0, hasAssigns: false)
            }
        }
        return nil
    }

    // Answers nil for the normal flow. An event it answers was raised by an assign-time check (a
    // bytes value past the cap of its variable), and the run loop handles it in the source state.
    public func executeTransitionActions(_ source: State, _ index: Int) -> Event? {
        return nil
    }

    /// Runs a procedure over `serviceHandler` with the helpers and the inputs it names.
    public static func execute(
        _ serviceHandler: @escaping ProcedureServiceHandler,
        _ sensorValue: UInt16,
        _ mode: String
    ) -> ProcedureRunResult {
        let sceMachine = ProcedureDiamond()
        sceMachine.serviceHandler = serviceHandler
        sceMachine.setSensorValue(sensorValue)
        sceMachine.setMode(mode)
        return sceMachine.runToCompletion()
    }
}
