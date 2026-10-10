// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reclamation domain of a `segmented` queue (SCE Protocol-Synthesis RFC
//! §synth-5-P, *Reclamation domain*): hazard pointers, sized statically from
//! the number of participants and built by the caller.
//!
//! A segment can still be read by one participant after another has taken it
//! out of the queue, so it cannot be freed at that moment. Each participant
//! publishes, in a slot of its own, the one segment it is about to touch
//! ([`Hazard::protect`]); the thread that unlinks a segment hands it to the
//! domain ([`HazardDomain::retire`]); and the domain frees it only when no slot
//! names it. A participant that stalls therefore holds back the segments its
//! slot names and nothing else, which is why hazard pointers were chosen over
//! epochs: the memory a stalled participant keeps unreclaimed is bounded, and
//! so the domain needs no storage that grows (RFC §synth-5-P).
//!
//! **No allocation.** The domain is a fixed array of `H` slots, and the
//! retired segments wait on an intrusive list threaded through a header each
//! segment carries ([`Retired`]), so the domain itself needs neither a heap nor
//! a bound on the list. What a retired segment does when it is finally freed is
//! the retiring queue's: the header names the function to call and the context
//! it is called with (the queue's allocator), so one domain serves queues of
//! different element types.
//!
//! **Progress.** `protect` and `retire` are lock-free: a retry means another
//! participant moved the pointer or pushed a node. The scan that frees nodes is
//! run by one thread at a time; a thread that finds a scan running does not wait
//! for it, it returns and leaves the nodes on the list, so no operation of a
//! queue waits for another participant on account of the domain.
//!
//! **Ordering.** A hazard is published with a sequentially consistent store and
//! re-validated with a sequentially consistent load of the pointer it guards,
//! and the scan reads every hazard with a sequentially consistent load after the
//! node was unlinked. Either the scan sees the hazard and keeps the node, or the
//! participant's validation sees the pointer already moved and does not use it.
//!
//! One domain per queue is the intended use. A domain several queues share holds
//! the nodes of all of them; what a node is freed with is the allocator it names,
//! which the queue borrows for as long as the domain may hold the node, so the
//! queue itself can move. A queue that is dropped flushes the domain
//! ([`HazardDomain::flush`]) before its allocator can go.

use core::ptr;

