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
//! backing store is a fixed ring of `N` slots in the engine itself. The depth
//! `N` is per-machine and per-queue. The external queue's carries the
//! `<scxml sce:capacity="N">` / deploy `default_event_queue_capacity` value
//! that codegen resolves into `EVENT_QUEUE_CAPACITY`, defaulting to
//! [`crate::MAX_EVENT_QUEUE_DEPTH`] for machines that declare no capacity (see
//! [`crate::StatePolicy::ExternalQueue`] for the policy-carried wiring, and
//! lib.rs for the default's reasoning). The internal queue's is the depth the
//! generator proved from the document where it could, `0` among them, and the
//! external rule where it could not ([`crate::StatePolicy::InternalQueue`],
//! docs/adr/0076). Overflow under no_std panics with an explicit
//! message rather than silently dropping the event — W3C SCXML Appendix D's
//! `mainEventLoop` processes every queued event, so the no_std backing fails
//! loud rather than drop.

#[cfg(any(test, not(feature = "std")))]
use core::mem::MaybeUninit;
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
///
/// `N` may be `0` (docs/adr/0076): a queue of no slots, which holds nothing,
/// takes nothing off and stops the program on a push. The generator gives an
/// internal queue that depth only where it proved the document never raises
/// an event for itself.
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
            ring_queued(&self.slots, self.head, self.len)
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
// host's tests, which build the `std` queue, compile and hold them too. Each
// holds for a ring of no slots (docs/adr/0076): its length is never more than
// zero, so no index is taken modulo its capacity.

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

/// The `len` items from `head`, front first, left in place.
#[cfg(any(test, not(feature = "std")))]
fn ring_queued<T>(slots: &[T], head: u16, len: u16) -> impl Iterator<Item = &T> {
    let head = usize::from(head);
    let capacity = slots.len();
    (0..usize::from(len)).map(move |offset| &slots[(head + offset) % capacity])
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

/// A first-in first-out ring of at most `N` items, `N` zero included: the
/// bounded store of an [`EventQueueManager`], and so of the payloads of an
/// [`EventQueue`], under `no_std`.
///
/// The crate's own rather than `heapless::Deque`, whose constructor refuses a
/// depth of zero while the program is compiled: an internal queue the
/// generator proved empty has no slots (docs/adr/0076), and the payload store
/// beside its events has to have none either. The slots are uninitialised
/// until the ring writes them, so a ring of a zero-size item, a schemaless
/// machine's `()`, is its head and its length and nothing more.
///
/// Every `unsafe` of this type rests on the one invariant its fields keep:
/// `len <= N`, and the `len` slots from `head`, wrapping at the end, hold an
/// item each; no other slot does.
#[cfg(any(test, not(feature = "std")))]
struct FixedRing<T, const N: usize> {
    slots: [MaybeUninit<T>; N],
    head: u16,
    len: u16,
}

#[cfg(any(test, not(feature = "std")))]
impl<T, const N: usize> FixedRing<T, N> {
    /// An empty ring; a constant expression.
    const fn new() -> Self {
        // SAFETY: an array of `MaybeUninit` asks nothing of its bytes, so an
        // uninitialised one is a valid value of it; the ring reads no slot
        // before it writes one.
        let slots = unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() };
        Self {
            slots,
            head: 0,
            len: 0,
        }
    }

    /// How many items the ring holds.
    fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether the ring holds no item.
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Put `item` behind the items held; past the `N`th, stop the program
    /// rather than drop it.
    fn push_back(&mut self, item: T) {
        let count = self.len();
        let capacity = self.slots.len();
        if count == capacity {
            queue_full();
        }
        // `count < capacity`, so the slot is one no item is held in: writing
        // it forgets nothing.
        self.slots[(usize::from(self.head) + count) % capacity] = MaybeUninit::new(item);
        self.len += 1;
    }

    /// Take the item at the front, `None` when there is none.
    fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let first = usize::from(self.head);
        // SAFETY: the ring holds an item, so the slot at `head` holds the
        // first; it is read out once, and the slot is counted vacant below, so
        // it is never read again.
        let item = unsafe { self.slots[first].assume_init_read() };
        self.head = ((first + 1) % self.slots.len()) as u16;
        self.len -= 1;
        Some(item)
    }

    /// The items held, front first, left in place.
    fn iter(&self) -> impl Iterator<Item = &T> {
        let head = usize::from(self.head);
        let capacity = self.slots.len();
        (0..self.len()).map(move |offset| {
            // SAFETY: `offset < len`, so the slot is one of the `len` from
            // `head` that hold an item, borrowed for as long as the ring is.
            unsafe { self.slots[(head + offset) % capacity].assume_init_ref() }
        })
    }

    /// Drop every item held.
    fn clear(&mut self) {
        while self.pop_front().is_some() {}
    }
}

