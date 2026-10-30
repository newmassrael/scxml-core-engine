// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// A §scxml-6.4 pending invoke, for the defer, cancel and execute pattern.
///
/// Ports Rust `PendingInvoke` from `backends/rust/runtime/src/invoke.rs`.
public struct PendingInvoke<State: Hashable> {
    /// The §scxml-6.4.1 runtime-generated invoke identifier. Format:
    /// `stateid.platformid.index`.
    public var invokeID: String

    /// The state containing the invoke element.
    public var state: State

    /// The `<invoke>` this entry defers, as the document names it: what the execute
    /// step dispatches on, compared whole. Reading it back out of `invokeID` by
    /// substring matched `probe` against `probe2`.
    public var documentID: String

    public init(invokeID: String, state: State, documentID: String) {
        self.invokeID = invokeID
        self.state = state
        self.documentID = documentID
    }
}

/// An active §scxml-6.4 child session.
///
/// Ports Rust `ChildSession` from `backends/rust/runtime/src/invoke.rs`.
public struct ChildSession: Sendable, Equatable {
    /// The child's session id, in the form `parentSessionId.invokeId`.
    public var sessionID: String

    /// The invoke element id, such as `_invoke_0`.
    public var invokeID: String

    /// The parent's session id.
    public var parentSessionID: String

    /// Whether to autoforward events to the child (§scxml-6.4.1).
    public var autoforward: Bool

    /// The `<finalize>` handler script content (§scxml-6.5).
    public var finalizeScript: String

    public init(
        sessionID: String,
        invokeID: String,
        parentSessionID: String,
        autoforward: Bool,
        finalizeScript: String
    ) {
        self.sessionID = sessionID
        self.invokeID = invokeID
        self.parentSessionID = parentSessionID
        self.autoforward = autoforward
        self.finalizeScript = finalizeScript
    }
}

/// Adds a pending invoke for deferred execution at macrostep end (§scxml-6.4).
public func deferInvoke<State: Hashable>(_ pending: inout [PendingInvoke<State>], _ invoke: PendingInvoke<State>) {
    pending.append(invoke)
}

/// Removes pending invokes for an exited state (§scxml-6.4).
public func cancelInvokesForState<State: Hashable>(_ pending: inout [PendingInvoke<State>], _ state: State) {
    pending.removeAll { $0.state == state }
}

/// Executes all pending invokes at macrostep end (§scxml-6.4). Copies the list and
/// clears it first, so executing one cannot disturb the iteration.
public func executePendingInvokes<State: Hashable>(
    _ pending: inout [PendingInvoke<State>],
    executor: (PendingInvoke<State>) -> Void
) {
    if pending.isEmpty {
        return
    }
    // §scxml-6.4: Copy the pending list so a start that defers another is not
    // iterated in this pass.
    let invokesToExecute = pending
    pending = []
    for invoke in invokesToExecute {
        executor(invoke)
    }
}

/// What every `done.invoke.<id>` begins with (§scxml-6.3.1), spelled once so that
/// building the name and recognising it cannot disagree.
public let doneInvokePrefix = "done.invoke."

/// The generic completion descriptor (§scxml-3.12.1). A document that names no
/// specific `done.invoke.<id>` matches every completion through it, so it is the
/// event a completion falls back to.
public let doneInvokeEvent = "done.invoke"

/// Names the event an invocation that did not finish raises instead of its
/// completion (a host-run invocation past its `_sce_deadline_ms`), the way
/// `sce:mesh-rpc` names a request that ran out of time (SCE_MESH.md §9.5), so a
/// document handles the two alike. Specific form: `error.invoke.<id>`.
public let errorInvokePrefix = "error.invoke."

/// The generic form of `errorInvokePrefix`, for a document that names no specific
/// `error.invoke.<id>`, as `doneInvokeEvent` is for a completion.
public let errorInvokeEvent = "error.invoke"

/// A `done.invoke` event name (§scxml-6.3.1).
public func createDoneInvokeEventName(_ invokeID: String) -> String {
    doneInvokePrefix + invokeID
}

/// Whether an invoke id has a valid format (§scxml-3.12.1).
public func isValidInvokeID(_ invokeID: String) -> Bool {
    !invokeID.isEmpty
}

/// How many invokes are pending (§scxml-6.4).
public func pendingCount<State: Hashable>(_ pending: [PendingInvoke<State>]) -> Int {
    pending.count
}

/// Whether a specific invoke is pending (§scxml-6.4).
public func isInvokePending<State: Hashable>(_ pending: [PendingInvoke<State>], invokeID: String) -> Bool {
    pending.contains { $0.invokeID.utf8.elementsEqual(invokeID.utf8) }
}

/// What parent state machines use to interact with child invoke sessions
/// (§scxml-6.4). Generated code implements this for each concrete child state
/// machine type.
///
/// The mesh members have defaults that do nothing, for a child given no Mesh router.
public protocol ChildEngine: AnyObject {
    func initialize()
    func tick()

    /// Exits the child session when its parent cancels it (§scxml-D-exitInterpreter):
    /// its states' `<onexit>` run. What that exit sends to `#_parent` lands on the
    /// child's own queue, which the parent drops with the child undrained
    /// (§scxml-6.4 cancel-drop, test252).
    func stop()

    var isInFinalState: Bool { get }

    func raiseExternalByName(_ eventName: String, eventData: String)

    /// Runs the child's queues to stability (§scxml-6.2): what a session does with
    /// an event delivered to it. See `deliverToChild`.
    func step()

    /// Delivers an autoforwarded event whose `_event` fields are the parent's
    /// (§scxml-6.4 exact-copy contract). Name-addressed because the child's event
    /// enum is an unrelated type.
    func raiseExternalByNameWithMeta(_ eventName: String, metadata: EventMetadata)

