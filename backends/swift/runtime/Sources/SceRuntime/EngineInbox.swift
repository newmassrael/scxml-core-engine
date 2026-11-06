// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The inbox of a Swift engine: the one part of it a thread that does not own it may use
// (docs/adr/0084, ARCHITECTURE.md "Engine Threading"). The Swift sibling of the Kotlin
// runtime's `EngineInbox` and of the C++ `SCE::Core::EngineInbox`.

import Synchronization

/// The end of one start of a host-run invocation, as a thread that does not own the engine
/// reports it: what `Engine.completeHostInvoke` and `Engine.failHostInvoke` take, field for
/// field. The Swift sibling of the C++ `SCE::HostInvokeEnding`.
public struct HostInvokeEnding: Sendable, Equatable {
    /// The `type` the request named.
    public var processorType: String

    /// The invocation's id, as the request named it.
    public var invokeID: String

    /// The token the request carried.
    public var token: UInt64

    /// True for a completion (`done.invoke`), false for a failure (`error.invoke`).
    public var done: Bool

    /// `_event.data` of the event the ending raises.
    public var data: String

    /// `_event.origin` and `_event.origintype` of that event, for a host whose invocation
    /// stands for a party it can name, in the host's own terms (docs/adr/0090, section 2):
    /// its declared type and the address a `<send>` of that type reaches the party at. Nil
    /// for a host that names no party.
    public var origin: EventOrigin?

    public init(
        processorType: String,
        invokeID: String,
        token: UInt64,
        done: Bool,
        data: String,
        origin: EventOrigin? = nil
    ) {
        self.processorType = processorType
        self.invokeID = invokeID
        self.token = token
        self.done = done
        self.data = data
        self.origin = origin
    }
}

/// What may be posted to an engine's `EngineInbox`. The set is closed: it is the inputs that
/// have a caller on another thread, each carrying exactly what the owner-thread operation of the
/// same name takes, because the owner applies a posted input by calling that operation.
///
/// A Mesh delivery is not among them: nothing in the Swift runtime calls `applyMeshCall` from
/// another thread (a child's router is pulled by its parent, on the owner thread, with
/// `applyMeshCalls`), and `MeshCall` carries `EventMetadata`, which is not `Sendable`.
public enum EngineInboxInput<Event> {
    /// An external event, applied with `Engine.raiseExternal(_:eventData:origin:)`.
    /// `origin` is the pair the host names for it in its own terms, nil for none
    /// (docs/adr/0090, section 2).
    case externalEvent(Event, eventData: String = "", origin: EventOrigin? = nil)

    /// An external event named by text, applied with
    /// `Engine.raiseExternalByName(_:eventData:origin:)`: resolved against the document's names
    /// when it is applied, and dropped then if the document has no event the name could be
    /// delivered as.
    case externalEventByName(String, eventData: String = "", origin: EventOrigin? = nil)

    /// The end of a host-run invocation's start, applied with `Engine.completeHostInvoke` or
    /// `Engine.failHostInvoke`. The token check those make is made when the input is applied, so
    /// an ending posted for a start that was cancelled or started again by then raises nothing,
    /// as a direct one would (§scxml-6.4).
    case hostInvokeEnding(HostInvokeEnding)
}

extension EngineInboxInput: Sendable where Event: Sendable {}

/// What one inbox holds behind its lock.
struct InboxHold<Event> {
    /// What was posted and not applied yet, oldest first.
    var inputs: [EngineInboxInput<Event>] = []

    /// Whether posts are refused.
    var closed: Bool

    /// What `EngineInbox.wakeWith` installed.
    var waker: (@Sendable () -> Void)?
}

/// The owner's half of an inbox, as the engine holds it whatever its event type: the engine
/// makes its inbox only when its event type is `Sendable` (`Engine.inbox`), and drains it from
/// code that cannot say so.
protocol OwnedInbox<Event>: AnyObject {
    associatedtype Event

    /// Posts are taken from now on.
    func acceptPosts()

    /// Every post from now on is refused.
    func refusePosts()

    /// What the inbox held when this was called, oldest first, and nothing left in it.
    func takeAll() -> [EngineInboxInput<Event>]
}

/// The inbox of one engine (`Engine.inbox`): a multi-producer, single-consumer queue, and the
/// only part of an engine that is safe to use from any thread. The engine's owner applies what was
/// posted at its next macrostep boundary, before it takes the next external event, and before it
/// puts an event of its own on the external queue, in the order the posts were taken.
public final class EngineInbox<Event: Sendable>: Sendable {
    private let hold: Mutex<InboxHold<Event>>

    init(closed: Bool) {
        hold = Mutex(InboxHold<Event>(closed: closed))
    }

