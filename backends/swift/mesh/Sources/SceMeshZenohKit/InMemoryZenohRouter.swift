// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Dispatch
import SceMesh
import Synchronization

// What the Zenoh binding (SCE_MESH.md section 20) needs of a router, written as a program: an executable
// statement of the part of Zenoh the binding relies on, and the router a client's tests run against when no
// real one is there. It is not a Zenoh implementation. It models exactly this, and nothing past it:
//
// - A key expression matches a key when they have the same number of chunks and each chunk of the
//   expression is `*` or the chunk of the key. `*` stands for one chunk, never for several.
// - A put reaches every subscriber whose expression matches, on every session, the putting one included.
// - A liveliness token is a key a session declares. It exists until it is undeclared or its session ends,
//   and every presence subscription that matches it is told when it appears and when it ends. A presence
//   subscription that asks for history is told of the tokens that are already there.
// - A session ends when its client closes it, which tells the client nothing, or when the router drops it
//   (a lost link, an expired lease), which tells the client once with the router's words. Either way the
//   session's subscriptions go and its tokens end.
//
// Every callback runs on one serial queue, so a subscription is called one call at a time and the order of
// deliveries is the order of the events that caused them. `settle()` waits until that queue is empty and no
// delivery is left to make, callbacks that cause further deliveries included.

/// A Zenoh router, in memory.
public final class InMemoryZenohRouter: Sendable {
    private enum Callback {
        case sample(@Sendable (String, [UInt8]) -> Void)
        case presence(history: Bool, @Sendable (String, ZenohPresence) -> Void)
    }

    private struct Subscription {
        var session: Int
        var filter: String
        var callback: Callback
    }

    private struct Token {
        var session: Int
        var key: String
    }

    private struct Session {
        var onEnd: (@Sendable (String) -> Void)?
    }

    private struct State {
        var nextID = 0
        var sessions: [Int: Session] = [:]
        var subscriptions: [Int: Subscription] = [:]
        var tokens: [Int: Token] = [:]

        mutating func newID() -> Int {
            nextID += 1
            return nextID
        }
    }

    private let state = Mutex(State())
    private let queue = DispatchQueue(label: "SceMeshZenohKit.InMemoryZenohRouter")
    private let pending = DispatchGroup()

    public init() {}

    /// A client with a session of its own on this router.
    public func connect() -> InMemoryZenohClient {
        let id = state.withLock { state -> Int in
            let id = state.newID()
            state.sessions[id] = Session()
            return id
        }
        return InMemoryZenohClient(router: self, session: id)
    }

    /// Wait until every delivery made so far has been made, and every one those caused.
    public func settle() {
        pending.wait()
    }

    /// The router ends `client`'s session: a lost link, an expired lease. Its subscriptions go, its tokens
    /// end for everyone who watches presence, and the client is told once, with `words`, that its session
    /// ended. A session that has ended already is not ended again.
    public func drop(_ client: InMemoryZenohClient, words: String) {
        let onEnd = end(session: client.session, tell: true)
        if let onEnd {
            deliver { onEnd(words) }
        }
    }

    /// The keys of the tokens that exist now, sorted.
    public var tokens: [String] {
        state.withLock { $0.tokens.values.map(\.key).sorted() }
    }

    /// Whether `filter` matches `key`: the same number of chunks, and each chunk of the filter `*` or equal.
    public static func matches(filter: String, key: String) -> Bool {
        let wanted = filter.split(separator: "/", omittingEmptySubsequences: false)
        let given = key.split(separator: "/", omittingEmptySubsequences: false)
        guard wanted.count == given.count else {
            return false
        }
        return zip(wanted, given).allSatisfy { $0 == "*" || $0 == $1 }
    }

    // MARK: What a client asks of its router

    func isLive(session: Int) -> Bool {
        state.withLock { $0.sessions[session] != nil }
    }

    func setOnEnd(session: Int, _ onEnd: @escaping @Sendable (String) -> Void) {
        state.withLock { $0.sessions[session]?.onEnd = onEnd }
    }

    func subscribe(
        session: Int, filter: String, onSample: @escaping @Sendable (String, [UInt8]) -> Void
    ) -> Int {
        state.withLock { state in
            let id = state.newID()
            state.subscriptions[id] = Subscription(session: session, filter: filter, callback: .sample(onSample))
            return id
        }
    }

    func subscribePresence(
        session: Int, filter: String, history: Bool,
        onChange: @escaping @Sendable (String, ZenohPresence) -> Void
    ) -> Int {
        let (id, existing) = state.withLock { state -> (Int, [String]) in
            let id = state.newID()
            state.subscriptions[id] = Subscription(
                session: session, filter: filter, callback: .presence(history: history, onChange))
            let keys = state.tokens.values.map(\.key).filter { Self.matches(filter: filter, key: $0) }
            return (id, keys.sorted())
        }
        if history {
            for key in existing {
                deliver { onChange(key, .put) }
            }
        }
        return id
    }

    func removeSubscription(_ id: Int) {
        state.withLock { _ = $0.subscriptions.removeValue(forKey: id) }
    }

    func declare(session: Int, key: String) -> Int {
        let (id, watchers) = state.withLock { state -> (Int, [Watcher]) in
            let id = state.newID()
            state.tokens[id] = Token(session: session, key: key)
            return (id, Self.watchers(of: key, in: state))
        }
        for watch in watchers {
            deliver { watch(key, .put) }
        }
        return id
    }

