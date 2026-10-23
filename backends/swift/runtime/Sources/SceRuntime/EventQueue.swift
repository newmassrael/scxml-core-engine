// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// A FIFO queue for SCXML events with metadata.
///
/// Ports Rust `EventQueueManager<T>` from
/// `backends/rust/runtime/src/helpers/event_queue.rs`. §scxml-C-1: internal
/// events are processed exhaustively before any external event. The engine's
/// macrostep loop drains the internal queue first, then processes one external
/// event, then drains the internal queue again.
public struct EventQueueManager<Element> {
    private var queue: [Element] = []

    /// Index of the front element in `queue`. Elements before it are spent;
    /// they are dropped in one move once they outnumber the live ones, so a pop
    /// is constant time on average and not a shift of the whole array.
    private var head = 0

    public init() {
        queue.reserveCapacity(8)
    }

    /// Enqueues an event at the back (§scxml-3.12.1).
    public mutating func raise(_ event: Element) {
        queue.append(event)
    }

    /// Dequeues the next event (FIFO); nil if the queue is empty.
    public mutating func pop() -> Element? {
        guard head < queue.count else {
            return nil
        }
        let event = queue[head]
        head += 1
        if head > 32 && head * 2 > queue.count {
            queue.removeFirst(head)
            head = 0
        }
        return event
    }

    /// The event at the head of the queue, left where it is; nil if the queue is
    /// empty.
    ///
    /// A cut of the external-event budget reports the event it declined to take
    /// (ARCHITECTURE.md "External-Event Budget"), and has to read it without
    /// taking it. Matches Rust `EventQueueLike::queued().next()`.
    public func front() -> Element? {
        head < queue.count ? queue[head] : nil
    }

    /// Every queued event, front first, in an array of its own: what a save
    /// reads, and what no caller can change the queue through.
    public func snapshot() -> [Element] {
        Array(queue[head...])
    }

    /// Whether the queue holds any event.
    public var hasEvents: Bool {
        head < queue.count
    }

    /// The number of events currently queued.
    public var count: Int {
        queue.count - head
    }

    /// Whether the queue is empty (the opposite of `hasEvents`).
    public var isEmpty: Bool {
        head >= queue.count
    }

    /// Removes every queued event.
    public mutating func clear() {
        queue.removeAll(keepingCapacity: true)
        head = 0
    }
}