    /// Posts `input`; true when the inbox took it.
    ///
    /// The inbox is unbounded, so it takes every post from the moment the engine is built until
    /// the engine stops (`Engine.stop`) or its run reaches a top-level `<final>`, and refuses every
    /// post after either until a run is entered again (`Engine.initialize`, `Engine.enterAt`). A
    /// post taken before the first run is applied at that run's first macrostep boundary. After a
    /// post the inbox took, the waker the host installed is called on this thread.
    public func post(_ input: EngineInboxInput<Event>) -> Bool {
        let outcome: (taken: Bool, waker: (@Sendable () -> Void)?) = hold.withLock { held in
            if held.closed {
                return (false, nil)
            }
            held.inputs.append(input)
            return (true, held.waker)
        }
        guard outcome.taken else {
            return false
        }
        outcome.waker?()
        return true
    }

    /// Installs `waker`, called on the posting thread after each post the inbox took, so a host
    /// that drives the engine (`Engine.tick`, `Engine.step`) knows there is something to apply.
    /// Nil removes it.
    public func wakeWith(_ waker: (@Sendable () -> Void)?) {
        hold.withLock { held in
            held.waker = waker
        }
    }
}

extension EngineInbox: OwnedInbox {
    func acceptPosts() {
        hold.withLock { held in
            held.closed = false
        }
    }

    func refusePosts() {
        hold.withLock { held in
            held.closed = true
        }
    }

    func takeAll() -> [EngineInboxInput<Event>] {
        hold.withLock { held in
            let taken = held.inputs
            held.inputs = []
            return taken
        }
    }
}

extension Engine where Policy.Event: Sendable {
    /// This engine's inbox: the handle a thread that does not own the engine posts external
    /// events and host-run invocations' endings through (docs/adr/0084). Made the first time it
    /// is asked for, on the owner; every later answer is the same inbox.
    ///
    /// Offered when the event type is `Sendable`, which every generated machine's is: a handle
    /// that crosses threads carries the machine's events across with it.
    public var inbox: EngineInbox<Event> {
        if let made = ownedInbox as? EngineInbox<Event> {
            return made
        }
        let made = EngineInbox<Event>(closed: inboxClosed)
        ownedInbox = made
        return made
    }
}

extension Engine {
    /// A run is entered: the inbox takes posts again.
    func openInbox() {
        inboxClosed = false
        ownedInbox?.acceptPosts()
    }

    /// The run ended, or was stopped: the inbox refuses posts from now on, and what it still
    /// holds is applied as a direct call would be now. An event joins the external queue of a
    /// machine that will not dequeue it in this run, and an ending finds its invocation over.
    func closeInbox() {
        inboxClosed = true
        ownedInbox?.refusePosts()
        drainInbox()
    }

    /// The owner's half of the inbox (docs/adr/0084): applies, on the owner, what other threads
    /// posted, in the order the inbox took it, each by the owner-thread operation of the same
    /// name. Reached at the macrostep boundary and before an event of the owner's own joins the
    /// external queue (`enqueueExternal`). What the inbox held when the drain began, so posters
    /// that never stop cannot hold the owner here; what they post meanwhile is applied at the
    /// next drain.
    ///
    /// An owner-thread operation, as every call on the engine is: the `Sendable` handle posts
    /// and never drains.
    func drainInbox() {
        // One drain at a time. Applying an input is the owner-thread operation of the same name,
        // and each of those raises its event through `enqueueExternal`, which drains first:
        // without this flag a drain would start inside the one applying the inputs before it,
        // and take what was posted since ahead of the rest of what it took.
        guard !drainingInbox, let owned = ownedInbox else {
            return
        }
        drainingInbox = true
        defer { drainingInbox = false }
        for input in owned.takeAll() {
            applyInboxInput(input)
        }
    }

    /// Applies one posted `input` by the operation a direct call on the owner makes.
    private func applyInboxInput(_ input: EngineInboxInput<Event>) {
        switch input {
        case let .externalEvent(event, eventData, origin):
            raiseExternal(event, eventData: eventData, origin: origin)
        case let .externalEventByName(name, eventData, origin):
            raiseExternalByName(name, eventData: eventData, origin: origin)
        case let .hostInvokeEnding(ending):
            if ending.done {
                completeHostInvoke(
                    processorType: ending.processorType,
                    invokeID: ending.invokeID,
                    token: ending.token,
                    doneData: ending.data,
                    origin: ending.origin
                )
            } else {
                failHostInvoke(
                    processorType: ending.processorType,
                    invokeID: ending.invokeID,
                    token: ending.token,
                    errorData: ending.data,
                    origin: ending.origin
                )
            }
        }
    }
}
