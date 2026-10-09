// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `intrusive` storage mode (SCE Protocol-Synthesis RFC §synth-5-P):
//! Vyukov's intrusive MPSC list, in which each element carries its own link,
//! the caller owns every element, and a push cannot fail.
//!
//! **The link is an index.** The element document names a `u32` field
//! (`<sce:intrusive link-field="next"/>`), and the queue uses that field in
//! place as the index of the node that follows. The nodes are an array the
//! caller owns, and a node is named by its position in it. An index fits the
//! field on a 64-bit target where a pointer does not, and it means the same in
//! every process that maps the array, which is what the mesh's shared-memory
//! channel needs (RFC §synth-5-P, Migration).
//!
//! **The algorithm.** `tail` names the newest node. A push links its node into
//! the list in two steps — it exchanges itself onto `tail`, then stores its own
//! index into the old tail's link — and a pop walks from the oldest node.
//!
//! The caller gets back the very node it pushed and may reuse it at once, which
//! a queue with a dummy node cannot give (the node that carried the returned
//! value would stay behind as the new dummy). The list keeps a *stub* node for
//! this, and the pop of the last node puts the stub behind it, through the same
//! exchange the producers use, so that the last node has a successor and can be
//! returned. The stub is a node of the caller's array that the caller sets aside
//! at construction and never pushes.
//!
//! **Progress.** Push is wait-free: one exchange and one store, whatever the
//! others are doing. Pop is `blocking`, and that is not a defect of this
//! implementation: a producer stopped between its exchange and its store hides
//! every node pushed after it from the consumer, because the consumer reaches
//! them only through the link the stopped producer has not stored, and in that
//! window `try_pop` returns `None` while pushes that completed sit behind it.
//! The queue's histories are judged with `empty_pops`
//! `while-a-push-is-in-flight` for that reason.
//!
//! **Many consumers** are serialised by a test-and-set flag: a pop first takes
//! the flag, so only one walks the list at a time. A consumer that finds the flag
//! taken waits for the holder to finish, which is the other half of why a pop
//! is `blocking`.
//!
//! **Ordering.** A push writes its node, exchanges with AcqRel, and stores the
//! predecessor's link with Release; a pop reads a link with Acquire, so a
//! node's contents written before its push happen-before the pop that returns it.
//! The consumer's own position is private to it (to the flag's holder, with many
//! consumers) and is ordered by the flag's Acquire/Release pair.

use core::marker::PhantomData;

use super::sync::{AtomicU32, Ordering};
use super::Padded;

/// The value of a link that names no node.
pub const NIL: u32 = u32::MAX;

/// The link a queue uses in place in nodes of type `T`: a marker type, one per
/// queue, and not a property of `T`. A node type may serve several queues, each
/// linking through its own field, which a trait on the node could not say (one
/// impl per type), and two queues over one type with the same field would
/// conflict.
///
/// # Safety
///
/// `OFFSET` must be the byte offset, within `T`, of a `u32` field that is
/// aligned for an `AtomicU32`, and nothing but the queue the marker belongs to
/// may read or write that field while the node is in that queue.
pub unsafe trait Link<T> {
    /// The byte offset of the link field within the node.
    const OFFSET: usize;
}

/// Vyukov's intrusive MPSC list over a caller-owned array of nodes.
///
/// `MANY_CONSUMERS` serialises the consumers with a test-and-set flag; with
/// `false`, exactly one thread may call [`Mpsc::try_pop`] at a time, which the
/// caller guarantees (the generated consumer handle is not `Clone`).
pub struct Mpsc<T, L: Link<T>, const MANY_CONSUMERS: bool> {
    /// The newest node. Exchanged by every push and by the stub's.
    tail: Padded<AtomicU32>,
    /// The oldest node not yet returned. Private to the one consumer, or to the
    /// holder of `busy`.
    head: Padded<AtomicU32>,
    /// The test-and-set flag that serialises many consumers. Unused with one.
    busy: Padded<AtomicU32>,
    /// The stub: a node of the array that is never pushed by a caller.
    stub: u32,
    nodes: *mut T,
    len: u32,
    _owns: PhantomData<(T, L)>,
}

// SAFETY: the queue hands nodes from the threads that push them to the thread
// that pops them, so a node must be `Send`; every access to a node's link is
// atomic, and a node's other fields are the caller's, handed over by the
// Release/Acquire pair on the link (module documentation).
unsafe impl<T: Send, L: Link<T>, const MANY_CONSUMERS: bool> Send for Mpsc<T, L, MANY_CONSUMERS> {}
unsafe impl<T: Send, L: Link<T>, const MANY_CONSUMERS: bool> Sync for Mpsc<T, L, MANY_CONSUMERS> {}

