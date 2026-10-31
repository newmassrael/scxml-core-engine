// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// The contract that generated state machine code implements.
///
/// The Swift equivalent of Rust's `StatePolicy` trait from
/// `backends/rust/runtime/src/policy.rs` and of the Go `StatePolicy[S, E]`
/// interface. The associated types are the machine's state and event types.
/// Generated code produces one final class conforming to `StatePolicy` per SCXML
/// source file.
///
/// The Rust version has associated consts as feature flags. Here they are
/// properties; the generated implementations return constant values.
///
/// The policy is a class and not a value: the engine holds it, the engine's
/// procedures call into it, and what it does calls back into the engine through the
/// `engine` argument of each instance method. A value type would be copied across
/// those calls.
///
/// # What the policy answers, and what it does not
///
/// The policy answers what only the document knows: its structure, which of a
/// state's transitions an event enables, what a transition's content is, what a
/// state's onentry and onexit do, what a history recorded. What W3C SCXML Appendix D
/// does with those answers (the walk from each atomic state up through its
/// ancestors, the ordered set, conflict removal, the exit set, the entry set) is
/// `Microstep.swift`, written once for every machine. Every instance method below
/// answers for one state or one transition.
///
/// # History identity
///
/// A `<history>` is named by a `HistoryID` the policy issues, and not by a type of
/// its own: `Engine` is the type a host names, and a further generic parameter on it
/// would change that for every host to carry a fact only the policy uses.
public protocol StatePolicy: AnyObject {
    associatedtype State: Hashable
    associatedtype Event: Hashable

    // MARK: Static metadata
    //
    // These encode the SCXML document structure. Generated code returns constant
    // data from these methods.

    /// The state the engine names before `initialize` enters the initial
    /// configuration (§scxml-3.2).
    ///
    /// Not what `initialize` enters: that is the document's initial transition,
    /// `getDocumentInitialTargets`, which may name several states and a `<history>`.
    func initialState() -> State

    /// Whether `state` is a `<final>` state (§scxml-3.7).
    func isFinalState(_ state: State) -> Bool

    /// The parent of `state` in the document hierarchy; nil for a child of the
    /// `<scxml>` element.
    func getParent(_ state: State) -> State?

    /// Whether `state` is a compound state: a `<state>` with child states
    /// (§scxml-3.3). A `<parallel>` answers false. This is Appendix D's
    /// `isCompoundState`, and it decides which ancestors can be a transition's
    /// domain.
    func isCompoundState(_ state: State) -> Bool

    /// Whether `state` is a `<parallel>` state (§scxml-3.4).
    func isParallelState(_ state: State) -> Bool

    /// §scxml-D-getChildStates: `state`'s `<state>`, `<parallel>` and `<final>`
    /// children, in document order; for a `<parallel>`, its regions.
    func getChildStates(_ state: State) -> [State]

    /// A compound state's initial transition target, as written (§scxml-3.3): one
    /// entry per token of `initial` or of the `<initial>` element's transition, or
    /// the first child state when the document names none. Empty for every state
    /// that is not compound.
    func getInitialTargets(_ state: State) -> [EntryTarget<State>]

    /// The target of the document's own initial transition, as written (§scxml-3.2):
    /// what `initialize` enters, from the `<scxml>` element.
    func getDocumentInitialTargets() -> [EntryTarget<State>]

    /// The state a `<history>` is declared in (§scxml-3.10).
    func getHistoryParent(_ history: HistoryID) -> State

    /// A `<history>`'s default transition target, as written: its default stored
    /// state configuration (§scxml-3.10.2).
    func getHistoryDefaultTargets(_ history: HistoryID) -> [EntryTarget<State>]

    /// The document order index of `state` (W3C SCXML Appendix D). Document order is
    /// also entry order, and its reverse is exit order.
    func getDocumentOrder(_ state: State) -> Int

    /// The human-readable name of `event`, such as `error.execution`. Used for
    /// `_event.name` population, logging and HTTP send payloads.
    func getEventName(_ event: Event) -> String

    /// The reverse lookup: the event whose name is `name`; nil for a name that is
    /// no event of the document. Used by `raiseExternalByName` and child invoke
    /// autoforward.
    func getEventFromName(_ name: String) -> Event?

    /// The human-readable name of `state`, such as `s0` and `passingState`. Required
    /// for the `In()` predicate and `_state.active` queries.
    func getStateName(_ state: State) -> String

