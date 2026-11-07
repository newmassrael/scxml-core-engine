// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Dispatch
import Synchronization

// A Zenoh binding that opens itself again when its session to the router ends (SCE_MESH.md section 20.3).
//
// `ZenohInbox.open` makes a binding on one client and is over when that client's session is. This is the
// binding a host keeps: when the client says its session ended it waits out a backoff, asks the factory for
// a new client and sets the machine up on it again through the same code as the plain binding (the end
// notice, the inbox, presence with history, the token, in that order).
//
// What the host reads is one stream through every session, in order: when a session ends the binding has
// already told the core every peer it had ready is lost, and the presence history of the next session
// announces whoever is still there as ready again. A peer that left while the session was down cannot be
// missed, because everyone was lost first and only the ones still there come back.
//
// The policy and the clock are apart. `ZenohBackoff` is a pure function of the attempt and of one random draw,
// and the waiting is a `ZenohScheduler` the host chooses, so a test drives the whole run on a clock of its own
// and no test sleeps.

/// How a new client is made each time a session has to be opened: the first, and every one after an end. The
/// factory knows the router; a host makes one that dials it.
public protocol ZenohClientFactory: Sendable {
    /// Make a client with a session of its own. A throw is the router not answering, and is retried after a
    /// backoff, except for the first connection, which is the caller's error.
    func connect() throws(ZenohClientError) -> any ZenohClient
}

/// How long the binding waits before each attempt to open a session again: `initial` before the first,
/// multiplied by `factor` after each failed one, never more than `max`.
public struct ZenohBackoff: Sendable, Equatable {
    /// The wait before the first attempt after a session ended.
    public var initial: Duration

    /// The longest wait between two attempts, before jitter.
    public var max: Duration

    /// What each failed attempt multiplies the wait by; below 1 is read as 1.
    public var factor: Int

    /// Whether each wait is spread by up to 20 percent either way, so machines that lost their session to one
    /// router together do not all dial it in the same instant.
    public var jitter: Bool

    public init(
        initial: Duration = .milliseconds(250), max: Duration = .seconds(30), factor: Int = 2, jitter: Bool = true
    ) {
        self.initial = initial
        self.max = max
        self.factor = factor
        self.jitter = jitter
    }

    /// The wait before attempt `attempt` (the first is 0), without jitter: `initial` times `factor` to the
    /// power `attempt`, capped at `max`.
    public func baseDelay(attempt: Int) -> Duration {
        // A factor past this makes every wait the cap at the first failure, whatever the initial wait; it is
        // here so the product cannot overflow a duration.
        let factor = Swift.min(Swift.max(self.factor, 1), 1 << 20)
        var delay = Swift.min(initial, max)
        var step = 0
        while step < attempt {
            if delay >= max || delay == .zero || factor == 1 {
                break
            }
            delay = Swift.min(delay * factor, max)
            step += 1
        }
        return delay
    }

    /// The wait before attempt `attempt`: `baseDelay` exactly when `jitter` is off, and otherwise that scaled
    /// by 80 to 120 percent as `draw`, a number from 0 to 1, says (0 is 80, one half is 100, 1 is 120).
    public func delay(attempt: Int, draw: Double) -> Duration {
        let base = baseDelay(attempt: attempt)
        if !jitter {
            return base
        }
        // Whole thousandths, so the scaled wait is a division of integers and the same on every machine.
        let clamped = Swift.min(Swift.max(draw, 0), 1)
        let thousandths = 800 + Int((400 * clamped).rounded())
        return base * thousandths / 1000
    }
}

/// A wait the binding asked a scheduler for, which it can take back.
public protocol ZenohTimer: AnyObject, Sendable {
    func cancel()
}

/// Where the binding's waiting happens: it asks for `work` to run after `delay`, once, on a thread of the
/// scheduler's choosing, never while the binding holds a lock.
public protocol ZenohScheduler: Sendable {
    func schedule(after delay: Duration, _ work: @escaping @Sendable () -> Void) -> any ZenohTimer
}

/// The scheduler a host uses: one serial dispatch queue of its own.
public final class DispatchZenohScheduler: ZenohScheduler {
    private let queue: DispatchQueue

    public init(label: String = "SceMesh.ZenohReconnect") {
        queue = DispatchQueue(label: label)
    }

    public func schedule(after delay: Duration, _ work: @escaping @Sendable () -> Void) -> any ZenohTimer {
        let item = DispatchWorkItem(block: work)
        let parts = delay.components
        let nanoseconds = parts.seconds * 1_000_000_000 + parts.attoseconds / 1_000_000_000
        queue.asyncAfter(deadline: .now() + .nanoseconds(Int(clamping: nanoseconds)), execute: item)
        return DispatchTimer(item: item)
    }
}

private final class DispatchTimer: ZenohTimer, @unchecked Sendable {
    private let item: DispatchWorkItem

    init(item: DispatchWorkItem) {
        self.item = item
    }

    func cancel() {
        item.cancel()
    }
}

