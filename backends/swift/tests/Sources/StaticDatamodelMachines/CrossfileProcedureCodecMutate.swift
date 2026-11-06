// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: crossfile_procedure_codec_mutate.scxml:3 :: _forge_body

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
//
// External dependencies (from sce:payload expressions, which must be in scope):
//   frame.encode()

import SceForgeRuntime

public final class CrossfileProcedureCodecMutate: ProcedureStateMachine {
    public enum State {
        case start
        case send
        case done
        case error
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

    private var msgId: UInt8 = 0
    private var frame = CodecSimpleFrame()

    /// The bytes `frame` writes, or `nil` when it refuses the values the procedure set.
    private func sceEncodeFrame() -> [UInt8]? {
        encodeToBytes { try frame.encode(into: &$0) }
    }

    public init() {}

    public func setMsgId(_ value: UInt8) {
        self.msgId = value
    }

    public func initialState() -> State { .start }

    public func isFinal(_ state: State) -> Bool {
        if state == .done { return true }
        if state == .error { return true }
        return false
    }

    public func finalStateName(_ state: State) -> String {
        if state == .done { return "done" }
        if state == .error { return "error" }
        return ""
    }

    public func executeEntryActions(_ state: State) -> (event: Event, data: String) {
        if state == .send {
            if let sceHandler = serviceHandler {
                let sceRequest = ProcedureServiceRequest(service: "transport", payload: sceEncodeFrame())
                let sceResponse = sceHandler(sceRequest)
                return (sceResponse.success ? .ok : .fail, sceResponse.data)
            }
        }
        if state == .done {
            doneData["result"] = String(describing: "success")
        }
        if state == .error {
            doneData["result"] = String(describing: "failure")
        }
        return (.noEvent, "")
    }

    public func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>? {
        if state == .start {
            if event == .noEvent {
                return ProcedureTransition(target: .send, index: 0, hasAssigns: true)
            }
        }
        if state == .send {
            if event == .ok {
                return ProcedureTransition(target: .done, index: 0, hasAssigns: false)
            }
            if event == .fail {
                return ProcedureTransition(target: .error, index: 1, hasAssigns: false)
            }
        }
        return nil
    }

    // Answers nil for the normal flow. An event it answers was raised by an assign-time check (a
    // bytes value past the cap of its variable), and the run loop handles it in the source state.
    public func executeTransitionActions(_ source: State, _ index: Int) -> Event? {
        if source == .start {
            if index == 0 {
                frame.msgId = msgId
            }
        }
        return nil
    }

    /// Runs a procedure over `serviceHandler` with the helpers and the inputs it names.
    public static func execute(
        _ serviceHandler: @escaping ProcedureServiceHandler,
        _ msgId: UInt8
    ) -> ProcedureRunResult {
        let sceMachine = CrossfileProcedureCodecMutate()
        sceMachine.serviceHandler = serviceHandler
        sceMachine.setMsgId(msgId)
        return sceMachine.runToCompletion()
    }
}
