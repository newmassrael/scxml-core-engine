// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

use std::collections::VecDeque;

/// FIFO queue for SCXML events with metadata.
///
/// Generic over the event type (typically `EventWithMetadata<E>` where `E`
/// is the policy's event enum). Ports C++ `SCE::Core::EventQueueManager<T>`.
///
/// Implementation: thin wrapper around `VecDeque<T>`. Named to match C++ so
/// generated code reads naturally when cross-referenced with C++ source.
#[derive(Debug)]
pub struct EventQueueManager<T> {
    queue: VecDeque<T>,
}

impl<T> EventQueueManager<T> {
    /// Construct an empty queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// W3C SCXML 3.12.1: Enqueue an event at the back of the FIFO queue.
    ///
    /// Matches C++ `raise(T&&)`.
    pub fn raise(&mut self, event: T) {
        self.queue.push_back(event);
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
