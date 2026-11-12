// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: crossfile_procedure_filter.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
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

public final class CrossfileProcedureFilter: ProcedureStateMachine {
    public enum State {
        case sample
        case done
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

    private var rawSample: Double = 0.0
    private var smoothed: Double = 0.0
    private let smoother = FilterLowPass()

    public init() {}

    public func setRawSample(_ value: Double) {
        self.rawSample = value
    }

    public func initialState() -> State { .sample }

    public func isFinal(_ state: State) -> Bool {
        if state == .done { return true }
        return false
    }

    public func finalStateName(_ state: State) -> String {
        if state == .done { return "done" }
        return ""
    }

    public func executeEntryActions(_ state: State) -> (event: Event, data: String) {
        if state == .done {
            doneData["result"] = String(describing: "success")
        }
        return (.noEvent, "")
    }

    public func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>? {
        if state == .sample {
            if event == .noEvent {
                return ProcedureTransition(target: .done, index: 0, hasAssigns: true)
            }
        }
        return nil
    }

    // Answers nil for the normal flow. An event it answers was raised by an assign-time check (a
    // bytes value past the cap of its variable), and the run loop handles it in the source state.
    public func executeTransitionActions(_ source: State, _ index: Int) -> Event? {
        if source == .sample {
            if index == 0 {
                smoothed = smoother.update(rawSample)
            }
        }
        return nil
    }

    /// Runs a procedure over `serviceHandler` with the helpers and the inputs it names.
    public static func execute(
        _ serviceHandler: @escaping ProcedureServiceHandler,
        _ rawSample: Double
    ) -> ProcedureRunResult {
        let sceMachine = CrossfileProcedureFilter()
        sceMachine.serviceHandler = serviceHandler
        sceMachine.setRawSample(rawSample)
        return sceMachine.runToCompletion()
    }
}
