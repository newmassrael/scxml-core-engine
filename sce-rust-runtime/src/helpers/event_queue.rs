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
//! Watching-zenoh RFC §5.J.2 (lines 1989-1994): under `--features=no_std` the
//! backing store is a stack-allocated `heapless::Deque` capped at
//! [`crate::MAX_EVENT_QUEUE_DEPTH`] (= 64 in v1; see lib.rs for the
//! reasoning + per-document tunable deferral). Overflow under no_std panics
//! with an explicit message rather than silently dropping the event
//! (W3C §C.1 mandates no silent transition drop).

#[cfg(not(feature = "no_std"))]
use std::collections::VecDeque;

/// FIFO queue for SCXML events with metadata.
///
/// Generic over the event type (typically `EventWithMetadata<E>` where `E`
/// is the policy's event enum). Ports C++ `SCE::Core::EventQueueManager<T>`.
///
/// Implementation: under std, a thin wrapper around `VecDeque<T>`
/// (unbounded). Under `--features=no_std`, a wrapper around
/// `heapless::Deque<T, MAX_EVENT_QUEUE_DEPTH>` (stack-allocated, capped at
/// 64 in v1). Named to match C++ so generated code reads naturally when
/// cross-referenced with C++ source.
#[derive(Debug)]
pub struct EventQueueManager<T> {
    #[cfg(not(feature = "no_std"))]
    queue: VecDeque<T>,
    #[cfg(feature = "no_std")]
    queue: ::heapless::Deque<T, { crate::MAX_EVENT_QUEUE_DEPTH }>,
}

impl<T> EventQueueManager<T> {
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
    /// past [`crate::MAX_EVENT_QUEUE_DEPTH`] panics rather than silently
    /// dropping the event — W3C SCXML §C.1 mandates no silent transition drop.
    pub fn raise(&mut self, event: T) {
        #[cfg(not(feature = "no_std"))]
        {
            self.queue.push_back(event);
        }
        #[cfg(feature = "no_std")]
        {
            self.queue.push_back(event).map_err(|_| ()).expect(
                "EventQueueManager: heapless capacity exhausted (MAX_EVENT_QUEUE_DEPTH=64 — peak chained <raise> exceeded queue size; W3C §C.1 mandates no silent drop)",
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

impl<T> Default for EventQueueManager<T> {
    fn default() -> Self {
        Self::new()
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
        let mut q = EventQueueManager::new();
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
        let mut q = EventQueueManager::new();
        q.raise("a");
        q.raise("b");
        q.clear();
        assert!(!q.has_events());
    }
}
