// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! §scxml-3.12.1 / C.1: internal and external event queues (FIFO).
//!
//! Ports C++ `sce/include/core/EventQueueManager.h` and `AOTEventQueue.h`.
//! The engine holds one [`EventQueueManager`] for internal events (high
//! priority, raised via `<raise>` and internal sends) and one for external
//! events (lower priority, from `<send>` without a target or with external
//! targets).
//!
//! §scxml-C-1 (test189): internal events are processed exhaustively before
//! any external event. The engine's macrostep loop drains the internal queue
//! first, then processes one external event, then re-drains the internal queue.
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 (lines 1989-1994): under `--no-default-features` the
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

#[cfg(feature = "std")]
use std::collections::VecDeque;

use crate::event::EventMetadata;
#[cfg(not(feature = "std"))]
use crate::event::EventType;
use crate::helpers::tables::EventId;
#[cfg(feature = "std")]
use crate::SceString;

/// One event on a queue, as the engine's main loop takes it: its number in the
/// machine's event enum and its metadata (docs/adr/0068).
///
/// A number and not the machine's event type, so the code that puts an event on
/// a queue, takes it off and asks whether there is one is the same for every
/// machine and compiled once. The typed payload a schema-carrying machine
/// attaches rides beside it in the queue's own storage ([`EventQueue`]).
#[derive(Debug, Clone, Default)]
pub struct QueuedEvent {
    /// The event's number ([`StatePolicy::event_id`](crate::StatePolicy::event_id)).
    pub event: EventId,
    /// What §scxml-5.10 binds into `_event` besides its name.
    pub metadata: EventMetadata,
    /// §scxml-C-2: the HTTP POST target URL, for an event a BasicHTTP send
    /// raised; see [`EventWithMetadata`](crate::EventWithMetadata).
    #[cfg(feature = "std")]
    pub target: SceString,
}

impl QueuedEvent {
    /// An empty slot of a bounded queue's storage, never read: a slot is
    /// written before the ring counts it.
    #[cfg(not(feature = "std"))]
    const EMPTY: Self = Self {
        event: EventId(0),
        metadata: EventMetadata {
            event_type: EventType::Internal,
        },
    };
}

/// A machine's event queue: its events as numbers, and, for a machine whose
/// events carry a typed payload, the payloads in the same order
/// (docs/adr/0068).
///
/// The storage is the machine's, sized by its resolved capacity `N` under
/// `no_std` and unbounded under `std`. The engine drives the events through
/// [`EventQueueMut`], which has no `N` and no machine type, so the code that
/// does is one copy for every machine. Only the payloads are the machine's
/// own, and a schemaless machine's payload is `()`, which nothing stores.
#[derive(Debug)]
pub struct EventQueue<Payload, const N: usize = { crate::MAX_EVENT_QUEUE_DEPTH }> {
    #[cfg(feature = "std")]
    events: VecDeque<QueuedEvent>,
    #[cfg(not(feature = "std"))]
    slots: [QueuedEvent; N],
    #[cfg(not(feature = "std"))]
    head: u16,
    #[cfg(not(feature = "std"))]
    len: u16,
    payloads: EventQueueManager<Payload, N>,
}

impl<Payload, const N: usize> EventQueue<Payload, N> {
    /// An empty queue; a constant expression on both profiles, which is what
    /// lets the queue declare its empty value as an associated constant
    /// ([`EventQueueStorage::EMPTY`]).
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            events: VecDeque::new(),
            #[cfg(not(feature = "std"))]
            slots: [QueuedEvent::EMPTY; N],
            #[cfg(not(feature = "std"))]
            head: 0,
            #[cfg(not(feature = "std"))]
            len: 0,
            payloads: EventQueueManager::new(),
        }
    }

    /// The events, as the engine drives them.
    #[inline]
    pub fn events(&mut self) -> EventQueueMut<'_> {
        EventQueueMut {
            #[cfg(feature = "std")]
            events: &mut self.events,
            #[cfg(not(feature = "std"))]
            slots: &mut self.slots,
            #[cfg(not(feature = "std"))]
            head: &mut self.head,
            #[cfg(not(feature = "std"))]
            len: &mut self.len,
        }
    }

    /// Whether the queue holds an event.
    #[inline]
    pub fn has_events(&self) -> bool {
        #[cfg(feature = "std")]
        {
            !self.events.is_empty()
        }
        #[cfg(not(feature = "std"))]
        {
            self.len != 0
        }
    }

    /// The queued events, front first, left in place: what a saved state
    /// records of the queue and what a report names.
    pub fn queued(&self) -> impl Iterator<Item = &QueuedEvent> {
        #[cfg(feature = "std")]
        {
            self.events.iter()
        }
        #[cfg(not(feature = "std"))]
        {
            let head = usize::from(self.head);
            (0..usize::from(self.len)).map(move |offset| &self.slots[(head + offset) % N])
        }
    }

    /// Put `payload` behind the payloads already queued, for the event pushed
    /// with it.
    #[inline]
    pub fn push_payload(&mut self, payload: Payload) {
        self.payloads.raise(payload);
    }

    /// The payload of the event at the front, taken off with it.
    #[inline]
    pub fn pop_payload(&mut self) -> Option<Payload> {
        self.payloads.pop()
    }
}

