// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The building blocks of the `procedure` kind (SCE_FORGE.md Section 4.5): the request a procedure
// sends to a service, the answer it gets, and the loop that drives a generated procedure from its
// initial state to a final one. Siblings: `com.sce.forge.runtime.procedure` (Kotlin),
// `sce_forge_runtime::procedure` (Rust) and `scealgorithm` (Go).

/// A request a procedure sends to a service while it runs.
///
/// The fields are the four attributes of the `<send>` that makes it:
///
/// ```
/// <send sce:service="Diag" sce:subfunc="0x02" sce:addr="ecuAddr" sce:payload="frame.encode()"/>
/// ```
///
/// `service` is always present. The other three are `nil` where the attribute is absent, which an
/// empty value is not. `payload` is the bytes of a wire format, as a codec's `encode` writes them;
/// `subfunc` and `addr` are text, since the document may name a variable of any type for them.
public struct ProcedureServiceRequest: Equatable {
    public let service: String
    public let subfunc: String?
    public let addr: String?
    public let payload: [UInt8]?

    public init(
        service: String,
        subfunc: String? = nil,
        addr: String? = nil,
        payload: [UInt8]? = nil
    ) {
        self.service = service
        self.subfunc = subfunc
        self.addr = addr
        self.payload = payload
    }
}

/// What a service answers: whether it succeeded, and the text it returns.
public struct ProcedureServiceResponse: Equatable {
    public let success: Bool
    public let data: String

    public init(success: Bool = false, data: String = "") {
        self.success = success
        self.data = data
    }
}

/// How a procedure ended: whether it reached a final state, which one, and the `<donedata>` that
/// state wrote. A procedure that stopped short of a final state reports neither.
public struct ProcedureRunResult: Equatable {
    public let completed: Bool
    public let finalState: String
    public let doneData: [String: String]

    public init(completed: Bool = false, finalState: String = "", doneData: [String: String] = [:]) {
        self.completed = completed
        self.finalState = finalState
        self.doneData = doneData
    }
}

/// The function a procedure calls for each `<send>` that names a service.
public typealias ProcedureServiceHandler = (ProcedureServiceRequest) -> ProcedureServiceResponse

/// The transition a state takes: the state it goes to, which of the state's transitions it is
/// (the index `executeTransitionActions` is given), and whether it carries `<assign>`s to run.
public struct ProcedureTransition<State> {
    public let target: State
    public let index: Int
    public let hasAssigns: Bool

    public init(target: State, index: Int, hasAssigns: Bool) {
        self.target = target
        self.index = index
        self.hasAssigns = hasAssigns
    }
}

/// An event-driven procedure: the policy a generated class states, and the loop that runs it.
///
/// A generated class names its states and events, says which state it starts in and which end it,
/// and what each state does on entry and on each event; `runToCompletion` drives it from the
/// initial state through its service sends until it reaches a final state or no transition is
/// possible. The loop is the runtime's, so every procedure runs by the same rules.
public protocol ProcedureStateMachine: AnyObject {
    associatedtype State: Equatable
    associatedtype Event: Equatable

    /// The function the `<send sce:service>` actions call. `nil` where none is set, and a send
    /// then does nothing.
    var serviceHandler: ProcedureServiceHandler? { get set }

    /// The `<donedata>` the final state wrote.
    var doneData: [String: String] { get set }

    /// The data of the event being handled (`_event.data`), which an entry action sets from a
    /// service's answer.
    var pendingEventData: String { get set }

    /// The state the procedure starts in.
    func initialState() -> State

    /// Whether `state` is a `<final>` state.
    func isFinal(_ state: State) -> Bool

    /// The SCXML id of the final state `state`, such as `done` or `error`.
    func finalStateName(_ state: State) -> String

    /// Runs the entry actions of `state`: the event to handle next and its data.
    func executeEntryActions(_ state: State) -> (event: Event, data: String)

    /// The transition `state` takes on `event`, or `nil` where it takes none.
    func processTransition(_ state: State, _ event: Event) -> ProcedureTransition<State>?

    /// Runs the `<assign>`s of the transition at `index` out of `source`.
    ///
    /// Answers `nil` for the normal flow. An event answers that an assign-time check raised it, a
    /// value that does not fit the bytes cap of its variable for one, and the loop handles that
    /// event in `source` and takes no transition.
    func executeTransitionActions(_ source: State, _ index: Int) -> Event?
}

extension ProcedureStateMachine {
    /// The bound on the loop, so a procedure that never reaches a final state ends.
    public static var maxIterations: Int { 1000 }

    /// Runs the procedure to completion, blocking.
    public func runToCompletion() -> ProcedureRunResult {
        var current = initialState()
        let initial = executeEntryActions(current)
        var event = initial.event
        if !initial.data.isEmpty { pendingEventData = initial.data }

        for _ in 0..<Self.maxIterations {
            if isFinal(current) { break }
            guard let transition = processTransition(current, event) else { break }
            if transition.hasAssigns {
                // An assign-time check may raise an event: it is handled in the source state
                // instead of advancing to the target. The document's own `<transition>` for it
                // picks it up, and a state that has none ends the run short of a final state,
                // as an internal event nothing handles does.
                if let raised = executeTransitionActions(current, transition.index) {
                    event = raised
                    continue
                }
            }
            current = transition.target
            let entry = executeEntryActions(current)
            event = entry.event
            if !entry.data.isEmpty { pendingEventData = entry.data }
        }

        let completed = isFinal(current)
        return ProcedureRunResult(
            completed: completed,
            finalState: completed ? finalStateName(current) : "",
            doneData: completed ? doneData : [:]
        )
    }
}
