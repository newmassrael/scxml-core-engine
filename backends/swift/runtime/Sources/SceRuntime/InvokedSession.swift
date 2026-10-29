// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// A policy that can run as the child session of an `<invoke>` (§scxml-6.4): it holds
/// the queue its parent installs, and the id its parent derives for it.
///
/// Every generated policy is one: a root has no queue and keeps the id it drew, and a
/// child is handed both by the machine that starts it. The two are the whole of what a
/// parent needs from a child besides its engine.
public protocol InvocablePolicy: StatePolicy {
    /// The queue of what this session sends `#_parent`, installed by the invoking
    /// parent and drained by it (§scxml-6.2.4 + §scxml-6.4). Its presence is the
    /// link to the parent.
    var parentEventQueue: ParentEventQueue? { get set }

    /// This session's id, `_sessionid` (§scxml-5.10): a child's is the id its parent
    /// started it under.
    var sessionID: String { get set }
}

/// One invoked child session: a policy and the engine that runs it, behind the
/// `ChildEngine` surface a parent drives (§scxml-6.4).
///
/// The parent's policy and the child's are different types, so a parent holds its
/// children through `ChildEngine` and not through `Engine<ChildPolicy>`. This class is
/// the one conformance every child needs, written once: each member forwards to the
/// engine, and `parentEventQueue` to the policy that owns it.
public final class InvokedSession<Policy: InvocablePolicy>: ChildEngine {
    /// The child's engine, for a host that drives it directly and a test that
    /// reads it.
    public let engine: Engine<Policy>

    /// Builds the session of `policy` under `sessionID`, whose sends to `#_parent`
    /// reach `queue` (§scxml-6.4). The engine is not running yet: the parent calls
    /// `initialize()` once the session is registered, as it does for a child it builds
    /// any other way.
    public init(policy: Policy, sessionID: String, queue: ParentEventQueue) {
        policy.sessionID = sessionID
        policy.parentEventQueue = queue
        engine = Engine(policy: policy)
    }

    public func initialize() {
        engine.initialize()
    }

    public func tick() {
        engine.tick()
    }

    public func stop() {
        engine.stop()
    }

    public var isInFinalState: Bool {
        engine.isInFinalState
    }

    public func raiseExternalByName(_ eventName: String, eventData: String) {
        engine.raiseExternalByName(eventName, eventData: eventData)
    }

    public func step() {
        engine.step()
    }

    public func raiseExternalByNameWithMeta(_ eventName: String, metadata: EventMetadata) {
        engine.raiseExternalByNameWithMeta(eventName, metadata: metadata)
    }

    public func setCompletionCallback(_ callback: (() -> Void)?) {
        engine.setCompletionCallback(callback)
    }

    public var parentEventQueue: ParentEventQueue? {
        engine.policy.parentEventQueue
    }

    public var donedataAtFinal: String {
        engine.donedataAtFinal
    }

    public func timeUntilNextScheduled() -> Duration? {
        engine.timeUntilNextScheduled()
    }
}
