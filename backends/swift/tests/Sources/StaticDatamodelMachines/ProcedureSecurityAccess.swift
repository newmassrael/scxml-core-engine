// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: procedure_security_access.scxml:1 :: _forge_body

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
//   computeKey(seed)

import SceForgeRuntime

public final class ProcedureSecurityAccess: ProcedureStateMachine {
    public enum State {
        case sendTesterPresent
        case requestSeed
        case sendKey
        case retry
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

    private var ecuAddr: UInt32 = 0
    private var seed: [UInt8] = []
    private var maxRetries: Int32 = 3
    private var retryCount: Int32 = 0

    // <sce:helper> DI closures: each fails fast until the host sets it.
    private var computeKey: ([UInt8]) -> [UInt8] = { _ in fatalError("helper 'computeKey' not set: call setComputeKey() before runToCompletion()") }

    public init() {}

    public func setComputeKey(_ fn: @escaping ([UInt8]) -> [UInt8]) {
        self.computeKey = fn
    }

    public func setEcuAddr(_ value: UInt32) {
        self.ecuAddr = value
    }

    public func initialState() -> State { .sendTesterPresent }

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
        if state == .sendTesterPresent {
            if let sceHandler = serviceHandler {
                let sceRequest = ProcedureServiceRequest(service: "TesterPresent", addr: String(ecuAddr))
                let sceResponse = sceHandler(sceRequest)
                return (sceResponse.success ? .ok : .fail, sceResponse.data)
            }
        }
        if state == .requestSeed {
            if let sceHandler = serviceHandler {
                let sceRequest = ProcedureServiceRequest(service: "SecurityAccess", subfunc: "0x01")
                let sceResponse = sceHandler(sceRequest)
                return (sceResponse.success ? .ok : .fail, sceResponse.data)
            }
        }
        if state == .sendKey {
            if let sceHandler = serviceHandler {
                let sceRequest = ProcedureServiceRequest(service: "SecurityAccess", subfunc: "0x02", payload: computeKey(seed))
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
        if state == .sendTesterPresent {
            if event == .ok {
                return ProcedureTransition(target: .requestSeed, index: 0, hasAssigns: false)
            }
            if event == .fail {
                return ProcedureTransition(target: .error, index: 1, hasAssigns: false)
            }
        }
        if state == .requestSeed {
            if event == .ok {
                return ProcedureTransition(target: .sendKey, index: 0, hasAssigns: true)
            }
            if event == .fail {
                return ProcedureTransition(target: .retry, index: 1, hasAssigns: false)
            }
        }
        if state == .sendKey {
            if event == .ok {
                return ProcedureTransition(target: .done, index: 0, hasAssigns: false)
            }
            if event == .fail {
                return ProcedureTransition(target: .retry, index: 1, hasAssigns: false)
            }
        }
        if state == .retry {
            if event == .noEvent {
                if retryCount < maxRetries {
                    return ProcedureTransition(target: .requestSeed, index: 0, hasAssigns: true)
                }
            }
            if event == .noEvent {
                if retryCount >= maxRetries {
                    return ProcedureTransition(target: .error, index: 1, hasAssigns: false)
                }
            }
        }
        return nil
    }

    // Answers nil for the normal flow. An event it answers was raised by an assign-time check (a
    // bytes value past the cap of its variable), and the run loop handles it in the source state.
    public func executeTransitionActions(_ source: State, _ index: Int) -> Event? {
        if source == .requestSeed {
            if index == 0 {
                do {
                    let sceValue = Array(pendingEventData.utf8)
                    if sceValue.count > 64 {
                        return .errorExecution
                    }
                    seed = sceValue
                }
            }
        }
        if source == .retry {
            if index == 0 {
                retryCount = retryCount + 1
            }
        }
        return nil
    }

    /// Runs a procedure over `serviceHandler` with the helpers and the inputs it names.
    public static func execute(
        _ serviceHandler: @escaping ProcedureServiceHandler,
        _ computeKey: @escaping ([UInt8]) -> [UInt8],
        _ ecuAddr: UInt32
    ) -> ProcedureRunResult {
        let sceMachine = ProcedureSecurityAccess()
        sceMachine.serviceHandler = serviceHandler
        sceMachine.setComputeKey(computeKey)
        sceMachine.setEcuAddr(ecuAddr)
        return sceMachine.runToCompletion()
    }
}
