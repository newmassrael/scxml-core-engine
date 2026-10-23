// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// Where a delayed send's event goes when it comes due.
public enum RouteKind: Sendable, Equatable {
    /// `#_internal`: this session's internal queue.
    case internalQueue

    /// `#_<invokeid>`: an invocation of this session.
    case invocation

    /// `#_parent`: the session that invoked this one.
    case parent

    /// A child session named by its id: its published location (§scxml-C-1).
    case session
}

/// Where a delayed `<send>`'s event goes when it comes due (§scxml-6.2).
///
/// A delay postpones a send; it does not change where the send goes. The target is
/// resolved when the send is made and recorded here, so the delivery at the end of
/// the delay reaches it. `eventName` is the event as the receiving machine resolves
/// it: a child or a parent is another machine with its own events, so the name is
/// what crosses, as it does on an immediate send. For this session's own internal
/// queue it is the name the event was sent under, which is `_event.name` there when
/// the member is not called that (§scxml-5.10).
public struct ScheduledRoute: Sendable, Equatable {
    public var kind: RouteKind
    public var eventName: String

    /// Names the invocation, for `.invocation`.
    public var invokeID: String

    /// Names the child session, for `.session`.
    public var sessionID: String

    public init(kind: RouteKind, eventName: String, invokeID: String = "", sessionID: String = "") {
        self.kind = kind
        self.eventName = eventName
        self.invokeID = invokeID
        self.sessionID = sessionID
    }
}

/// One act that came due: an event to raise.
///
/// `route` is nil for an ordinary delayed send, whose `event`, `data`, `sendID` and
/// `origin` are what to raise on this session's own external queue.
public struct ReadyAct<Event> {
    /// The event to raise.
    public var event: Event?

    public var data: String

    /// The `<send>`'s id, which `_event.sendid` carries (§scxml-5.10.1).
    public var sendID: String

    /// The sending session, which `_event.origin` publishes (§scxml-C-1).
    public var origin: String

    /// The name `event` was sent under when that is not its own (§scxml-5.10), or
    /// the empty text.
    public var name: String

    /// The target the send resolved when it was made (§scxml-6.2), or nil for this
    /// session's own external queue.
    public var route: ScheduledRoute?
}

/// An entry still waiting in the scheduler, with the moment of the engine's clock it
/// comes due: what a save reads (SCE Accepted Subset §2.15).
public struct PendingAct<Event> {
    /// When it comes due, in milliseconds on the engine's clock.
    public var readyAtMs: Int64

    /// What it does when it comes due.
    public var act: ReadyAct<Event>
}

/// The ready entry `PullScheduler.nextReadyAt` reports, left where it is.
public struct NextReady<Event> {
    /// The instant it came due.
    public var dueMs: Int64

    /// The event it delivers to this machine's own queues; nil for an entry that
    /// delivers no event of this machine's: a send routed to a child or a parent,
    /// whose event is a name the other machine resolves.
    public var event: Event?
}

/// Delayed event scheduling (§scxml-6.2).
///
/// Port of Rust `PullScheduler<E>` from `backends/rust/runtime/src/engine.rs`.
/// Stores delayed events with their ready-at time. The engine polls via
/// `popReadyAct` during `tick()` to fire events whose delay has elapsed.
public struct PullScheduler<Event> {
    /// A single scheduled event with its metadata.
    ///
    /// `readyAtMs` is milliseconds on the engine's `SceClock`, not an instant: the
    /// queue must not know which clock the engine reads, or it would be reading one
    /// of its own and the host could not own time. Every method below is handed the
    /// reading instead of taking one.
    private struct Entry {
        /// Nil for a send routed to a child or a parent, whose event is a name the
        /// other machine resolves.
        var event: Event?
        var eventData: String
        var sendID: String

        /// The sending session, which `_event.origin` publishes when the event
        /// arrives (§scxml-C-1). A delayed send is the same event as an immediate
        /// one, only later, so waiting must not cost it the address a reply goes
        /// back to.
        var origin: String

        /// The name the event was sent under when that is not the event's own
        /// (§scxml-5.10): a `<send eventexpr>` computes it, and the member that takes
        /// it is the document's event of that name, the longest token prefix of it
        /// the document writes, or its wildcard. Empty when the member's own name is
        /// the name sent; `_event.name` is the whole name when the wait ends.
        var name: String
        var readyAtMs: Int64

        /// The target a delayed send resolved when it was made (§scxml-6.2), or nil
        /// for this session's own external queue.
        var route: ScheduledRoute?
    }

    private var entries: [Entry] = []
    private var nextAutoSendID: UInt64 = 0

    public init() {
        entries.reserveCapacity(4)
    }

    private mutating func effectiveSendID(_ sendID: String) -> String {
        if !sendID.isEmpty {
            return sendID
        }
        nextAutoSendID += 1
        return "auto_send_\(nextAutoSendID)"
    }

    /// Queues an event to come due at `readyAtMs` on the engine's clock
    /// (§scxml-6.2).
    ///
    /// If `sendID` is empty, an automatic id is generated. Returns the id used (the
    /// caller can use it to cancel). Matches Rust `PullScheduler::schedule_event_at`:
    /// the deadline is resolved by the caller, which is the only party that knows
    /// which clock the engine reads.
    @discardableResult
    public mutating func scheduleEvent(
        _ event: Event,
        readyAtMs: Int64,
        sendID: String,
        eventData: String,
        origin: String,
        name: String = ""
    ) -> String {
        append(
            event: event,
            readyAtMs: readyAtMs,
            sendID: sendID,
            eventData: eventData,
            origin: origin,
            name: name,
            route: nil
        )
    }