    func setCompletionCallback(_ callback: (() -> Void)?)

    /// The queue of events the child has sent its parent.
    var parentEventQueue: ParentEventQueue? { get }

    /// The donedata payload stashed by the child's top-level `<final>` onentry, for
    /// the parent's `raiseDoneInvoke` to lift onto `done.invoke.<id>._event.data`
    /// (§scxml-5.5 + 6.3.1).
    var donedataAtFinal: String { get }

    /// The child's own next deadline: the parent ticks its children, so it is a
    /// deadline of the parent's clock too, and the parent's
    /// `timeUntilNextScheduled` counts it (§scxml-6.4).
    func timeUntilNextScheduled() -> Duration?

    /// Applies what arrived for the child on the Mesh router its parent gave it
    /// (SCE_MESH.md, Child Sessions in Mesh): the parent calls it each time it
    /// drives the child, before the child's macrostep.
    func applyMeshCalls()

    /// Tells the child's Mesh router its session ended, once: after the child
    /// reached a top-level `<final>`, or after its parent stopped it.
    func detachMesh()
}

extension ChildEngine {
    public func applyMeshCalls() {}

    public func detachMesh() {}
}

/// Delivers a `<send>` addressed to an invoked session and lets that session take it
/// now (§scxml-6.2.4, §scxml-6.4), as the C++ and Kotlin cores do.
///
/// Enqueued alone, the event waits for the parent's next `tickChildren`, and a tick
/// promotes its due sends before it ticks children. So a delayed send coming due in a
/// late tick reached a child that had not yet taken an earlier immediate one: a `stop`
/// sent at once and a delayed `lost` found the child still running, `lost` joined its
/// queue behind `stop`, and vanished with the child, neither delivered nor reported
/// (C.1). Measured 2026-09-29 under load.
///
/// What the child sends to its parent while it runs here still waits on its own queue
/// for `tickChildren` to drain, so the parent sees it where it did.
public func deliverToChild(_ child: any ChildEngine, eventName: String, eventData: String) {
    child.raiseExternalByName(eventName, eventData: eventData)
    child.step()
}

/// Drains events from a child's parent event queue and raises them in the parent
/// engine with proper metadata (§scxml-6.4). Port of Rust `drain_and_raise_child_events`.
public func drainAndRaiseChildEvents<Policy: StatePolicy>(
    childEngine: (any ChildEngine)?,
    activeInvokes: [String: ChildSession],
    invokeKey: String,
    engine: Engine<Policy>
) {
    guard let queue = childEngine?.parentEventQueue else {
        return
    }
    for event in queue.drain() {
        // §scxml-3.12.1 + §scxml-5.10: the child names the event, and its names are
        // not the parent's, so one the parent does not write reaches it as the event
        // of the longest prefix it does, under the name the child sent.
        // §scxml-5.10.1: what a child sends its parent arrives from another session,
        // so it is an external event. The default of EventType is the internal one,
        // which a metadata built here would otherwise carry.
        var metadata = EventMetadata(data: event.data, eventType: .externalEvent)
        if let session = activeInvokes[invokeKey] {
            metadata.invokeID = session.invokeID
            metadata.origin = session.sessionID
            metadata.originType = scxmlEventProcessorType
        }
        if let arriving = engine.arrivingEvent(name: event.name, metadata: metadata) {
            engine.raiseExternalWithMeta(arriving)
        }
    }
}

/// The stem of the document a hybrid `<invoke>`'s evaluated `srcexpr` names: what the
/// value is matched against the declared `sce:candidates` by (§scxml-6.4,
/// SCE_ACCEPTED_SUBSET.md §2.13). Port of Rust `document_stem()`.
///
/// An expression is free to compute `file:x.scxml`, `./x.scxml`, an absolute path or a
/// Windows one for the same document, so the value is reduced to what the build named
/// the generated child by: the last path segment, without a `file:` scheme and without
/// its extension. A leading dot is a name, not an extension, as the build reads it
/// when it derives the candidate's stem.
public func documentStem(_ value: String) -> String {
    var name = Array(value.utf8)
    if let slash = name.lastIndex(where: { $0 == UInt8(ascii: "/") || $0 == UInt8(ascii: "\\") }) {
        name = Array(name[(slash + 1)...])
    }
    let scheme = Array("file:".utf8)
    if name.starts(with: scheme) {
        name = Array(name.dropFirst(scheme.count))
    }
    if let dot = name.lastIndex(of: UInt8(ascii: ".")), dot > 0 {
        return String(decoding: name[..<dot], as: UTF8.self)
    }
    return String(decoding: name, as: UTF8.self)
}

/// Raises a `done.invoke` event in the parent engine (§scxml-6.3.1). Port of Rust
/// `raise_done_invoke()`.
///
/// `donedata` is the child's stashed top-level `<final>` donedata payload
/// (§scxml-5.5 + 6.3.1); it is threaded onto `EventMetadata.data` so the parent's
/// transition `cond` can read `_event.data`. Pass the empty text when the child's
/// `<final>` carries no donedata.
public func raiseDoneInvoke<Policy: StatePolicy>(invokeID: String, donedata: String, engine: Engine<Policy>) {
    // §scxml-5.10: `_event.name` is the specific name, whichever descriptor of the
    // document (`done.invoke.<id>`, `done.invoke`, `done`) it was matched through.
    let metadata = EventMetadata(data: donedata, invokeID: invokeID)
    if let arriving = engine.arrivingEvent(name: createDoneInvokeEventName(invokeID), metadata: metadata) {
        engine.raiseExternalWithMeta(arriving)
    }
}
