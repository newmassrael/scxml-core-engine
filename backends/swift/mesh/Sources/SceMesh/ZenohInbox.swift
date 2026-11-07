// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

// The Zenoh binding of a host-level Mesh core (SCE_MESH.md section 20): every machine opens one client
// session to a Zenoh router and reaches every other machine through it. This file is the binding and the
// seam it needs, and no Zenoh library: a client that moves bytes and presence over a real session is
// written against `ZenohClient` by whoever owns that dependency (the router's own repository, whose
// build depends on this one and not the other way round).
//
// Three things are decided here and nowhere else.
//
// The key space (section 20.1) is four functions of `ZenohInboxKeys`; no other code spells a key.
//
// The meaning of presence (section 20.3) is `ZenohSessionState`, a small machine: a peer is ready from its
// token appearing until it ends, and the end of the session tells the core every peer it believed ready is
// lost, once, and from then on makes no peer ready. The state changes together with the event it stands for,
// under one lock, so the end of the session cannot fall between a peer being counted ready and the core
// being told so.
//
// The order of what the core is told is the order the state changed in. Events are queued under that lock
// and given to the host's sink outside it, one drainer at a time, so a sink that calls back into the
// binding cannot deadlock it and two client threads cannot interleave two events' delivery.

/// The key space every machine of one Mesh shares (section 20.1). `1` is the version of the binding.
public enum ZenohInboxKeys {
    public static let prefix = "sce-mesh/1"

    /// The key `sender` publishes on to reach `receiver`. The sender's name is in the key because the core
    /// must be told who an envelope came from, and a name in the payload is whatever the sender wrote.
    public static func inbox(receiver: String, sender: String) -> String {
        "\(prefix)/to/\(receiver)/\(sender)"
    }

    /// What a machine subscribes to for everything addressed to it, and nothing wider.
    public static func inboxFilter(receiver: String) -> String {
        "\(prefix)/to/\(receiver)/*"
    }

    /// The key of the liveliness token that says `machine` is up.
    public static func presence(machine: String) -> String {
        "\(prefix)/up/\(machine)"
    }

    /// What a machine subscribes to for the presence of every machine.
    public static let presenceFilter = "\(prefix)/up/*"

    /// The last chunk of a key: the machine name an inbox or presence key ends in.
    public static func lastChunk(of key: String) -> String {
        guard let slash = key.lastIndex(of: "/") else {
            return key
        }
        return String(key[key.index(after: slash)...])
    }
}

/// Something a client holds open for the binding. Undeclaring it stops what it stands for: the subscriber
/// stops delivering, the presence token ends.
public protocol ZenohHold: AnyObject, Sendable {
    func undeclare()
}

/// What a presence subscription reports about one key.
public enum ZenohPresence: Sendable, Equatable {
    /// A token appeared on the key, or was already there when the subscription asked for history.
    case put

    /// The token on the key ended: its session closed, or the router's lease on it ran out.
    case delete
}

/// Why a client could not declare a subscriber or a token; the words are the client's own.
public struct ZenohClientError: Error, Equatable, Sendable {
    public var message: String

    public init(_ message: String) {
        self.message = message
    }
}

/// Why a client's put was refused, and whether putting the same bytes again could succeed.
public struct ZenohPutError: Error, Equatable, Sendable {
    public var retryable: Bool
    public var message: String

    public init(retryable: Bool, message: String) {
        self.retryable = retryable
        self.message = message
    }
}

/// The four operations the binding needs of a Zenoh client session, and the notice that the session ended,
/// and nothing more. The binding owns the keys, the order and the meaning of what arrives; a client only
/// moves bytes and presence, and says when it lost its session.
///
/// The operations are synchronous: each returns once the client has done it. The callbacks run on a thread
/// of the client's choosing, one call at a time per subscription, and must not be assumed to run before the
/// declaring call returns.
///
/// A client must make presence end when `close()` has run and the holds are undeclared: that is the only way
/// the other machines learn this one has left, and the binding keeps no keepalive of its own.
public protocol ZenohClient: AnyObject, Sendable {
    /// Subscribe to the keys that match `filter` (a key expression whose last chunk may be `*`), calling
    /// `onSample` with the full key and the payload bytes of each put that matches.
    func subscribe(
        filter: String,
        onSample: @escaping @Sendable (_ key: String, _ payload: [UInt8]) -> Void
    ) throws(ZenohClientError) -> any ZenohHold

    /// Subscribe to the presence tokens that match `filter`, calling `onChange` with the key of a token and
    /// whether it appeared or ended. With `history`, the tokens that are already there are reported as
    /// `.put` too, so a machine that opens after another learns it is up.
    func subscribePresence(
        filter: String,
        history: Bool,
        onChange: @escaping @Sendable (_ key: String, _ kind: ZenohPresence) -> Void
    ) throws(ZenohClientError) -> any ZenohHold