impl<Payload, const N: usize> Default for EventQueue<Payload, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// The events of an [`EventQueue`], without its capacity or its payload type:
/// what the engine's code that is the same for every machine drives
/// (docs/adr/0068).
#[derive(Debug)]
pub struct EventQueueMut<'a> {
    #[cfg(feature = "std")]
    events: &'a mut VecDeque<QueuedEvent>,
    #[cfg(not(feature = "std"))]
    slots: &'a mut [QueuedEvent],
    #[cfg(not(feature = "std"))]
    head: &'a mut u16,
    #[cfg(not(feature = "std"))]
    len: &'a mut u16,
}

impl EventQueueMut<'_> {
    /// §scxml-3.12.1: put `event` at the back of the queue.
    ///
    /// Under `no_std` a push past the machine's resolved depth stops the
    /// program rather than drop the event: Appendix D processes every queued
    /// event, so the bounded queue fails loud.
    pub fn push(&mut self, event: QueuedEvent) {
        #[cfg(feature = "std")]
        {
            self.events.push_back(event);
        }
        #[cfg(not(feature = "std"))]
        ring_push(self.slots, self.head, self.len, event);
    }

    /// Take the event at the front of the queue, `None` when it is empty.
    pub fn pop(&mut self) -> Option<QueuedEvent> {
        #[cfg(feature = "std")]
        {
            self.events.pop_front()
        }
        #[cfg(not(feature = "std"))]
        {
            ring_pop(self.slots, self.head, self.len)
        }
    }

    /// The event at the front of the queue, left there.
    pub fn front(&self) -> Option<&QueuedEvent> {
        #[cfg(feature = "std")]
        {
            self.events.front()
        }
        #[cfg(not(feature = "std"))]
        {
            ring_front(self.slots, *self.head, *self.len)
        }
    }

    /// Whether the queue holds an event.
    #[inline]
    pub fn has_events(&self) -> bool {
        #[cfg(feature = "std")]
        {
            !self.events.is_empty()
        }
        #[cfg(not(feature = "std"))]
        {
            *self.len != 0
        }
    }
}

// The bounded queue's ring: `len` events from `head`, wrapping at the end of
// `slots`. Free functions over the parts rather than methods of the view, so the
// host's tests, which build the `std` queue, compile and hold them too.

/// Put `item` behind the `len` items from `head`.
#[cfg(any(test, not(feature = "std")))]
fn ring_push<T>(slots: &mut [T], head: &mut u16, len: &mut u16, item: T) {
    let capacity = slots.len();
    let count = usize::from(*len);
    if count == capacity {
        queue_full();
    }
    slots[(usize::from(*head) + count) % capacity] = item;
    *len += 1;
}

/// Take the item at `head`, `None` when there is none.
#[cfg(any(test, not(feature = "std")))]
fn ring_pop<T: Default>(slots: &mut [T], head: &mut u16, len: &mut u16) -> Option<T> {
    if *len == 0 {
        return None;
    }
    let first = usize::from(*head);
    let item = core::mem::take(&mut slots[first]);
    *head = ((first + 1) % slots.len()) as u16;
    *len -= 1;
    Some(item)
}

/// The item at `head`, left there.
#[cfg(any(test, not(feature = "std")))]
fn ring_front<T>(slots: &[T], head: u16, len: u16) -> Option<&T> {
    if len == 0 {
        None
    } else {
        Some(&slots[usize::from(head)])
    }
}

/// A push past a bounded queue's depth. No formatted message, so an image does
/// not carry the formatting machinery for it.
#[cfg(any(test, not(feature = "std")))]
#[cold]
#[inline(never)]
fn queue_full() -> ! {
    panic!(
        "an event queue is full: raise the machine's <scxml sce:capacity>; Appendix D processes \
         every queued event, so the engine stops rather than drop one"
    )
}

/// FIFO queue for SCXML events with metadata.
///
/// Generic over the event type (typically `EventWithMetadata<E, P>` where `E`
/// is the policy's event enum) and the no_std FIFO depth `N`. Ports C++
/// `SCE::Core::EventQueueManager<T>`.
///
/// Implementation: under std, a thin wrapper around `VecDeque<T>`
/// (unbounded) — `N` is inert (the std build keeps the spec's unbounded
/// `Queue`). Under `--no-default-features`, a wrapper around
/// `heapless::Deque<T, N>` (stack-allocated, sized exactly to the machine's
/// resolved event-queue capacity). `N` defaults to
/// [`crate::MAX_EVENT_QUEUE_DEPTH`] so a bare `EventQueueManager<T>` keeps the
/// crate baseline for schemaless/unspecified machines. Named to match C++ so
/// generated code reads naturally when cross-referenced with C++ source.
#[derive(Debug)]
pub struct EventQueueManager<T, const N: usize = { crate::MAX_EVENT_QUEUE_DEPTH }> {
    #[cfg(feature = "std")]
    queue: VecDeque<T>,
    #[cfg(not(feature = "std"))]
    queue: ::heapless::Deque<T, N>,
}