use super::sync::{fence, AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use super::Padded;

/// The header a retired object carries so the domain can chain it and, once no
/// hazard names it, hand it back to what allocated it.
///
/// The object that embeds it must have the header as its first field
/// (`#[repr(C)]`): a hazard names the object by its address, and the scan
/// compares that address with the header's.
pub struct Retired {
    /// The next retired object. Written only by the domain.
    next: AtomicPtr<Retired>,
    /// What frees the object. Written by [`HazardDomain::retire`] before the
    /// object is published to the list, read by the scan after it took the list.
    reclaim: Option<unsafe fn(*mut Retired, *const ())>,
    /// The value `reclaim` is called with: the allocator that gave the object.
    context: *const (),
}

impl Retired {
    /// A header for an object that is not retired.
    pub fn new() -> Self {
        Self {
            next: AtomicPtr::new(ptr::null_mut()),
            reclaim: None,
            context: ptr::null(),
        }
    }
}

impl Default for Retired {
    fn default() -> Self {
        Self::new()
    }
}

/// A fixed set of `H` hazard slots and the list of objects waiting to be freed.
///
/// `H` is the most participants that hold a handle on a queue at the same time,
/// both sides together. [`HazardDomain::acquire`] hands out a slot and refuses
/// the request past `H`.
pub struct HazardDomain<const H: usize> {
    /// The object each participant is currently touching, or null.
    hazards: [Padded<AtomicPtr<()>>; H],
    /// Which slots a participant holds.
    taken: [AtomicBool; H],
    /// Retired objects not yet freed.
    retired: AtomicPtr<Retired>,
    /// How many are on `retired`; a scan runs when it reaches twice the slots,
    /// so the unreclaimed memory is bounded by the participants.
    waiting: AtomicUsize,
    /// A scan is running.
    scanning: AtomicBool,
}

// SAFETY: every field is an atomic; the raw pointers a retired header carries
// are written before the header is published and read after it was taken, which
// the Release compare-and-swap that publishes it and the Acquire exchange that
// takes the list order.
unsafe impl<const H: usize> Sync for HazardDomain<H> {}
unsafe impl<const H: usize> Send for HazardDomain<H> {}

impl<const H: usize> HazardDomain<H> {
    /// How many retired objects wait before a scan runs.
    const SCAN_AT: usize = {
        assert!(H > 0, "a domain needs at least one slot");
        2 * H
    };

    /// An empty domain.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        let _ = Self::SCAN_AT;
        Self {
            hazards: [const { Padded(AtomicPtr::new(ptr::null_mut())) }; H],
            taken: [const { AtomicBool::new(false) }; H],
            retired: AtomicPtr::new(ptr::null_mut()),
            waiting: AtomicUsize::new(0),
            scanning: AtomicBool::new(false),
        }
    }

    /// An empty domain. Loom's atomics have no `const` constructor, so the
    /// loom build's is an ordinary function.
    #[cfg(loom)]
    pub fn new() -> Self {
        let _ = Self::SCAN_AT;
        Self {
            hazards: core::array::from_fn(|_| Padded(AtomicPtr::new(ptr::null_mut()))),
            taken: core::array::from_fn(|_| AtomicBool::new(false)),
            retired: AtomicPtr::new(ptr::null_mut()),
            waiting: AtomicUsize::new(0),
            scanning: AtomicBool::new(false),
        }
    }

    /// How many participants the domain serves.
    pub const fn slots(&self) -> usize {
        H
    }

    /// A slot for one participant, or `None` when all `H` are taken. Dropping the
    /// guard gives the slot back.
    pub fn acquire(&self) -> Option<Hazard<'_, H>> {
        for (index, taken) in self.taken.iter().enumerate() {
            if taken
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return Some(Hazard {
                    domain: self,
                    index,
                });
            }
        }
        None
    }

    /// Hand `node`, already unlinked from every structure a participant can
    /// reach it through, to the domain. It is freed by `reclaim(node, context)`
    /// once no hazard names it, now or at a later retirement.
    ///
    /// # Safety
    ///
    /// `node` points to the [`Retired`] header at the start of a live object that
    /// is no longer reachable except through a hazard already published; it is
    /// retired once; `reclaim` frees exactly that object; and `context` stays
    /// valid until it has.
    pub unsafe fn retire(
        &self,
        node: *mut Retired,
        reclaim: unsafe fn(*mut Retired, *const ()),
        context: *const (),
    ) {
        // SAFETY: the caller's contract makes the header live and, before it is
        // published below, ours alone.
        unsafe {
            ptr::addr_of_mut!((*node).reclaim).write(Some(reclaim));
            ptr::addr_of_mut!((*node).context).write(context);
        }
        self.push(node, node);
        if self.waiting.fetch_add(1, Ordering::AcqRel) + 1 >= Self::SCAN_AT {
            self.scan();
        }
    }

    /// Push the chain `first ..= last` (linked through `next`) onto the list.
    fn push(&self, first: *mut Retired, last: *mut Retired) {
        let mut head = self.retired.load(Ordering::Relaxed);
        loop {
            // SAFETY: `last` is a live header of a chain this thread owns until
            // the compare-and-swap below publishes it.
            unsafe { (*last).next.store(head, Ordering::Relaxed) };
            match self.retired.compare_exchange_weak(
                head,
                first,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(now) => head = now,
            }
        }
    }

    /// Free every retired object no hazard names. A thread that finds a scan
    /// already running returns: the objects wait for the next one.
    fn scan(&self) {
        if self.scanning.swap(true, Ordering::Acquire) {
            return;
        }
        self.scan_held();
    }

    /// The scan, run by the thread that holds `scanning`; it gives the flag back.
    fn scan_held(&self) {
        let mut list = self.retired.swap(ptr::null_mut(), Ordering::Acquire);
        // The objects on `list` were unlinked before they were retired, so a
        // participant that has not published a hazard for one by now will find
        // the pointer moved when it validates (module documentation).
        let mut named = [ptr::null_mut::<()>(); H];
        for (slot, hazard) in named.iter_mut().zip(self.hazards.iter()) {
            *slot = hazard.0.load(Ordering::SeqCst);
        }
        let mut kept: *mut Retired = ptr::null_mut();
        let mut kept_last: *mut Retired = ptr::null_mut();
        let mut freed = 0usize;
        while !list.is_null() {
            let node = list;
            // SAFETY: every node on the list is a live retired header, and this
            // thread took the whole list.
            list = unsafe { (*node).next.load(Ordering::Relaxed) };
            if named.contains(&(node as *mut ())) {
                // SAFETY: as above; the node leaves this local chain only by
                // `push` below.
                unsafe { (*node).next.store(kept, Ordering::Relaxed) };
                if kept.is_null() {
                    kept_last = node;
                }
                kept = node;
            } else {
                // SAFETY: no hazard names it, nothing else reaches it, and the
                // retirement recorded how to free it.
                unsafe { Self::reclaim(node) };
                freed += 1;
            }
        }
        if !kept.is_null() {
            self.push(kept, kept_last);
        }
        if freed > 0 {
            self.waiting.fetch_sub(freed, Ordering::AcqRel);
        }
        self.scanning.store(false, Ordering::Release);
    }

    /// Free `node` as its retirement said.
    ///
    /// # Safety
    ///
    /// `node` was taken off the list by this thread and nothing reaches it.
    unsafe fn reclaim(node: *mut Retired) {
        // SAFETY: the header was written by `retire` before it was published.
        let (reclaim, context) = unsafe { ((*node).reclaim, (*node).context) };
        if let Some(reclaim) = reclaim {
            // SAFETY: `retire`'s contract.
            unsafe { reclaim(node, context) };
        }
    }

    /// Free what can be freed now: the retired objects no hazard names, unless a
    /// scan is already running, which is then left to finish. A participant that
    /// would rather not hold memory back until the next retirement calls it; the
    /// loom models call it to reach the free path, which a few retirements never
    /// would (the scan runs when twice the slots are waiting). Lock-free: it never
    /// waits for another scan.
    pub fn collect(&self) {
        self.scan();
    }

    /// Free every retired object no hazard names, waiting for a scan that is
    /// running instead of leaving the objects to it. A queue being dropped calls
    /// it: no participant of that queue is alive, so none of its segments is
    /// named, and every one of them is freed before the queue's allocator can go.
    /// Blocking, which a drop may be.
    pub fn flush(&self) {
        while self.scanning.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        self.scan_held();
    }

    /// How many retired objects are waiting to be freed. For tests and for a
    /// target that budgets: it is bounded by the slots (module documentation).
    pub fn waiting(&self) -> usize {
        self.waiting.load(Ordering::Acquire)
    }
}