    private mutating func append(
        event: Event?,
        readyAtMs: Int64,
        sendID: String,
        eventData: String,
        origin: String,
        name: String,
        route: ScheduledRoute?
    ) -> String {
        let id = effectiveSendID(sendID)
        entries.append(
            Entry(
                event: event,
                eventData: eventData,
                sendID: id,
                origin: origin,
                name: name,
                readyAtMs: readyAtMs,
                route: route
            )
        )
        return id
    }

    /// Queues a delayed send whose target is not this session's own external queue,
    /// to be delivered there at `readyAtMs` (§scxml-6.2).
    ///
    /// The same queue as `scheduleEvent`, so one deadline order, one
    /// `<cancel sendid>` path (§scxml-6.3) and one `nextReadyAtMs` cover it. `event`
    /// is this machine's own spelling of the event, used when the route is this
    /// session's internal queue; nil when the other machine resolves the name.
    @discardableResult
    public mutating func scheduleRouted(
        _ event: Event?,
        readyAtMs: Int64,
        sendID: String,
        eventData: String,
        origin: String,
        route: ScheduledRoute
    ) -> String {
        append(
            event: event,
            readyAtMs: readyAtMs,
            sendID: sendID,
            eventData: eventData,
            origin: origin,
            name: "",
            route: route
        )
    }

    /// Cancels a scheduled event by send id (§scxml-6.2.5). Returns whether it was
    /// found and removed.
    public mutating func cancelEvent(sendID: String) -> Bool {
        let before = entries.count
        entries.removeAll { $0.sendID.utf8.elementsEqual(sendID.utf8) }
        return entries.count < before
    }

    /// Whether any scheduled event is ready to fire (`readyAtMs <= nowMs`).
    public func hasReadyEvents(at nowMs: Int64) -> Bool {
        entries.contains { $0.readyAtMs <= nowMs }
    }

    /// The index of the ready entry that came due first, ties keeping insertion
    /// order.
    private func firstReadyIndex(at nowMs: Int64) -> Int? {
        var best: Int?
        for (i, entry) in entries.enumerated() where entry.readyAtMs <= nowMs {
            if best == nil || entry.readyAtMs < entries[best!].readyAtMs {
                best = i
            }
        }
        return best
    }

    /// The ready entry `popReadyAct` would take next, left where it is: the instant
    /// it came due, and the event it delivers to this machine's own queues when it is
    /// one. Nil when nothing is ready.
    ///
    /// The same-instant bound reads it before popping: an entry due at the reading is
    /// the only kind a handler can arm during the tick that pops it, and a cut reports
    /// the event it was still popping (ARCHITECTURE.md "External-Event Budget").
    /// Matches Rust `PullScheduler::next_ready_at_with_event`.
    public func nextReadyAt(_ nowMs: Int64) -> NextReady<Event>? {
        guard let best = firstReadyIndex(at: nowMs) else {
            return nil
        }
        let entry = entries[best]
        let own = entry.route == nil || entry.route?.kind == .internalQueue
        return NextReady(dueMs: entry.readyAtMs, event: own ? entry.event : nil)
    }

    /// Pops the act that came due first (§scxml-6.2.4 + §scxml-6.2.5), judged
    /// against `nowMs`. Deadline order is the queue's, so the kinds interleave by when
    /// they were due and by nothing else.
    ///
    /// Deadline order, not insertion order: the caller dispatches these one at a time
    /// and runs a macrostep between them, so whichever comes out first is the one
    /// whose transitions run first. Picking by insertion would let a later-scheduled
    /// event be delivered ahead of an earlier one whenever the host woke after both
    /// came due, which is the difference between a `<cancel>` landing and being lost.
    /// Ties keep insertion order.
    ///
    /// Matches Rust `PullScheduler::pop_ready_act_at`.
    public mutating func popReadyAct(at nowMs: Int64) -> ReadyAct<Event>? {
        guard let best = firstReadyIndex(at: nowMs) else {
            return nil
        }
        let entry = entries.remove(at: best)
        return ReadyAct(
            event: entry.event,
            data: entry.eventData,
            sendID: entry.sendID,
            origin: entry.origin,
            name: entry.name,
            route: entry.route
        )
    }

    /// Whether there are any scheduled events, ready or not.
    public var hasPendingEvents: Bool {
        !entries.isEmpty
    }

    /// When the earliest still-queued entry comes due, in milliseconds on the
    /// engine's clock, whether or not it is ready yet. Nil when nothing is
    /// scheduled.
    ///
    /// A host driving the machine has to decide when to call `tick` again, and
    /// without this it can only guess an interval; see
    /// `Engine.timeUntilNextScheduled` for what guessing costs. Matches Rust
    /// `PullScheduler::next_ready_at`.
    public var nextReadyAtMs: Int64? {
        entries.map(\.readyAtMs).min()
    }

    /// Every entry still waiting, in the order the queue would deliver them when each
    /// had come due: by the moment it comes due, and entries due at the same moment in
    /// the order they were armed (`popReadyAct` takes the first of them). A copy, so a
    /// caller cannot change the queue through it.
    public func pending() -> [PendingAct<Event>] {
        entries.enumerated()
            .sorted { ($0.element.readyAtMs, $0.offset) < ($1.element.readyAtMs, $1.offset) }
            .map { _, entry in
                PendingAct(
                    readyAtMs: entry.readyAtMs,
                    act: ReadyAct(
                        event: entry.event,
                        data: entry.eventData,
                        sendID: entry.sendID,
                        origin: entry.origin,
                        name: entry.name,
                        route: entry.route
                    )
                )
            }
    }
}