/// The core's side of a reconnecting binding: bytes published on the current session, or a retryable failure
/// while there is none. Closing it, or dropping it, stops the reopening and closes the current session, which
/// ends this machine's presence.
public final class ZenohReconnectingInbox: Transport, Sendable {
    private struct State {
        var current: (generation: Int, inbox: ZenohInbox)?
        var generation = 0
        var failures = 0
        var stopped = false
        var timer: (any ZenohTimer)?
    }

    private let factory: any ZenohClientFactory
    private let machine: String
    private let backoff: ZenohBackoff
    private let scheduler: any ZenohScheduler
    private let draw: @Sendable () -> Double
    private let queue: ZenohEventQueue
    private let state = Mutex(State())

    private init(
        factory: any ZenohClientFactory, machine: String, backoff: ZenohBackoff, scheduler: any ZenohScheduler,
        draw: @escaping @Sendable () -> Double, queue: ZenohEventQueue
    ) {
        self.factory = factory
        self.machine = machine
        self.backoff = backoff
        self.scheduler = scheduler
        self.draw = draw
        self.queue = queue
    }

    /// Open the binding for `machine` on a client from `factory`, and open it again on a new one each time its
    /// session ends, waiting `backoff` between attempts on `scheduler`.
    ///
    /// The name is checked and the first client made and set up before this returns, on the caller's thread,
    /// so an address that does not answer is this function's error and not a silent retry. Everything after
    /// that is the scheduler's. A send while there is no session is a retryable failure and never waits for the
    /// reconnection. `random` gives the draw of each jittered wait, a number from 0 to 1.
    public static func open(
        factory: any ZenohClientFactory,
        machine: String,
        backoff: ZenohBackoff = ZenohBackoff(),
        scheduler: any ZenohScheduler = DispatchZenohScheduler(),
        random: @escaping @Sendable () -> Double = { Double.random(in: 0...1) },
        events: @escaping @Sendable (LinkEvent) -> Void
    ) throws(ZenohError) -> ZenohReconnectingInbox {
        try checkZenohMachineName(machine)
        let inbox = ZenohReconnectingInbox(
            factory: factory, machine: machine, backoff: backoff, scheduler: scheduler, draw: random,
            queue: ZenohEventQueue(sink: events))
        try inbox.connect()
        return inbox
    }

    /// Make a client and set the machine up on it, and make it the one sends go to.
    private func connect() throws(ZenohError) {
        let client: any ZenohClient
        do throws(ZenohClientError) {
            client = try factory.connect()
        } catch {
            throw .session(error.message)
        }
        let generation = state.withLock { state -> Int in
            state.generation += 1
            return state.generation
        }
        let opened = try ZenohInbox.open(client: client, machine: machine, queue: queue) { [weak self] _ in
            self?.sessionEnded(generation)
        }
        let abandoned = state.withLock { state -> Bool in
            if state.stopped {
                return true
            }
            state.current = (generation, opened)
            state.failures = 0
            return false
        }
        if abandoned {
            opened.close()
            return
        }
        // The session may have ended before it was made the current one, and its end notice found nothing to
        // reopen: ask again now that it is.
        if opened.sessionEnded != nil {
            sessionEnded(generation)
        }
    }

    /// The session of `generation` ended; the binding has already told the core every peer is lost. Take it
    /// out of the way of sends and wait out the backoff before the next attempt.
    private func sessionEnded(_ generation: Int) {
        let dead = state.withLock { state -> ZenohInbox? in
            guard !state.stopped, let current = state.current, current.generation == generation else {
                return nil
            }
            state.current = nil
            state.timer = scheduleRetry(state: &state)
            return current.inbox
        }
        // The dead session's client is closed outside the lock, so a send is never kept waiting for it.
        dead?.close()
    }

    /// Ask the scheduler for the next attempt, after the wait the failures so far call for.
    private func scheduleRetry(state: inout State) -> any ZenohTimer {
        let wait = backoff.delay(attempt: state.failures, draw: draw())
        return scheduler.schedule(after: wait) { [weak self] in
            self?.retry()
        }
    }

    private func retry() {
        let proceed = state.withLock { state -> Bool in
            state.timer = nil
            return !state.stopped
        }
        guard proceed else {
            return
        }
        do throws(ZenohError) {
            try connect()
        } catch {
            state.withLock { state in
                if !state.stopped {
                    state.failures += 1
                    state.timer = scheduleRetry(state: &state)
                }
            }
        }
    }

    public func transmit(peer: String, bytes: [UInt8]) -> TransportFailure? {
        guard let inbox = state.withLock({ $0.current?.inbox }) else {
            return TransportFailure(retryable: true, message: "the session to the router is being reopened")
        }
        return inbox.transmit(peer: peer, bytes: bytes)
    }

    /// Stop reopening and close the current session. Closing twice does nothing the second time.
    public func close() {
        let (current, timer) = state.withLock { state -> (ZenohInbox?, (any ZenohTimer)?) in
            state.stopped = true
            let taken = (state.current?.inbox, state.timer)
            state.current = nil
            state.timer = nil
            return taken
        }
        timer?.cancel()
        current?.close()
    }

    deinit {
        close()
    }
}