#[cfg(not(loom))]
impl<const H: usize> Default for HazardDomain<H> {
    fn default() -> Self {
        Self::new()
    }
}

/// A participant's slot in a domain. It holds the slot until it is dropped, and
/// publishes what its owner is about to touch.
pub struct Hazard<'d, const H: usize> {
    domain: &'d HazardDomain<H>,
    index: usize,
}

impl<const H: usize> Hazard<'_, H> {
    fn slot(&self) -> &AtomicPtr<()> {
        &self.domain.hazards[self.index].0
    }

    /// Read `source` and protect what it points to: publish it, then check that
    /// `source` still holds it, and try again if not. The pointer returned is
    /// not freed while the hazard names it, unless it is null.
    pub fn protect<T>(&self, source: &AtomicPtr<T>) -> *mut T {
        let mut pointer = source.load(Ordering::Acquire);
        loop {
            self.slot().store(pointer.cast(), Ordering::SeqCst);
            fence(Ordering::SeqCst);
            let again = source.load(Ordering::SeqCst);
            if again == pointer {
                return pointer;
            }
            pointer = again;
        }
    }

    /// Stop protecting what the hazard names. Called when the participant is
    /// done with it, so it does not hold back the segment longer than it uses it.
    pub fn clear(&self) {
        self.slot().store(ptr::null_mut(), Ordering::Release);
    }
}

impl<const H: usize> Drop for Hazard<'_, H> {
    fn drop(&mut self) {
        self.clear();
        self.domain.taken[self.index].store(false, Ordering::Release);
    }
}