    /// The reverse lookup: the state whose name is `name`; nil for a name the
    /// document does not declare.
    ///
    /// Required (no default) for the reason `getStateName` is: the mapping is
    /// structural, and a default could only ever answer nil. A policy that had not
    /// emitted the table would then report every recorded configuration as unknown,
    /// which a caller reads as "this run has no history" and not as a policy that is
    /// incomplete. Mirrors `getEventFromName`, which is required for the same reason
    /// on the event side.
    ///
    /// This is what lets a configuration cross a process. A host can only record
    /// state names (a journal, a wire, a file) while `Engine.enterAt` takes an array
    /// of states. A consumer-side table would age silently the moment the document
    /// gained a state; only the generator writes one that ages with the document.
    ///
    /// The round trip is an identity: `getStateFromName(getStateName(s)) == s` for
    /// every state of the document, and a name the document does not carry is nil
    /// and not a guess.
    func getStateFromName(_ name: String) -> State?

    /// The sentinel event value for eventless transition dispatch (§scxml-3.13). The
    /// engine passes it to `firstEnabledTransition` when it selects eventless
    /// transitions.
    func nullEvent() -> Event

    // MARK: Run-time state the entry procedures read

    /// What `history` recorded when its parent was last exited; nil before that ever
    /// happened (§scxml-3.10).
    func historyValue(_ history: HistoryID) -> [State]?

    // MARK: Generated executable content
    //
    // These mirror the Rust policy methods that take `&mut Engine<Self>`. Generated
    // code mutates the policy and calls engine methods through the engine argument.
    // Each answers for one state or one transition: which states a microstep exits
    // and enters, and in which order, is the engine's Appendix D procedure, not the
    // policy's.

    /// Binds the event whose transitions are about to be selected as the `_event`
    /// their guards read (§scxml-5.10).
    ///
    /// Called once per selection, before the first guard runs, and with the null
    /// event for an eventless selection, which has no event of its own, so a policy
    /// binds nothing for it. A document whose guards never read `_event` has nothing
    /// to bind.
    func bindCurrentEvent(_ event: Event, engine: Engine<Self>)

    /// Appendix D `selectTransitions`, the half only the document can answer: the
    /// first of `state`'s own transitions, in document order, that `event` enables
    /// and whose guard holds. The null event asks for eventless transitions.
    ///
    /// The only place a guard is evaluated. The engine walks the atomic states and
    /// their ancestors and keeps the ordered set.
    func firstEnabledTransition(_ state: State, event: Event, engine: Engine<Self>) -> EnabledTransition<State>?

    /// Executes one transition's executable content (§scxml-3.13: run between the
    /// microstep's exits and its entries).
    ///
    /// `transitionIndex` is the one `firstEnabledTransition` reported for `source`.
    func executeTransitionContent(source: State, transitionIndex: Int, engine: Engine<Self>)

    /// Enters `state` (§scxml-3.8): adds it to the configuration, runs its
    /// `<onentry>`, and its initial transition's content when `isDefaultEntry`. May
    /// raise internal events via `engine.raise`, schedule delayed sends, and defer
    /// `<invoke>` starts until the macrostep ends (§scxml-6.4).
    ///
    /// One state, and nothing below it: which states a microstep enters is the
    /// engine's Appendix D entry set, entered front to back, so this neither enters
    /// regions nor descends to an initial child. `isDefaultEntry` is the entry set's
    /// `statesForDefaultEntry` answer: a compound state entered only as an ancestor
    /// of a deeper target was not entered by default, and its initial transition
    /// content does not run. For a `<final>`, this is also what the appendix does on
    /// entering one.
    func executeEntryActions(_ state: State, engine: Engine<Self>, isDefaultEntry: Bool)

    /// Exits `state` (§scxml-3.9): records its histories, runs its `<onexit>`, cancels
    /// its invocations and removes it from the configuration.
    ///
    /// One state, and nothing below it: the engine's Appendix D exit set already
    /// holds every active descendant, in exit order, ahead of this state.
    /// `configurationBeforeExit` is the configuration as it stood before the
    /// microstep's first exit, which every history of the microstep is recorded from
    /// (§scxml-3.10).
    func executeExitActions(_ state: State, engine: Engine<Self>, configurationBeforeExit: [State])

    /// Runs a `<history>`'s default transition content (§scxml-3.10.2), after its
    /// parent's onentry (and after the parent's own initial content) when the
    /// history was taken with nothing recorded.
    func executeHistoryDefaultContent(_ history: HistoryID, engine: Engine<Self>)

    // MARK: Feature flags
    //
    // The Rust associated consts. Generated implementations return constant values.

    /// Whether the document contains any `<parallel>` state.
    var hasParallelStates: Bool { get }

    /// Whether ECMAScript expression evaluation is required.
    var needsScriptEngine: Bool { get }

    /// Whether the document has `<datamodel>` variables requiring script-engine
    /// initialization.
    var needsDataModelInit: Bool { get }

    /// Whether the document has any static `<invoke>` children (§scxml-6.4).
    var hasInvokeSupport: Bool { get }