impl<T, L: Link<T>, const MANY_CONSUMERS: bool> Mpsc<T, L, MANY_CONSUMERS> {
    /// An empty list over `len` nodes at `nodes`, with node `stub` set aside.
    ///
    /// # Safety
    ///
    /// `nodes` must be valid for reads and writes of `len` consecutive `T`s for
    /// as long as the queue exists, with no reference to a node alive across a
    /// call that moves it between the caller and the queue; `stub < len`; and
    /// `len < NIL`, so that no index is the value that means "none".
    pub unsafe fn new(nodes: *mut T, len: u32, stub: u32) -> Self {
        assert!(stub < len && len < NIL, "the stub is a node of the array");
        let queue = Self {
            tail: Padded(AtomicU32::new(stub)),
            head: Padded(AtomicU32::new(stub)),
            busy: Padded(AtomicU32::new(0)),
            stub,
            nodes,
            len,
            _owns: PhantomData,
        };
        queue.link(stub).store(NIL, Ordering::Relaxed);
        queue
    }

    /// The number of nodes in the array, the stub included.
    pub fn nodes(&self) -> u32 {
        self.len
    }

    /// The node that is the stub.
    pub fn stub(&self) -> u32 {
        self.stub
    }

    /// The link of node `index`.
    fn link(&self, index: u32) -> &AtomicU32 {
        debug_assert!(index < self.len, "a node index is within the array");
        // SAFETY: `new`'s contract makes `nodes` valid for `len` nodes, and
        // `Link` makes `OFFSET` the offset of an aligned `u32` that only the
        // queue touches while the node is in it. `AtomicU32` has the layout of
        // `u32`.
        unsafe {
            &*self
                .nodes
                .cast::<u8>()
                .add(index as usize * core::mem::size_of::<T>() + L::OFFSET)
                .cast::<AtomicU32>()
        }
    }

    /// Put node `index` on the queue. Wait-free; it cannot fail.
    ///
    /// # Safety
    ///
    /// `index` must name a node of the array that is not the stub and is not in
    /// the queue, and the caller must have finished writing the node (its
    /// contents are published to the consumer by this call).
    pub unsafe fn push(&self, index: u32) {
        debug_assert!(index != self.stub, "the stub is never pushed by a caller");
        self.append(index);
    }

    /// Link `index` behind the newest node.
    fn append(&self, index: u32) {
        self.link(index).store(NIL, Ordering::Relaxed);
        // After this exchange `index` is the newest node and `previous` is
        // behind it, whatever the other producers do. Until the store below the
        // consumer cannot reach `index`.
        let previous = self.tail.0.swap(index, Ordering::AcqRel);
        self.link(previous).store(index, Ordering::Release);
    }

    /// Take the oldest node, or `None` when the queue holds none the consumer
    /// can reach. `None` is also the answer while a producer is between its two
    /// steps (module documentation).
    ///
    /// With many consumers the call first takes the flag and waits for its
    /// holder. With one consumer, only that consumer may call it.
    pub fn try_pop(&self) -> Option<u32> {
        if MANY_CONSUMERS {
            while self.busy.0.swap(1, Ordering::Acquire) != 0 {
                core::hint::spin_loop();
            }
            let popped = self.pop();
            self.busy.0.store(0, Ordering::Release);
            popped
        } else {
            self.pop()
        }
    }

    /// The walk, run by one consumer at a time.
    fn pop(&self) -> Option<u32> {
        let mut head = self.head.0.load(Ordering::Relaxed);
        let mut next = self.link(head).load(Ordering::Acquire);
        if head == self.stub {
            if next == NIL {
                return None;
            }
            self.head.0.store(next, Ordering::Relaxed);
            head = next;
            next = self.link(head).load(Ordering::Acquire);
        }
        if next != NIL {
            self.head.0.store(next, Ordering::Relaxed);
            return Some(head);
        }
        // `head` has no successor: it is the newest node, or a producer is
        // between its exchange and its store.
        if head != self.tail.0.load(Ordering::Acquire) {
            return None;
        }
        // It is the newest node. Put the stub behind it so that it has a
        // successor and can be returned.
        self.append(self.stub);
        next = self.link(head).load(Ordering::Acquire);
        if next != NIL {
            self.head.0.store(next, Ordering::Relaxed);
            return Some(head);
        }
        None
    }
}