#[cfg(any(test, not(feature = "std")))]
impl<T, const N: usize> Drop for FixedRing<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(any(test, not(feature = "std")))]
impl<T: core::fmt::Debug, const N: usize> core::fmt::Debug for FixedRing<T, N> {
    /// The items held: the vacant slots hold nothing to show.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// FIFO queue for SCXML events with metadata.
///
/// Generic over the event type (typically `EventWithMetadata<E, P>` where `E`
/// is the policy's event enum) and the no_std FIFO depth `N`. Ports C++
/// `SCE::Core::EventQueueManager<T>`.
///
/// Implementation: under std, a thin wrapper around `VecDeque<T>`
/// (unbounded) — `N` is inert (the std build keeps the spec's unbounded
/// `Queue`). Under `--no-default-features`, a fixed ring of `N` slots held in
/// place, sized exactly to the queue's resolved depth, `0` included
/// (docs/adr/0076). `N` defaults to
/// [`crate::MAX_EVENT_QUEUE_DEPTH`] so a bare `EventQueueManager<T>` keeps the
/// crate baseline for schemaless/unspecified machines. Named to match C++ so
/// generated code reads naturally when cross-referenced with C++ source.
#[derive(Debug)]
pub struct EventQueueManager<T, const N: usize = { crate::MAX_EVENT_QUEUE_DEPTH }> {
    #[cfg(feature = "std")]
    queue: VecDeque<T>,
    #[cfg(not(feature = "std"))]
    queue: FixedRing<T, N>,
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
            queue: FixedRing::new(),
        }
    }

    /// §scxml-3.12.1: Enqueue an event at the back of the FIFO queue.
    ///
    /// Matches C++ `raise(T&&)`. Under `--no-default-features` an attempted push
    /// past the machine's resolved depth `N` panics rather than silently
    /// dropping the event — W3C SCXML Appendix D processes every queued event,
    /// so the bounded backing fails loud rather than drop.
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
/// [`EventQueue`] at the queue's resolved depth (see
/// [`crate::StatePolicy::InternalQueue`] and
/// [`crate::StatePolicy::ExternalQueue`], two types since docs/adr/0076), its
/// events through a view without that depth (docs/adr/0068).
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
        let held: Vec<u16> = ring_queued(&slots, head, len).copied().collect();
        assert_eq!(
            held,
            [3, 4, 5],
            "what a saved state reads is the FIFO order"
        );
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

    /// A ring of no slots, the internal queue of a document that raises
    /// nothing for itself (docs/adr/0076), holds nothing and takes nothing
    /// off: no index of it is taken modulo its capacity of zero.
    #[test]
    fn a_ring_of_no_slots_holds_nothing_and_takes_nothing_off() {
        let mut slots: [u16; 0] = [];
        let (mut head, mut len) = (0u16, 0u16);
        assert_eq!(ring_pop(&mut slots, &mut head, &mut len), None);
        assert_eq!(ring_front(&slots, head, len), None);
        assert_eq!(ring_queued(&slots, head, len).count(), 0);
    }

    /// A push onto a ring of no slots stops the program, as a push past any
    /// ring's depth does: the generator proved it never happens.
    #[test]
    #[should_panic(expected = "an event queue is full")]
    fn a_push_onto_a_ring_of_no_slots_is_refused_loud() {
        let mut slots: [u16; 0] = [];
        let (mut head, mut len) = (0u16, 0u16);
        ring_push(&mut slots, &mut head, &mut len, 1);
    }

    /// The payload store of a queue of depth zero is a ring of no slots too,
    /// which `heapless::Deque` refused to be while the program was compiled.
    #[test]
    fn a_payload_ring_of_no_slots_holds_nothing() {
        let mut ring: FixedRing<u32, 0> = FixedRing::new();
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
        assert_eq!(ring.pop_front(), None);
        assert_eq!(ring.iter().count(), 0);
    }

    #[test]
    #[should_panic(expected = "an event queue is full")]
    fn a_push_onto_a_payload_ring_of_no_slots_is_refused_loud() {
        let mut ring: FixedRing<(), 0> = FixedRing::new();
        ring.push_back(());
    }

    /// The payload ring keeps FIFO order across its wrap-around, refuses a
    /// push past its depth, and drops every item it still holds.
    #[test]
    fn the_payload_ring_keeps_fifo_order_and_drops_what_it_holds() {
        let mut ring: FixedRing<u32, 3> = FixedRing::new();
        for item in [1, 2, 3] {
            ring.push_back(item);
        }
        assert_eq!(ring.pop_front(), Some(1));
        assert_eq!(ring.pop_front(), Some(2));
        ring.push_back(4);
        ring.push_back(5);
        assert_eq!(ring.len(), 3);
        let held: Vec<u32> = ring.iter().copied().collect();
        assert_eq!(held, [3, 4, 5]);
        assert_eq!(format!("{ring:?}"), "[3, 4, 5]");
        ring.clear();
        assert!(ring.is_empty());
        assert_eq!(ring.pop_front(), None);

        let owner = std::rc::Rc::new(());
        {
            let mut held: FixedRing<std::rc::Rc<()>, 2> = FixedRing::new();
            held.push_back(std::rc::Rc::clone(&owner));
            held.push_back(std::rc::Rc::clone(&owner));
            assert_eq!(std::rc::Rc::strong_count(&owner), 3);
        }
        assert_eq!(
            std::rc::Rc::strong_count(&owner),
            1,
            "a ring dropped with items in it drops them"
        );
    }

    /// The queue type of depth zero is a constant, as every queue's empty
    /// value is (docs/adr/0070), and an empty one. On the host profile its
    /// events are a `VecDeque`, so only the bare-metal ring above refuses a
    /// push.
    #[test]
    fn a_queue_of_depth_zero_is_an_empty_constant() {
        let mut queue = <EventQueue<(), 0> as EventQueueStorage<()>>::EMPTY;
        assert!(!queue.has_events());
        assert_eq!(queue.queued().count(), 0);
        assert!(queue.events().pop().is_none());
        assert!(!queue.events().has_events());
        assert!(queue.pop_payload().is_none());
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