    /// Whether the document's children deliver to the parent through `<finalize>`
    /// (§scxml-6.5).
    var hasFinalize: Bool { get }

    /// Whether the document autoforwards parent events to any invoke (§scxml-6.4.1).
    var hasAutoforward: Bool { get }

    /// Whether the policy keeps its own active-state set.
    var hasActiveStates: Bool { get }

    /// Whether the policy supports child-tick for nested invokes.
    var hasChildTick: Bool { get }

    // MARK: Optional instance methods
    //
    // Every generated policy implements the ones its document needs. One a document
    // does not need takes the default below, which does nothing, under the feature
    // flag above that says so.

    /// Whether the document sends to its parent session (a literal
    /// `<send target="#_parent">`): the generate manifest's `needs_parent`. A policy
    /// that does not say needs no parent.
    var needsParent: Bool { get }

    /// The ids of the document's `<invoke>`s a host runs (§scxml-6.4.1). Their
    /// `done.invoke.<id>` is accepted only through `Engine.completeHostInvoke`; a policy
    /// of a document with none answers none.
    var hostInvokeIDs: [String] { get }

    /// Initializes the datamodel via the script engine (§scxml-5.3).
    func initializeDataModel(engine: Engine<Self>)

    /// Executes any `<invoke>` deferred during entry (§scxml-6.4).
    func executePendingInvokes(engine: Engine<Self>)

    /// Executes `<finalize>` handlers for a child's event (§scxml-6.5).
    func executeFinalizeForChildEvent(_ event: inout EventWithMetadata<Event>, engine: Engine<Self>)

    /// The active states of a machine that keeps its own set (§scxml-3.4).
    func getActiveStates() -> [State]

    /// Hands a machine that keeps its own active set that set back (§scxml-3.4). The
    /// write half of `getActiveStates`, and the only caller is `Engine.enterAt`: the
    /// ordinary entry and exit paths grow the set one state at a time as they walk,
    /// while a restore is handed the whole configuration at once.
    ///
    /// A no-op on a policy whose `hasActiveStates` is false: such a machine's
    /// configuration is the parent walk from its current state, which `enterAt`
    /// restores by setting that state.
    func setActiveStates(_ states: [State])

    /// Forwards an external event to autoforward children (§scxml-6.4.1).
    func forwardToAutoforwardChildren(eventName: String, metadata: EventMetadata, engine: Engine<Self>)

    /// Ticks child state machines (§scxml-6.4).
    func tickChildren(engine: Engine<Self>)

    /// Populates pending event metadata fields from an event's metadata (§scxml-5.10).
    func populateEventMetadata(_ metadata: EventMetadata)

    /// Binds the typed `_event.data` view the natively lowered guards read (NL-to-IR
    /// Item C1 Path A), for the event being dequeued.
    ///
    /// The typed carrier `populateEventMetadata` reads is filled by one producer: the
    /// generated `<Machine>Raise<Event>` inject seam. Every other producer fills
    /// `EventMetadata.data`, so the fields are lifted out of that here and the same
    /// guard answers the same way whichever producer sent the event, including one on
    /// the far side of an invoke boundary.
    ///
    /// A throw says the data cannot be read as this event's schema. The engine then
    /// raises `error.execution` and the guard does not fire, which is what §scxml-3.13
    /// gives for a guard that cannot be evaluated, and what the script engine gives
    /// for the same guard on the same data. A policy with no typed payloads does not
    /// throw.
    func liftTypedPayload(event: Event, metadata: EventMetadata) throws

    /// Clears pending event metadata after transition processing.
    func clearEventMetadata()

    // MARK: Delivery to the sessions around this one (§scxml-6.2 + §scxml-6.4)

    /// Settles what this policy's children sent it while one was driven, so that
    /// nothing is between the two sessions when the drive returns (§scxml-6.4;
    /// docs/adr/0045): the events a child sent `#_parent` are raised on this session,
    /// and a child that reached its final state raises `done.invoke` once. A save at
    /// any boundary of the session then finds every child's buffer empty.
    func settleChildren(engine: Engine<Self>)

    /// Hands a delayed send's event to one of this policy's invocations
    /// (§scxml-6.2 + §scxml-6.4). Whether that invocation was there to take it.
    func deliverToInvocation(invokeID: String, eventName: String, eventData: String, sendID: String) -> Bool

    /// Delivers a delayed `<send target="#_parent">` to the session that invoked this
    /// one. Whether there was a parent to take it.
    func deliverToParent(eventName: String, eventData: String) -> Bool

    /// Whether a session invoked this one (W3C SCXML C.1), asked when a
    /// `<send targetexpr>` names `#_parent`.
    var hasParentSession: Bool { get }