    func undeclare(token id: Int) {
        let ended = state.withLock { state -> (String, [Watcher])? in
            guard let token = state.tokens.removeValue(forKey: id) else {
                return nil
            }
            return (token.key, Self.watchers(of: token.key, in: state))
        }
        if let (key, watchers) = ended {
            for watch in watchers {
                deliver { watch(key, .delete) }
            }
        }
    }

    /// Publish `payload` on `key`; false when the session has ended and the put reached nobody.
    func put(session: Int, key: String, payload: [UInt8]) -> Bool {
        let receivers = state.withLock { state -> [@Sendable (String, [UInt8]) -> Void]? in
            guard state.sessions[session] != nil else {
                return nil
            }
            return state.subscriptions.values.compactMap { subscription in
                if case let .sample(onSample) = subscription.callback,
                    Self.matches(filter: subscription.filter, key: key)
                {
                    return onSample
                }
                return nil
            }
        }
        guard let receivers else {
            return false
        }
        for receive in receivers {
            deliver { receive(key, payload) }
        }
        return true
    }

    /// The client closes its own session: nothing is told to it.
    func close(session: Int) {
        _ = end(session: session, tell: false)
    }

    // MARK: Ending a session

    /// Remove `session`, its subscriptions and its tokens, and tell the presence subscribers of the others
    /// that the tokens ended. Returns the client's end notice when it is to be told and was asked for.
    private func end(session: Int, tell: Bool) -> (@Sendable (String) -> Void)? {
        let ending = state.withLock { state -> Ending? in
            guard let gone = state.sessions.removeValue(forKey: session) else {
                return nil
            }
            state.subscriptions = state.subscriptions.filter { $0.value.session != session }
            let mine = state.tokens.filter { $0.value.session == session }
            for id in mine.keys {
                state.tokens.removeValue(forKey: id)
            }
            var ending = Ending(onEnd: tell ? gone.onEnd : nil)
            for key in mine.values.map(\.key).sorted() {
                ending.ended.append((key: key, watchers: Self.watchers(of: key, in: state)))
            }
            return ending
        }
        guard let ending else {
            return nil
        }
        for (key, watchers) in ending.ended {
            for watch in watchers {
                deliver { watch(key, .delete) }
            }
        }
        return ending.onEnd
    }

    /// What ending a session leaves to do: the tokens that ended with who watches each, and the client's
    /// notice when it is to be told.
    private struct Ending {
        var onEnd: (@Sendable (String) -> Void)?
        var ended: [(key: String, watchers: [Watcher])] = []
    }

    private typealias Watcher = @Sendable (String, ZenohPresence) -> Void

    private static func watchers(of key: String, in state: State) -> [Watcher] {
        state.subscriptions.values.compactMap { subscription in
            if case let .presence(_, onChange) = subscription.callback,
                matches(filter: subscription.filter, key: key)
            {
                return onChange
            }
            return nil
        }
    }

    private func deliver(_ work: @escaping @Sendable () -> Void) {
        pending.enter()
        queue.async { [pending] in
            work()
            pending.leave()
        }
    }
}

/// A client with one session on an `InMemoryZenohRouter`.
public final class InMemoryZenohClient: ZenohClient {
    private let router: InMemoryZenohRouter
    let session: Int

    init(router: InMemoryZenohRouter, session: Int) {
        self.router = router
        self.session = session
    }

    public func subscribe(
        filter: String, onSample: @escaping @Sendable (String, [UInt8]) -> Void
    ) throws(ZenohClientError) -> any ZenohHold {
        try requireSession()
        let id = router.subscribe(session: session, filter: filter, onSample: onSample)
        return InMemoryHold { [router] in router.removeSubscription(id) }
    }

    public func subscribePresence(
        filter: String, history: Bool, onChange: @escaping @Sendable (String, ZenohPresence) -> Void
    ) throws(ZenohClientError) -> any ZenohHold {
        try requireSession()
        let id = router.subscribePresence(session: session, filter: filter, history: history, onChange: onChange)
        return InMemoryHold { [router] in router.removeSubscription(id) }
    }

    public func declarePresence(key: String) throws(ZenohClientError) -> any ZenohHold {
        try requireSession()
        let id = router.declare(session: session, key: key)
        return InMemoryHold { [router] in router.undeclare(token: id) }
    }

    public func put(key: String, payload: [UInt8]) throws(ZenohPutError) {
        if !router.put(session: session, key: key, payload: payload) {
            throw ZenohPutError(retryable: true, message: "the session is closed")
        }
    }

    public func onSessionEnd(_ onEnd: @escaping @Sendable (String) -> Void) throws(ZenohClientError) {
        try requireSession()
        router.setOnEnd(session: session, onEnd)
    }

    public func close() {
        router.close(session: session)
    }

    private func requireSession() throws(ZenohClientError) {
        if !router.isLive(session: session) {
            throw ZenohClientError("the session is closed")
        }
    }
}

/// What a client holds open: undeclaring it runs `release` once.
private final class InMemoryHold: ZenohHold {
    private let release: @Sendable () -> Void
    private let done = Mutex(false)

    init(release: @escaping @Sendable () -> Void) {
        self.release = release
    }

    func undeclare() {
        let first = done.withLock { done -> Bool in
            let first = !done
            done = true
            return first
        }
        if first {
            release()
        }
    }
}