    /// Declare a presence token on `key`: it exists for the other machines' presence subscriptions until
    /// the returned hold is undeclared or the client's session ends.
    func declarePresence(key: String) throws(ZenohClientError) -> any ZenohHold

    /// Publish `payload` as one put on `key`. Under congestion it must block until the put is accepted and
    /// never drop it (section 20.2): an envelope is the unit the core's delivery rules count, and one the
    /// network dropped silently is one the core would wait for.
    func put(key: String, payload: [UInt8]) throws(ZenohPutError)

    /// Tell the binding when the session this client holds has ended, by calling `onEnd` with the client's
    /// own words, at most once, from a thread of the client's choosing, when the session has ended and the
    /// client will not carry on by itself: the router went away, the link failed, or the session was closed
    /// from outside. A client that reconnects inside itself and re-declares what it held must call it when
    /// it loses the session, and not when it has the new one: the router ended this machine's presence token
    /// when the session went, and a client that replays only what exists now can never learn who left in the
    /// meantime, so the binding resyncs from scratch.
    ///
    /// The binding calls it first, before any declaration, so a session that ends while the binding is still
    /// being set up is not missed. A throw fails the open.
    func onSessionEnd(_ onEnd: @escaping @Sendable (_ words: String) -> Void) throws(ZenohClientError)

    /// Close the client's session. The binding calls it before it undeclares what the client holds.
    func close()
}

/// Why a binding could not be opened.
public enum ZenohError: Error, Equatable, Sendable {
    /// A machine name is not a key chunk as it stands (section 20.1).
    case notAKeyChunk(String)

    /// The session, a subscriber or the token could not be opened; the words are the client's.
    case session(String)
}

/// Refuse a machine name that is not a key chunk as it stands. `open` asks it first thing; a caller that has
/// to build a client before it can open the binding asks it before that, so a bad name never costs a session.
public func checkZenohMachineName(_ machine: String) throws(ZenohError) {
    if !isPathSegment(machine) {
        throw .notAKeyChunk(machine)
    }
}

/// What every peer is told when the session they were ready on ends.
let zenohSessionEndedWords = "the session to the router ended"

/// What a send answers once the session has ended.
let zenohSessionHasEndedWords = "the session to the router has ended"

/// What the binding knows of the session its client holds: the peers it has told the core are ready, and
/// whether the client has said the session ended.
struct ZenohSessionState {
    /// The peers whose presence was reported and has not ended since: what the core believes is ready, and
    /// so what an end of the session must tell it is lost. Empty once the session has ended.
    private(set) var ready: Set<String> = []

    /// The client's words, once it has said the session ended.
    private(set) var endedWords: String?

    /// What a presence change of `peer` tells the core, or nothing once the session has ended: a client
    /// that carries on by itself after the end must not make peers ready on a binding that has already told
    /// the core they are lost.
    mutating func presence(of peer: String, _ kind: ZenohPresence) -> LinkEvent? {
        if endedWords != nil {
            return nil
        }
        switch kind {
        case .put:
            ready.insert(peer)
            return .ready(peer: peer)
        case .delete:
            ready.remove(peer)
            return .lost(peer: peer, reason: "its presence token ended")
        }
    }

    /// Record that the session ended, saying `words`, and give back the peers the core must now be told are
    /// lost, in name order: every one it was told is ready and was not told is gone. The first end is the
    /// only one: a client that says it twice changes nothing the second time.
    mutating func end(_ words: String) -> [String]? {
        if endedWords != nil {
            return nil
        }
        endedWords = words
        let peers = ready.sorted()
        ready.removeAll()
        return peers
    }
}

/// A session's state and the queue of what the core has yet to be told, behind one lock.
final class ZenohSessionCell: Sendable {
    private struct Inner {
        var state = ZenohSessionState()
        var queue: [LinkEvent] = []
        var draining = false
    }

    private let inner = Mutex(Inner())
    private let sink: @Sendable (LinkEvent) -> Void

    init(sink: @escaping @Sendable (LinkEvent) -> Void) {
        self.sink = sink
    }

    /// Change the state and queue the events the change stands for, in one step, then give the queue to the
    /// sink if no other thread is already doing so. A thread that finds a drainer at work leaves its events
    /// to it, so the sink is called by one thread at a time and in the order the state changed.
    func apply(_ change: (inout ZenohSessionState) -> [LinkEvent]) {
        let drain = inner.withLock { inner -> Bool in
            inner.queue.append(contentsOf: change(&inner.state))
            if inner.draining || inner.queue.isEmpty {
                return false
            }
            inner.draining = true
            return true
        }
        guard drain else {
            return
        }
        while true {
            let next = inner.withLock { inner -> LinkEvent? in
                if inner.queue.isEmpty {
                    inner.draining = false
                    return nil
                }
                return inner.queue.removeFirst()
            }
            guard let event = next else {
                return
            }
            sink(event)
        }
    }