    /// Delivers a send addressed to a child's published location (§scxml-C-1).
    func deliverToChildSession(childSessionID: String, eventName: String, eventData: String) -> Bool

    /// Whether the child session `childSessionID` is running now: the test
    /// `deliverToChildSession` makes.
    func isChildSessionRunning(_ childSessionID: String) -> Bool

    /// Whether `#_<invokeid>` names an invocation running now (§scxml-6.4 +
    /// §scxml-C-1). One that has not started, or whose session has reached its final
    /// state, is not there to address, and a send naming it is reported when it is
    /// made.
    func isInvocationRunning(_ invokeID: String) -> Bool

    /// The nearest deadline of any child this policy is running, as a duration from
    /// now; nil if there is none (§scxml-6.4).
    ///
    /// The parent's tick ticks every running child, so a moment a child needs is a
    /// moment of this machine's clock too, and `timeUntilNextScheduled` counts it. A
    /// child that has ended answers nothing, and a child's own children count through
    /// the child's own answer.
    func nextChildDeadline() -> Duration?
}

extension StatePolicy {
    public var needsParent: Bool { false }

    public var hostInvokeIDs: [String] { [] }

    public func initializeDataModel(engine: Engine<Self>) {}

    public func executePendingInvokes(engine: Engine<Self>) {}

    public func executeFinalizeForChildEvent(_ event: inout EventWithMetadata<Event>, engine: Engine<Self>) {}

    public func getActiveStates() -> [State] { [] }

    public func setActiveStates(_ states: [State]) {}

    public func forwardToAutoforwardChildren(eventName: String, metadata: EventMetadata, engine: Engine<Self>) {}

    public func tickChildren(engine: Engine<Self>) {}

    public func populateEventMetadata(_ metadata: EventMetadata) {}

    public func liftTypedPayload(event: Event, metadata: EventMetadata) throws {}

    public func clearEventMetadata() {}

    public func settleChildren(engine: Engine<Self>) {}

    public func deliverToInvocation(invokeID: String, eventName: String, eventData: String, sendID: String) -> Bool {
        false
    }

    public func deliverToParent(eventName: String, eventData: String) -> Bool { false }

    public var hasParentSession: Bool { false }

    public func deliverToChildSession(childSessionID: String, eventName: String, eventData: String) -> Bool {
        false
    }

    public func isChildSessionRunning(_ childSessionID: String) -> Bool { false }

    public func isInvocationRunning(_ invokeID: String) -> Bool { false }

    public func nextChildDeadline() -> Duration? { nil }
}

/// A policy as `Microstep.swift`'s procedures read the document: its static tables,
/// and what each `<history>` recorded.
///
/// The engine drives the procedures through it, and generated code asks two of them
/// directly (whether a `<parallel>` has completed, `isInFinalState`, and what a
/// `<history>` records as its parent exits, `recordedHistory`), so both read the one
/// transcription and not a copy written into each machine.
public struct PolicyDocument<Policy: StatePolicy>: Document {
    public typealias State = Policy.State

    public let policy: Policy

    public init(_ policy: Policy) {
        self.policy = policy
    }

    public func parentOf(_ state: State) -> State? { policy.getParent(state) }

    public func isCompound(_ state: State) -> Bool { policy.isCompoundState(state) }

    public func isParallel(_ state: State) -> Bool { policy.isParallelState(state) }

    public func isFinal(_ state: State) -> Bool { policy.isFinalState(state) }

    public func childStates(_ state: State) -> [State] { policy.getChildStates(state) }

    public func initialTargets(_ state: State) -> [EntryTarget<State>] { policy.getInitialTargets(state) }

    public func historyParent(_ history: HistoryID) -> State { policy.getHistoryParent(history) }

    public func historyValue(_ history: HistoryID) -> [State]? { policy.historyValue(history) }

    public func historyDefaultTargets(_ history: HistoryID) -> [EntryTarget<State>] {
        policy.getHistoryDefaultTargets(history)
    }

    public func documentOrder(_ state: State) -> Int { policy.getDocumentOrder(state) }

    /// Appendix D's `isInFinalState` over this document and `configuration`; see the
    /// package function of the same name.
    public func isInFinalState(_ state: State, configuration: [State]) -> Bool {
        SceRuntime.isInFinalState(self, state: state, configuration: configuration)
    }

    /// What a `<history>` of `parent` records as `parent` exits; see the package
    /// function of the same name.
    public func recordedHistory(parent: State, deep: Bool, configurationBeforeExit: [State]) -> [State] {
        SceRuntime.recordedHistory(
            self,
            parent: parent,
            deep: deep,
            configurationBeforeExit: configurationBeforeExit
        )
    }
}
