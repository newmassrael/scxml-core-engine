// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML 3.12.1 / C.1: internal and external event queues (FIFO).
//!
//! Ports C++ `sce/include/core/EventQueueManager.h` and `AOTEventQueue.h`.
//! The engine holds one [`EventQueueManager`] for internal events (high
//! priority, raised via `<raise>` and internal sends) and one for external
//! events (lower priority, from `<send>` without a target or with external
//! targets).
//!
//! W3C SCXML C.1 (test189): internal events are processed exhaustively before
//! any external event. The engine's macrostep loop drains the internal queue
//! first, then processes one external event, then re-drains the internal queue.
//!
//! Watching-zenoh RFC §synth-5-J-2 (lines 1989-1994): under `--features=no_std` the
//! backing store is a stack-allocated `heapless::Deque<T, N>`. The depth `N`
//! is per-machine — the const generic parameter carries the
//! `<scxml sce:capacity="N">` / deploy `default_event_queue_capacity` value
//! that codegen resolves into `EVENT_QUEUE_CAPACITY`, defaulting to
//! [`crate::MAX_EVENT_QUEUE_DEPTH`] for machines that declare no capacity (see
//! [`crate::StatePolicy::EventQueue`] for the policy-carried wiring, and lib.rs
//! for the default's reasoning). Overflow under no_std panics with an explicit
//! message rather than silently dropping the event — W3C SCXML Appendix D's
//! `mainEventLoop` processes every queued event, so the no_std backing fails
//! loud rather than drop.

#[cfg(not(feature = "no_std"))]
use std::collections::VecDeque;

/// FIFO queue for SCXML events with metadata.
///
/// Generic over the event type (typically `EventWithMetadata<E, P>` where `E`
/// is the policy's event enum) and the no_std FIFO depth `N`. Ports C++
/// `SCE::Core::EventQueueManager<T>`.
///
/// Implementation: under std, a thin wrapper around `VecDeque<T>`
/// (unbounded) — `N` is inert (the std build keeps the spec's unbounded
/// `Queue`). Under `--features=no_std`, a wrapper around
/// `heapless::Deque<T, N>` (stack-allocated, sized exactly to the machine's
/// resolved event-queue capacity). `N` defaults to
/// [`crate::MAX_EVENT_QUEUE_DEPTH`] so a bare `EventQueueManager<T>` keeps the
/// crate baseline for schemaless/unspecified machines. Named to match C++ so
/// generated code reads naturally when cross-referenced with C++ source.
#[derive(Debug)]
pub struct EventQueueManager<T, const N: usize = { crate::MAX_EVENT_QUEUE_DEPTH }> {
    #[cfg(not(feature = "no_std"))]
    queue: VecDeque<T>,
    #[cfg(feature = "no_std")]
    queue: ::heapless::Deque<T, N>,
}

impl<T, const N: usize> EventQueueManager<T, N> {
    /// Construct an empty queue.
    pub fn new() -> Self {
        Self {
            #[cfg(not(feature = "no_std"))]
            queue: VecDeque::new(),
            #[cfg(feature = "no_std")]
            queue: ::heapless::Deque::new(),
        }
    }

    /// W3C SCXML 3.12.1: Enqueue an event at the back of the FIFO queue.
    ///
    /// Matches C++ `raise(T&&)`. Under `--features=no_std` an attempted push
    /// past the machine's resolved depth `N` panics rather than silently
    /// dropping the event — W3C SCXML Appendix D processes every queued event,
    /// so the bounded backing fails loud rather than drop.
    pub fn raise(&mut self, event: T) {
        #[cfg(not(feature = "no_std"))]
        {
            self.queue.push_back(event);
        }
        #[cfg(feature = "no_std")]
        {
            self.queue.push_back(event).map_err(|_| ()).expect(
                "EventQueueManager: heapless capacity exhausted (peak chained <raise> exceeded this machine's resolved EVENT_QUEUE_CAPACITY / MAX_EVENT_QUEUE_DEPTH; raise <scxml sce:capacity>; W3C Appendix D processes every enqueued event, so SCE fails loud rather than drop)",
            );
        }
    }

    /// Dequeue the next event (FIFO). Returns `None` if the queue is empty.
    ///
    /// Matches C++ `pop() -> std::optional<T>`.
    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    /// Whether the queue contains any events.
    ///
    /// Matches C++ `hasEvents() const -> bool`.
    pub fn has_events(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Number of events currently queued.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue is empty (equivalent to `!has_events()`).
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Remove all queued events.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl<T, const N: usize> Default for EventQueueManager<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Queue abstraction the [`Engine`](crate::Engine) depends on (W3C SCXML
/// Appendix D `internalQueue` / `externalQueue`).
///
/// Mirrors the C++ `EventQueueAdapter` concept
/// (`sce/include/core/EventQueueConcept.h`): the engine drives the FIFO through
/// this trait rather than a concrete sized type, so each machine's policy can
/// supply a [`EventQueueManager<T, N>`] sized to its own resolved capacity
/// (see [`crate::StatePolicy::EventQueue`]) without the engine hardcoding `N`.
/// Surface restricted to what the engine's macrostep loop actually drives
/// (Interface Segregation): `enqueue` / `dequeue` / `isEmpty`, mirroring the
/// C++ `EventQueueAdapter` concept's `popNext()` + `hasEvents()`. The richer
/// inspection methods (`len` / `is_empty` / `clear`) stay as inherent methods on
/// [`EventQueueManager`] — they are not part of the engine's dependency.
pub trait EventQueueLike<T> {
    /// W3C SCXML Appendix D `enqueue`: append an event to the FIFO back.
    fn raise(&mut self, event: T);
    /// W3C SCXML Appendix D `dequeue`: remove and return the FIFO front.
    fn pop(&mut self) -> Option<T>;
    /// Whether the queue holds any events (`!isEmpty`).
    fn has_events(&self) -> bool;
}

impl<T, const N: usize> EventQueueLike<T> for EventQueueManager<T, N> {
    fn raise(&mut self, event: T) {
        EventQueueManager::raise(self, event)
    }
    fn pop(&mut self) -> Option<T> {
        EventQueueManager::pop(self)
    }
    fn has_events(&self) -> bool {
        EventQueueManager::has_events(self)
    }
}

// ──────────────────────────────────────────────
// Unit tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_queue_has_no_events() {
        let q: EventQueueManager<i32> = EventQueueManager::new();
        assert!(!q.has_events());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn fifo_order_preserved() {
        let mut q: EventQueueManager<_> = EventQueueManager::new();
        q.raise(1);
        q.raise(2);
        q.raise(3);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn clear_removes_all() {
        let mut q: EventQueueManager<_> = EventQueueManager::new();
        q.raise("a");
        q.raise("b");
        q.clear();
        assert!(!q.has_events());
    }
}