impl<T, const N: usize> EventQueueManager<T, N> {
    /// Construct an empty queue.
    ///
    /// A constant expression on both profiles, which is what lets a queue
    /// declare its empty value as an associated constant
    /// ([`EventQueueStorage::EMPTY`]).
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            queue: VecDeque::new(),
            #[cfg(not(feature = "std"))]
            queue: ::heapless::Deque::new(),
        }
    }

    /// §scxml-3.12.1: Enqueue an event at the back of the FIFO queue.
    ///
    /// Matches C++ `raise(T&&)`. Under `--no-default-features` an attempted push
    /// past the machine's resolved depth `N` panics rather than silently
    /// dropping the event — W3C SCXML Appendix D processes every queued event,
    /// so the bounded backing fails loud rather than drop.
    pub fn raise(&mut self, event: T) {
        #[cfg(feature = "std")]
        {
            self.queue.push_back(event);
        }
        #[cfg(not(feature = "std"))]
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

    /// The queued events, front first — what a saved state records of the
    /// queue without taking anything off it.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.queue.iter()
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

/// The queue a machine's policy supplies (W3C SCXML Appendix D `internalQueue`
/// / `externalQueue`), as the [`Engine`](crate::Engine) drives it: an
/// [`EventQueue`] at the machine's resolved capacity (see
/// [`crate::StatePolicy::EventQueue`]), its events through a view without that
/// capacity (docs/adr/0068).
pub trait EventQueueStorage<Payload>: Default {
    /// An empty queue, as a constant (docs/adr/0070).
    ///
    /// A constant and not only `Default`, for two reasons. An engine built in
    /// a constant expression (`Engine::new_const`) needs its queues as values
    /// written as constants. And an engine built at run time writes this into
    /// its own field: the empty slots are zeroes the compiler clears where they
    /// live, where a call to `default()` built the queue on the constructor's
    /// stack and copied it in, a loop and a frame the size of both queues in
    /// every machine.
    const EMPTY: Self;
    /// The events, as the engine drives them.
    fn events(&mut self) -> EventQueueMut<'_>;
    /// Whether the queue holds an event.
    fn has_events(&self) -> bool;
    /// The queued events, front first, left in place — what a saved state
    /// records of a queue (SCE Accepted Subset §2.15).
    fn queued(&self) -> impl Iterator<Item = &QueuedEvent>;
    /// Queue the payload of the event pushed with it.
    fn push_payload(&mut self, payload: Payload);
    /// Take the payload of the event taken off the front.
    fn pop_payload(&mut self) -> Option<Payload>;
}

impl<Payload, const N: usize> EventQueueStorage<Payload> for EventQueue<Payload, N> {
    const EMPTY: Self = Self::new();

    #[inline]
    fn events(&mut self) -> EventQueueMut<'_> {
        EventQueue::events(self)
    }
    #[inline]
    fn has_events(&self) -> bool {
        EventQueue::has_events(self)
    }
    fn queued(&self) -> impl Iterator<Item = &QueuedEvent> {
        EventQueue::queued(self)
    }
    #[inline]
    fn push_payload(&mut self, payload: Payload) {
        EventQueue::push_payload(self, payload)
    }
    #[inline]
    fn pop_payload(&mut self) -> Option<Payload> {
        EventQueue::pop_payload(self)
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

    /// The bounded queue's ring keeps FIFO order across its wrap-around: a
    /// ring of three, filled, half drained and filled again, gives its items
    /// back in the order they went in (docs/adr/0068).
    #[test]
    fn the_ring_keeps_fifo_order_across_its_wrap() {
        let mut slots = [0u16; 3];
        let (mut head, mut len) = (0u16, 0u16);
        for item in [1, 2, 3] {
            ring_push(&mut slots, &mut head, &mut len, item);
        }
        assert_eq!(ring_pop(&mut slots, &mut head, &mut len), Some(1));
        assert_eq!(ring_pop(&mut slots, &mut head, &mut len), Some(2));
        // The next two land at the end and wrap to the start.
        ring_push(&mut slots, &mut head, &mut len, 4);
        ring_push(&mut slots, &mut head, &mut len, 5);
        assert_eq!(ring_front(&slots, head, len), Some(&3));
        let drained: Vec<u16> =
            core::iter::from_fn(|| ring_pop(&mut slots, &mut head, &mut len)).collect();
        assert_eq!(drained, [3, 4, 5]);
        assert_eq!(ring_front(&slots, head, len), None);
    }

    /// A push past the ring's capacity stops the program rather than drop an
    /// event: Appendix D processes every queued event.
    #[test]
    #[should_panic(expected = "an event queue is full")]
    fn a_push_past_the_ring_is_refused_loud() {
        let mut slots = [0u16; 2];
        let (mut head, mut len) = (0u16, 0u16);
        for item in [1, 2, 3] {
            ring_push(&mut slots, &mut head, &mut len, item);
        }
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
