// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `intrusive` storage mode (SCE Protocol-Synthesis RFC §synth-5-P):
//! Vyukov's intrusive MPSC list over an array of [`Node`]s the caller owns, in
//! which a push cannot fail and the caller gets back the very node it pushed.
//!
//! **The node is the list's, the value is the caller's** (ADR 0088). A
//! [`Node<T>`] holds the element as its `value` and, after it, the link the list
//! threads through it. The link is the list's *hook*: it lives in the node, but
//! it is not part of the value, so the element type declares no link, and its
//! encode, decode, copy and comparison never see one. Only this module reads or
//! writes a link, and only as the atomic object it is.
//!
//! **The link is an index.** A node is named by its position in the caller's
//! array, and a link holds the index of the node that follows. An index fits 32
//! bits on a 64-bit target where a pointer does not, and it means the same in
//! every process that maps the array, which is what the mesh's shared-memory
//! channel needs (RFC §synth-5-P, Migration). `#[repr(C)]` fixes the link after
//! the value, so the array has one layout in every process; whether the value
//! itself may be shared between processes is a property of `T`.
//!
//! **One queue at a time.** A node has one hook, so it is in at most one queue
//! at a time. The node type depends only on the element type, so a node popped
//! from one queue may be pushed to another queue over the same element: a free
//! list and a work queue over one array is the usual use.
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
//! **Ordering.** A push writes its node's value, exchanges with AcqRel, and
//! stores the predecessor's link with Release; a pop reads a link with Acquire,
//! so a node's value written before its push happens-before the pop that
//! returns it. The consumer's own position is private to it (to the flag's
//! holder, with many consumers) and is ordered by the flag's Acquire/Release
//! pair.

use core::fmt;

use super::sync::{AtomicU32, Ordering};
use super::Padded;

/// The value of a link that names no node.
pub const NIL: u32 = u32::MAX;

/// A node of an intrusive list: the element it holds, and the link the list
/// threads through it.
///
/// `value` is the caller's. The link is the list's hook and is private to this
/// module: a node in no queue has the link [`NIL`], and while the node is in a
/// queue only that queue reads or writes it, atomically. A node is in at most
/// one queue at a time, and may move between queues over the same element.
///
/// The layout is fixed by `#[repr(C)]`: `value` first, the link after it, so an
/// array of nodes is laid out the same in every process that maps it.
///
/// A node has nothing that would copy or compare its link. It is not `Clone`:
/// a node is an object, named by its index in the array, and not a value; what
/// copies is the value it holds, `Node::new(node.value.clone())`, a node in no
/// queue. It is not `PartialEq`: values are compared, `a.value == b.value`. Its
/// `Debug` shows the value alone.
#[repr(C)]
pub struct Node<T> {
    /// The element the node holds. The queue never reads or writes it.
    pub value: T,
    /// The index of the node that follows, or [`NIL`]. Read and written only by
    /// the queue the node is in.
    next: AtomicU32,
}

impl<T> Node<T> {
    /// A node that holds `value` and is in no queue.
    #[cfg(not(loom))]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            next: AtomicU32::new(NIL),
        }
    }

    /// A node that holds `value` and is in no queue. Loom's atomics have no
    /// `const` constructor, so the loom build's is an ordinary function.
    #[cfg(loom)]
    pub fn new(value: T) -> Self {
        Self {
            value,
            next: AtomicU32::new(NIL),
        }
    }
}

impl<T: Default> Default for Node<T> {
    /// A node that holds `T`'s default value and is in no queue.
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for Node<T> {
    /// The value alone: the link is the list's, and reading it here would be a
    /// load of an atomic another thread may be storing to.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

/// Vyukov's intrusive MPSC list over a caller-owned array of [`Node<T>`].
///
/// `MANY_CONSUMERS` serialises the consumers with a test-and-set flag; with
/// `false`, exactly one thread may call [`Mpsc::try_pop`] at a time, which the
/// caller guarantees (the generated consumer handle is not `Clone`).
pub struct Mpsc<T, const MANY_CONSUMERS: bool> {
    /// The newest node. Exchanged by every push and by the stub's.
    tail: Padded<AtomicU32>,
    /// The oldest node not yet returned. Private to the one consumer, or to the
    /// holder of `busy`.
    head: Padded<AtomicU32>,
    /// The test-and-set flag that serialises many consumers. Unused with one.
    busy: Padded<AtomicU32>,
    /// The stub: a node of the array that is never pushed by a caller.
    stub: u32,
    nodes: *mut Node<T>,
    len: u32,
}

// SAFETY: the queue hands nodes from the threads that push them to the thread
// that pops them, so a value must be `Send`; every access to a node's link is
// atomic, and a node's value is the caller's, handed over by the
// Release/Acquire pair on the link (module documentation).
unsafe impl<T: Send, const MANY_CONSUMERS: bool> Send for Mpsc<T, MANY_CONSUMERS> {}
unsafe impl<T: Send, const MANY_CONSUMERS: bool> Sync for Mpsc<T, MANY_CONSUMERS> {}

impl<T, const MANY_CONSUMERS: bool> Mpsc<T, MANY_CONSUMERS> {
    /// An empty list over `len` nodes at `nodes`, with node `stub` set aside.
    ///
    /// # Safety
    ///
    /// `nodes` must be valid for reads and writes of `len` consecutive
    /// `Node<T>`s for as long as the queue exists, and while it exists the
    /// caller reaches a node only through `nodes` or a pointer derived from it,
    /// with no reference to a node alive across a call that moves it between
    /// the caller and the queue. The caller writes the value of a node only
    /// while the node is not in the queue, and never the stub's. `stub < len`,
    /// and `len < NIL`, so that no index is the value that means "none".
    pub unsafe fn new(nodes: *mut Node<T>, len: u32, stub: u32) -> Self {
        assert!(stub < len && len < NIL, "the stub is a node of the array");
        let queue = Self {
            tail: Padded(AtomicU32::new(stub)),
            head: Padded(AtomicU32::new(stub)),
            busy: Padded(AtomicU32::new(0)),
            stub,
            nodes,
            len,
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
        // SAFETY: `new`'s contract makes `nodes` valid for `len` nodes, and the
        // link is a field of the node that only the queue touches while the
        // node is in it. The place is borrowed alone, not the whole node, so
        // the caller's value is not aliased by it.
        unsafe { &(*self.nodes.add(index as usize)).next }
    }

    /// Put node `index` on the queue. Wait-free; it cannot fail.
    ///
    /// # Safety
    ///
    /// `index` must name a node of the array that is not the stub and is not in
    /// any queue, and the caller must have finished writing the node's value
    /// (it is published to the consumer by this call).
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