    /// Queue one event that changes no state.
    func emit(_ event: LinkEvent) {
        apply { _ in [event] }
    }

    var endedWords: String? {
        inner.withLock { $0.state.endedWords }
    }
}

/// The core's side of one Zenoh session: bytes published for the peer they name, and what the session tells
/// the host (who came up, what arrived, who went) given to the `events` sink the binding was opened with.
public final class ZenohInbox: Transport, Sendable {
    private let client: any ZenohClient
    private let machine: String
    private let session: ZenohSessionCell

    /// What the client holds open for this machine, in the order it was declared; closing the binding
    /// undeclares them, which is what takes the presence token away and tells the other machines this one is
    /// gone.
    private let holds = Mutex<[any ZenohHold]>([])
    private let closed = Mutex(false)

    private init(client: any ZenohClient, machine: String, session: ZenohSessionCell) {
        self.client = client
        self.machine = machine
        self.session = session
    }

    /// Open the binding for `machine` on `client`.
    ///
    /// Readiness is the presence token (section 20.3). The inbox is subscribed before the token is declared,
    /// so that a machine which sees this one come up can send to it at once and not into a subscriber that
    /// does not exist yet. The presence subscriber asks for history, so a machine that opens after another
    /// learns it is up. The client's notice that its session ended is asked for before anything is declared.
    /// A refusal at any step closes the client and undeclares what came before it.
    ///
    /// When the session ends, every peer that was ready is told to the sink as lost, once, and the binding is
    /// over: `sessionEnded` answers and sends are refused as retryable. This binding does not reconnect.
    public static func open(
        client: any ZenohClient,
        machine: String,
        events: @escaping @Sendable (LinkEvent) -> Void
    ) throws(ZenohError) -> ZenohInbox {
        try checkZenohMachineName(machine)
        let session = ZenohSessionCell(sink: events)
        let inbox = ZenohInbox(client: client, machine: machine, session: session)
        do throws(ZenohClientError) {
            try client.onSessionEnd { words in
                session.apply { state in
                    guard let peers = state.end(words) else {
                        return []
                    }
                    return peers.map { .lost(peer: $0, reason: zenohSessionEndedWords) }
                }
            }
            inbox.adopt(
                try client.subscribe(filter: ZenohInboxKeys.inboxFilter(receiver: machine)) { key, payload in
                    let sender = ZenohInboxKeys.lastChunk(of: key)
                    if isPathSegment(sender) {
                        session.emit(.received(peer: sender, bytes: payload))
                    }
                })
            inbox.adopt(
                try client.subscribePresence(filter: ZenohInboxKeys.presenceFilter, history: true) { key, kind in
                    let peer = ZenohInboxKeys.lastChunk(of: key)
                    if peer == machine || !isPathSegment(peer) {
                        return
                    }
                    session.apply { state in state.presence(of: peer, kind).map { [$0] } ?? [] }
                })
            inbox.adopt(try client.declarePresence(key: ZenohInboxKeys.presence(machine: machine)))
        } catch {
            inbox.close()
            throw .session(error.message)
        }
        return inbox
    }

    private func adopt(_ hold: any ZenohHold) {
        holds.withLock { $0.append(hold) }
    }

    /// The client's words if the session this binding was opened on has ended, and nil while it has not.
    /// Once it answers, the binding has told the core every peer it had ready is lost, it refuses sends as
    /// retryable, and nothing more will come from this session: the host opens a new binding.
    public var sessionEnded: String? {
        session.endedWords
    }

    public func transmit(peer: String, bytes: [UInt8]) -> TransportFailure? {
        if !isPathSegment(peer) {
            return TransportFailure(retryable: false, message: "peer name \"\(peer)\" is not a key chunk")
        }
        // A session that has ended carries nothing, and the client is not asked: the core keeps the envelope
        // and sends it again, on this binding's successor if there is one.
        if session.endedWords != nil {
            return TransportFailure(retryable: true, message: zenohSessionHasEndedWords)
        }
        // One envelope per put, and block rather than drop under congestion (section 20.2): the client is
        // required to do that, see `ZenohClient.put`.
        do throws(ZenohPutError) {
            try client.put(key: ZenohInboxKeys.inbox(receiver: peer, sender: machine), payload: bytes)
            return nil
        } catch {
            return TransportFailure(retryable: error.retryable, message: error.message)
        }
    }

    /// Close the client's session, then undeclare what it held in the order it was declared. Closing twice
    /// does nothing the second time; dropping the binding closes it.
    public func close() {
        let first = closed.withLock { closed -> Bool in
            let first = !closed
            closed = true
            return first
        }
        guard first else {
            return
        }
        client.close()
        let held = holds.withLock { holds -> [any ZenohHold] in
            let taken = holds
            holds.removeAll()
            return taken
        }
        for hold in held {
            hold.undeclare()
        }
    }

    deinit {
        close()
    }
}
