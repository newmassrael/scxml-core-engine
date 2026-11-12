// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: procedure_startup_check.scxml:2 :: _forge_body

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

public final class ProcedureStartupCheck: ProcedureStateMachine {
    public enum State {
        case checkVoltage
        case checkTemp
        case success
        case failVoltage
        case failOvertemp
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

    private var voltage: Float = 0.0
    private var temperature: Float = 0.0

    public init() {}

    public func setVoltage(_ value: Float) {
        self.voltage = value
    }

    public func setTemperature(_ value: Float) {
        self.temperature = value
    }

    public func initialState() -> State { .checkVoltage }

    public func isFinal(_ state: State) -> Bool {
        if state == .success { return true }
        if state == .failVoltage { return true }
        if state == .failOvertemp { return true }
        return false
    }

    public func finalStateName(_ state: State) -> String {
        if state == .success { return "success" }
        if state == .failVoltage { return "fail_voltage" }
        if state == .failOvertemp { return "fail_overtemp" }
        return ""
    }

    public func executeEntryActions(_ state: State) -> (event: Event, data: String) {
        return (.noEvent, "")
    }

    public func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>? {
        if state == .checkVoltage {
            if event == .noEvent {
                if voltage >= 11.5 && voltage <= 14.5 {
                    return ProcedureTransition(target: .checkTemp, index: 0, hasAssigns: false)
                }
            }
            if event == .noEvent {
                return ProcedureTransition(target: .failVoltage, index: 1, hasAssigns: false)
            }
        }
        if state == .checkTemp {
            if event == .noEvent {
                if temperature < 80.0 {
                    return ProcedureTransition(target: .success, index: 0, hasAssigns: false)
                }
            }
            if event == .noEvent {
                return ProcedureTransition(target: .failOvertemp, index: 1, hasAssigns: false)
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
        _ voltage: Float,
        _ temperature: Float
    ) -> ProcedureRunResult {
        let sceMachine = ProcedureStartupCheck()
        sceMachine.serviceHandler = serviceHandler
        sceMachine.setVoltage(voltage)
        sceMachine.setTemperature(temperature)
        return sceMachine.runToCompletion()
    }
}
