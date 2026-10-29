// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounded broadcast log's one implementation (ADR 0062 sections 2 and 4),
//! generic over the width of the atomics it keeps its stamps, its head and its
//! words in. The rows a target gets are instantiations of it:
//! [`crate::broadcast_log`] is the 64-bit row and [`crate::broadcast_log_narrow`]
//! the row for a target without 64-bit atomics.
//!
//! The contract is the sequential log of ADR 0062 section 2: `h` entries have
//! been published, only the last [`BroadcastLog::RETENTION`] are retained, and a
//! reader owns a cursor. [`BroadcastLog::read`] returns the entry at the cursor,
//! says the log has nothing new, or says how many entries the reader missed and
//! moves it to the oldest retained. Publishing never fails and never waits, and
//! neither does a read: both are wait-free in a number of steps that depends on
//! the entry's width and on nothing the other side does.
//!
//! **The protocol** (section 4). A slot holds a stamp and the entry's words. The
//! publisher writes a slot as `stamp := busy(n)` (release), a release fence, the
//! words, `stamp := done(n)` (release), then `head := n + 1` (release). A reader
//! holding cursor `r` loads `head` (acquire), answers `Empty` at the head and
//! `Lagged` outside the window, and otherwise loads the stamp of slot `r mod S`,
//! copies the words, an acquire fence, and loads the stamp again: if it is still
//! `done(r)` the copy is entry `r`; if not, the publisher has reused the slot, and
//! the reader reloads `head` and answers `Lagged`. This is a sequence lock in the
//! form Boehm showed correct under the C++11 model ("Can seqlocks get along with
//! programming language memory models?", MSPC 2012). Every order is justified at
//! the access that carries it, and the loom models
//! (`backends/rust/forge-runtime/tests/loom_broadcast_log.rs` and
//! `loom_broadcast_log_narrow.rs`) run this text under every outcome the memory
//! model allows within three preemptions.
//!
//! **The slot count is `RETENTION + 1`**: the slot being overwritten is in flux and
//! cannot be promised to a reader.
//!
//! # Lanes: what a row varies
//!
//! A [`Lane`] fixes the atomic cell, the payload word and the stamp width `w` in
//! bits. Everything is written once, in terms of arithmetic modulo `2^w`
//! (`M = 2^w`): the stored stamps and head are the low `w` bits of the count they
//! stand for, a sequence number is a count modulo `M`, and a cursor lives in
//! `[0, M)`. The 64-bit lane is the case in which the count never reaches `M`
//! (the publisher stops at [`BroadcastLog::HORIZON`]).
//!
//! - The stamp of entry `n` is `busy(n) = (2n + 1) mod M` while it is written and
//!   `done(n) = (2n + 2) mod M` once it is, so a stamp tells apart `K = M / 2`
//!   consecutive entries of one slot: `done(n) = done(n')` for `n` and `n'` in one
//!   slot exactly when `n = n' (mod K)`.
//! - The head stores `(n + 1) mod M`. A reader does not extend it to the true count
//!   by any shared state: it takes `behind = (head - cursor) mod M`, which is the
//!   true distance whenever that distance is below `M`.
//! - The slot index is `n mod S` with `S = RETENTION + 1`. A reader's cursor is only
//!   known modulo `M`, so the index stays the publisher's across the wrap exactly
//!   when `S` divides `M`: a narrow log refuses a slot count that is not a power
//!   of two, and one above `K` (compile-time, [`BroadcastLog::new`]).
//!
//! **Theorem (the wrap bound).** Let a reader's cursor stand for the entry `c` (the
//! count, not its residue), and let `B(t)` be the number of publishes begun at
//! time `t` (the busy stamp of entry `B - 1` stored). Suppose that during each
//! `read` call, up to its last shared access at `t_e`, the cursor is no ahead of
//! the head (`c <= H`, `H` the number completed) and `B(t_e) - c <= K`. Then the
//! call answers as the specification of section 2 does at its linearization
//! point: `Empty` only when `c = H`, `Item` only with entry `c` whole and `c` in
//! the window, and `Lagged` with the exact count and the cursor moved to the
//! oldest retained.
//!
//! *Proof.* (a) The head extension is exact: `H(t) - c <= B(t) - c <= K < M`, so
//! `(head - cursor) mod M` is `H - c` itself. This gives `Empty` (a distance of 0)
//! and the range `Lagged` (a distance of at least `S`) their exact value, and the
//! reload after a mismatch likewise. (b) A stamp equal to `done(c)` is the store of
//! `done(c)` and no other. Another store in the slot has the same value only if it
//! is `done(c + jS)` with `jS` a positive multiple of `K` (as `S` divides `K`), and
//! that store follows the busy store of entry `c + jS >= c + K`, which makes
//! `B >= c + K + 1`, against `B(t_e) - c <= K`. The comparison of the two stamps
//! therefore means what it meant with no wrap, and the proof of the seqlock read
//! carries over unchanged. (c) A stamp that is not `done(c)` is a later store than
//! `done(c)` (the head load synchronizes with its store), so the busy stamp of
//! entry `c + S` has been stored, the publish that began it started after the head
//! reached `c + S`, and the reload sees `H >= c + S`: the lag is at least
//! `S - RETENTION = 1`. (d) `RETENTION + 1 = S` slots are what make the first
//! load of `head` the last instant at which `c` was certainly in the window, as
//! section 5 of the ADR argues. Nothing else of the 64-bit proof depends on the
//! width. *QED.*
//!
//! The hypothesis is the *wrap bound*. It follows from the one a deploy can state:
//! if the publisher begins at most `P` publishes between a reader's head load in
//! one read and its last access of the next, then `B(t_e) - c <= P + S` (a read
//! leaves the cursor within `RETENTION` of the head it loaded, and one publish may
//! be in flight), so the bound holds when `P <= K - S`. A reader that does not meet
//! it is not told so: see "Outside the bound" on [`BroadcastLog::read`].
//!
//! No global state, no threads, no allocation (SCE_FORGE.md section 2.1, C1/C2):
//! the log owns its storage and where it lives is the caller's choice.

#[cfg(target_has_atomic = "8")]
use crate::queue::sync::AtomicBool;
#[cfg(target_has_atomic = "64")]
use crate::queue::sync::AtomicU64;
use crate::queue::sync::{fence, AtomicUsize, Ordering};
use crate::queue::Padded;

mod sealed {
    pub trait Sealed {}
}

/// The width a log keeps its stamps, its head and its words in. Sealed: the rows
/// are [`Wide`] and [`Narrow`], and the proof in the module documentation is
/// written for exactly the arithmetic they share.
pub trait Lane: sealed::Sealed + 'static {
    /// The atomic cell the stamp, the head and every word are kept in.
    type Cell;
    /// A payload word as the caller supplies and receives it.
    type Word: Copy;
    /// The stamp's width in bits, `w`: the log counts modulo `2^w`.
    const BITS: u32;
    /// The stamp of a slot nothing has been written to. It is never a stamp the
    /// protocol writes into the slot the test of [`BroadcastLog::reader_oldest`]
    /// reads (see [`Narrow`]).
    const UNWRITTEN: u64;
    /// A cell holding 0.
    #[cfg(not(loom))]
    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO_CELL: Self::Cell;
    /// A cell holding [`Lane::UNWRITTEN`].
    #[cfg(not(loom))]
    #[allow(clippy::declare_interior_mutable_const)]
    const UNWRITTEN_CELL: Self::Cell;
    /// A cell holding `value`. Loom's atomics have no `const` constructor, so the
    /// loom build makes its cells here.
    #[cfg(loom)]
    fn cell(value: u64) -> Self::Cell;
    /// Load the cell, widened to a `u64`.
    fn load(cell: &Self::Cell, order: Ordering) -> u64;
    /// Store `value`, which the caller has reduced to what the cell holds.
    fn store(cell: &Self::Cell, value: u64, order: Ordering);
    /// A word as the bits a cell holds.
    fn word_to_bits(word: Self::Word) -> u64;
    /// The word a cell's bits stand for.
    fn word_from_bits(bits: u64) -> Self::Word;
}

/// The 64-bit lane: stamps, head and words are `AtomicU64`, and the count never
/// wraps (the publisher stops at [`BroadcastLog::HORIZON`], 292 years at a billion
/// publishes per second). Present where the target has 64-bit atomics.
#[cfg(target_has_atomic = "64")]
pub struct Wide;

#[cfg(target_has_atomic = "64")]
impl sealed::Sealed for Wide {}

#[cfg(target_has_atomic = "64")]
impl Lane for Wide {
    type Cell = AtomicU64;
    type Word = u64;
    const BITS: u32 = 64;
    // Neither stamp the protocol writes is 0: `busy(n)` is odd and `done(n)` is at
    // least 2.
    const UNWRITTEN: u64 = 0;
    #[cfg(not(loom))]
    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO_CELL: AtomicU64 = AtomicU64::new(0);
    #[cfg(not(loom))]
    #[allow(clippy::declare_interior_mutable_const)]
    const UNWRITTEN_CELL: AtomicU64 = AtomicU64::new(0);

    #[cfg(loom)]
    fn cell(value: u64) -> AtomicU64 {
        AtomicU64::new(value)
    }

    fn load(cell: &AtomicU64, order: Ordering) -> u64 {
        cell.load(order)
    }

    fn store(cell: &AtomicU64, value: u64, order: Ordering) {
        cell.store(value, order)
    }

    fn word_to_bits(word: u64) -> u64 {
        word
    }

    fn word_from_bits(bits: u64) -> u64 {
        bits
    }
}

/// The narrow lane: a stamp of `B` bits kept in a pointer-width atomic, over words
/// of 32 bits. It needs no 64-bit atomic and no read-modify-write instruction, so
/// it is on every target whose pointer is at least 32 bits, and `B` is a
/// parameter so that a model can use a stamp of two or three bits and reach its
/// wrap in a few publishes. A row keeps `B = 32`.
///
/// `B` is between 2 and 32. The slot count of a log over it is a power of two no
/// greater than `2^(B - 1)` ([`BroadcastLog::new`] refuses another).
///
/// [`Lane::UNWRITTEN`] is 1, which no stamp written into the *last* slot equals:
/// the slots are `S = 2^s` with `s >= 1`, the last holds the entries `n = S - 1
/// (mod S)`, whose stamps are `2n + 1 = 2S - 1` and `2n + 2 = 0` modulo `2S`, and
/// `2S` divides `2^B`; 1 is neither (`2S >= 4`). So a last slot that still reads
/// 1 has never been written, which is how [`BroadcastLog::reader_oldest`] knows
/// that fewer than `RETENTION` entries have been published, when a sequence
/// number that counts modulo `2^B` cannot say.
pub struct Narrow<const B: u32>;

impl<const B: u32> sealed::Sealed for Narrow<B> {}

impl<const B: u32> Lane for Narrow<B> {
    type Cell = AtomicUsize;
    type Word = u32;
    const BITS: u32 = {
        assert!(
            B >= 2,
            "a stamp needs a bit for busy against done and one for the lap"
        );
        assert!(
            B <= 32,
            "a narrow stamp is at most 32 bits: its words are 32-bit"
        );
        B
    };
    const UNWRITTEN: u64 = 1;
    #[cfg(not(loom))]
    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO_CELL: AtomicUsize = AtomicUsize::new(0);
    #[cfg(not(loom))]
    #[allow(clippy::declare_interior_mutable_const)]
    const UNWRITTEN_CELL: AtomicUsize = AtomicUsize::new(1);

    #[cfg(loom)]
    fn cell(value: u64) -> AtomicUsize {
        AtomicUsize::new(value as usize)
    }

    fn load(cell: &AtomicUsize, order: Ordering) -> u64 {
        cell.load(order) as u64
    }

    fn store(cell: &AtomicUsize, value: u64, order: Ordering) {
        cell.store(value as usize, order)
    }

    fn word_to_bits(word: u32) -> u64 {
        u64::from(word)
    }

    fn word_from_bits(bits: u64) -> u32 {
        bits as u32
    }
}

/// What a read found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// The entry `seq` was copied into the caller's buffer, and the reader now
    /// holds `seq + 1`. A sequence number counts modulo `2^w` of the log's lane
    /// (the count itself in the 64-bit row).
    Item { seq: u64 },
    /// The reader is at the head: nothing newer is published. Its cursor is
    /// unchanged.
    Empty,
    /// The reader had fallen behind the window and missed `skipped >= 1`
    /// entries. It now holds the oldest retained entry, so its next read
    /// returns that. The caller's buffer is unspecified.
    Lagged { skipped: u64 },
}

/// A reader's position, owned by the reader and shared with nothing.
///
/// Opening a reader costs the log nothing: the publisher does not know how many
/// readers exist, which is what keeps publishing wait-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reader {
    cursor: u64,
    lost: u64,
}

impl Reader {
    /// The sequence number this reader reads next, modulo `2^w`.
    pub const fn cursor(&self) -> u64 {
        self.cursor
    }

    /// The entries this reader has been told it missed, summed over its reads.
    pub const fn lost(&self) -> u64 {
        self.lost
    }
}

struct Slot<const W: usize, L: Lane> {
    stamp: L::Cell,
    words: [L::Cell; W],
}

/// A bounded log whose entries are `W` words of the lane `L`, kept in `SLOTS` slots
/// and retaining the last `SLOTS - 1` entries.
pub struct BroadcastLog<const W: usize, const SLOTS: usize, L: Lane> {
    /// The number of entries published, modulo `2^w`. Written only by the
    /// publisher, loaded by every reader, so it has a cache line of its own.
    head: Padded<L::Cell>,
    /// Whether a [`Publisher`] exists. Where the target has no 8-bit
    /// compare-and-swap there is none, and the one publisher is a matter of
    /// ownership ([`BroadcastLog::split`]).
    #[cfg(target_has_atomic = "8")]
    publisher_taken: AtomicBool,
    slots: [Slot<W, L>; SLOTS],
}

impl<const W: usize, const SLOTS: usize, L: Lane> BroadcastLog<W, SLOTS, L> {
    /// The entries a reader may rely on.
    pub const RETENTION: u64 = (SLOTS - 1) as u64;

    /// The most entries a log can publish when its count does not wrap: the 64-bit
    /// stamps are `2n + 2`, which must fit a `u64`. A narrow log counts modulo its
    /// stamp space and has no horizon beyond it, so this is the size of the
    /// space and no publish reaches it.
    pub const HORIZON: u64 = if L::BITS >= 64 {
        (u64::MAX - 2) / 2
    } else {
        1u64 << L::BITS
    };

    /// The low `w` bits.
    const MASK: u64 = if L::BITS >= 64 {
        u64::MAX
    } else {
        (1u64 << L::BITS) - 1
    };

    /// The number of values a stamp takes, `2^w`; 0 for the 64-bit lane, whose
    /// count is never taken modulo it.
    const SPACE: u64 = Self::MASK.wrapping_add(1);

    /// Half the stamp space, `K`: how many consecutive entries of one slot a stamp
    /// tells apart.
    const HALF: u64 = 1u64 << (L::BITS - 1);

    /// Evaluated wherever a log of this size is constructed, so a size the
    /// protocol cannot represent is a compile error and not a runtime one.
    const CHECKED: () = {
        assert!(
            SLOTS >= 2,
            "a log retains at least one entry, so it has at least two slots"
        );
        if L::BITS < 64 {
            assert!(
                SLOTS.is_power_of_two(),
                "a narrow stamp wraps, and the slot index n mod S stays continuous across the wrap only if S divides the stamp space, so S is a power of two"
            );
            assert!(
                SLOTS as u64 <= Self::HALF,
                "a narrow log keeps at most half the stamp space in slots, so that a stamp tells apart more than one lap of a slot"
            );
        }
    };

    /// An empty log.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        let () = Self::CHECKED;
        Self {
            head: Padded(L::ZERO_CELL),
            #[cfg(target_has_atomic = "8")]
            publisher_taken: AtomicBool::new(false),
            slots: [const {
                Slot {
                    stamp: L::UNWRITTEN_CELL,
                    words: [const { L::ZERO_CELL }; W],
                }
            }; SLOTS],
        }
    }

    /// An empty log. Loom's atomics have no `const` constructor, so the loom
    /// build's is an ordinary function.
    #[cfg(loom)]
    pub fn new() -> Self {
        let () = Self::CHECKED;
        Self {
            head: Padded(L::cell(0)),
            #[cfg(target_has_atomic = "8")]
            publisher_taken: AtomicBool::new(false),
            slots: core::array::from_fn(|_| Slot {
                stamp: L::cell(L::UNWRITTEN),
                words: core::array::from_fn(|_| L::cell(0)),
            }),
        }
    }

    /// The stamp a slot holds while the entry `n` is being written into it.
    const fn busy(n: u64) -> u64 {
        n.wrapping_mul(2).wrapping_add(1) & Self::MASK
    }

    /// The stamp a slot holds once the entry `n` is in it.
    const fn done(n: u64) -> u64 {
        n.wrapping_mul(2).wrapping_add(2) & Self::MASK
    }

    /// How far `cursor` is behind `head`, both counts modulo `2^w`: the true
    /// distance whenever that is below `2^w`.
    const fn distance(head: u64, cursor: u64) -> u64 {
        head.wrapping_sub(cursor) & Self::MASK
    }

    /// `cursor` moved `by` entries on, modulo `2^w`.
    const fn advance(cursor: u64, by: u64) -> u64 {
        cursor.wrapping_add(by) & Self::MASK
    }

    /// The number of entries published so far, modulo `2^w`: the count itself in
    /// the 64-bit row.
    pub fn head(&self) -> u64 {
        L::load(&self.head.0, Ordering::Acquire)
    }

    /// The publisher, or `None` while another exists. Dropping it frees the
    /// place, and its successor continues the sequence. Present where the target
    /// has an 8-bit compare-and-swap; the form that needs none is
    /// [`BroadcastLog::split`].
    #[cfg(target_has_atomic = "8")]
    pub fn publisher(&self) -> Option<Publisher<'_, W, SLOTS, L>> {
        if self.publisher_taken.swap(true, Ordering::Acquire) {
            return None;
        }
        Some(Publisher::resume(self))
    }

    /// The one publisher of a log, and a shared view of the log for reading, taken
    /// by ownership: the exclusive borrow is the proof that there is no other
    /// publisher, so the algorithm's one assumption about its writer is a
    /// statement of the type system and needs no read-modify-write instruction.
    /// The publisher's successor, taken after both have been dropped, continues
    /// the sequence.
    ///
    /// ```ignore
    /// let (mut publisher, log) = log.split();
    /// let mut reader = log.reader_head();
    /// ```
    pub fn split(&mut self) -> (Publisher<'_, W, SLOTS, L>, &Self) {
        // Where there is a flag, a publisher raised by `split` holds it too, so a
        // second one through the shared view is refused as it is for `publisher`.
        #[cfg(target_has_atomic = "8")]
        self.publisher_taken.store(true, Ordering::Relaxed);
        let shared: &Self = self;
        (Publisher::resume(shared), shared)
    }

    /// The one publisher of a log that cannot be borrowed exclusively, a log in a
    /// `static`, taken on the caller's word.
    ///
    /// # Safety
    ///
    /// The caller guarantees, for as long as the returned publisher exists:
    ///
    /// - no other [`Publisher`] of this log exists, whether from this function, from
    ///   [`BroadcastLog::split`] or from `publisher`: two publishers write the
    ///   same slots and break the stamp protocol silently, with no data race but with
    ///   readers told entries that were never published or lags that skipped
    ///   nothing;
    /// - the last `publish` of any earlier publisher of this log happens before this
    ///   call (program order, a thread join, or a release store the caller
    ///   acquires), so that the sequence this publisher continues from `head` is
    ///   the one that was published.
    pub unsafe fn publisher_unchecked(&self) -> Publisher<'_, W, SLOTS, L> {
        #[cfg(target_has_atomic = "8")]
        self.publisher_taken.store(true, Ordering::Relaxed);
        Publisher::resume(self)
    }

    /// A reader at the oldest retained entry.
    ///
    /// The oldest retained entry is `max(0, h - RETENTION)`. A count modulo `2^w`
    /// cannot tell a log that has published fewer than `RETENTION` entries from
    /// one that has published more and wrapped, so the log says it itself: the last
    /// slot is first written by the publish of entry `RETENTION`, and still holds
    /// [`Lane::UNWRITTEN`] before it. If it does, `h <= RETENTION` when it was
    /// loaded and the oldest retained is entry 0, whose cursor is 0 (a count that
    /// has not wrapped). If it does not, the publish of entry `RETENTION` began, so
    /// `h >= RETENTION`, and the oldest is `h - RETENTION` of the head loaded after.
    pub fn reader_oldest(&self) -> Reader {
        // A 64-bit count does not wrap, so the oldest retained entry is the head
        // less the retention, no lower than 0: one load, as the first version of
        // this row had it. Asking the last slot as well adds a load that no
        // outcome depends on here, and under loom it multiplies the schedules of
        // every model that opens a reader (the wide `two_readers_each_hold_their_own_cursor`
        // went from 0.7 s to 187 s when it did). Only a narrow count needs it.
        if L::BITS >= 64 {
            return Reader {
                cursor: self.head().saturating_sub(Self::RETENTION),
                lost: 0,
            };
        }
        let untouched = L::load(&self.slots[SLOTS - 1].stamp, Ordering::Acquire) == L::UNWRITTEN;
        let cursor = if untouched {
            0
        } else {
            Self::distance(self.head(), Self::RETENTION)
        };
        Reader { cursor, lost: 0 }
    }

    /// A reader at the head: it sees only what is published after this call.
    pub fn reader_head(&self) -> Reader {
        Reader {
            cursor: self.head(),
            lost: 0,
        }
    }

    /// A reader at the sequence number `seq`, or at the head when `seq` is past
    /// it.
    ///
    /// In the 64-bit row `seq` is a count and is clamped to the head. A narrow
    /// log knows a position only modulo `2^w`, so `seq` is reduced to it and read
    /// as an entry the caller got from this log (a publish's return or an item's
    /// `seq`): fewer than `K = 2^(w-1)` entries behind the head it is taken as it
    /// is, and any other position (`K` or more behind, which is what one ahead of
    /// the head looks like) is the head.
    pub fn reader_at(&self, seq: u64) -> Reader {
        let head = self.head();
        let cursor = if L::BITS >= 64 {
            seq.min(head)
        } else {
            let seq = seq & Self::MASK;
            if Self::distance(head, seq) < Self::HALF {
                seq
            } else {
                head
            }
        };
        Reader { cursor, lost: 0 }
    }

    /// Read the entry at `reader`'s cursor into `out`. A bounded number of
    /// steps whatever the publisher is doing: a copy that the publisher
    /// overwrote is not retried, it is the answer ([`Read::Lagged`]).
    ///
    /// # Outside the bound
    ///
    /// The answer is what the theorem in the module documentation says while the
    /// reader is less than `K = 2^(w-1)` publishes behind (counting the one in
    /// flight) during the call. A narrow reader cannot know it has broken that
    /// hypothesis; what it does when it has is defined, not undefined, and is the
    /// arithmetic of its cursor modulo `2^w`:
    ///
    /// - its cursor is always right modulo `2^w`: `Empty` leaves it at the head's
    ///   residue and `Lagged` puts it at the oldest retained entry's;
    /// - `skipped` is the distance taken modulo `2^w` less the retention, so it is
    ///   short by a multiple of `2^w` when the reader was that many laps behind, and
    ///   a reader exactly `2^w` behind is told `Empty`;
    /// - a copy that validated is the entry a later lap wrote into the slot when the
    ///   reader was `K` or more behind while it read, and is then handed as the
    ///   entry the reader wanted (the stamp is the same value one lap on).
    ///
    /// The deploy states the bound it relies on, and the generator refuses a log
    /// whose bound does not cover it.
    pub fn read(&self, reader: &mut Reader, out: &mut [L::Word; W]) -> Read {
        let r = reader.cursor;
        debug_assert!(
            r <= Self::MASK,
            "a cursor is a count modulo the stamp space"
        );
        let head = L::load(&self.head.0, Ordering::Acquire);
        let behind = Self::distance(head, r);
        if behind == 0 {
            return Read::Empty;
        }
        if behind >= SLOTS as u64 {
            return Self::lagged(reader, behind);
        }
        // `r` was in the window when `head` was loaded, so its slot was filled
        // before that load. It holds `r` still unless the publisher has since
        // begun writing `r + SLOTS`.
        let slot = &self.slots[(r % SLOTS as u64) as usize];
        let first = L::load(&slot.stamp, Ordering::Acquire);
        if first == Self::done(r) {
            for (word, cell) in out.iter_mut().zip(slot.words.iter()) {
                *word = L::word_from_bits(L::load(cell, Ordering::Relaxed));
            }
            // The fence keeps the copy before this second load, which is what
            // makes an unchanged stamp mean an unchanged copy. The load is an
            // acquire for the other reason: if it sees the stamp of a later
            // lap, the reload of `head` below must see the publish that began
            // that lap. A fence orders a later load only against reads before
            // it, so this load cannot rely on the fence for that, and a relaxed
            // one reports a lag that skipped nothing (found by the loom model
            // `two_readers_each_hold_their_own_cursor`).
            fence(Ordering::Acquire);
            if L::load(&slot.stamp, Ordering::Acquire) == first {
                reader.cursor = Self::advance(r, 1);
                return Read::Item { seq: r };
            }
        }
        // The slot was reused while it was being read, so the publisher has
        // published through `r + RETENTION` and `head` shows it.
        let head = L::load(&self.head.0, Ordering::Acquire);
        Self::lagged(reader, Self::distance(head, r))
    }

    /// Move `reader` to the oldest entry retained and say how far that was, from
    /// `behind`, how far behind the head the reader found itself.
    fn lagged(reader: &mut Reader, behind: u64) -> Read {
        // A lag is only answered once the publisher is `SLOTS` entries or more
        // ahead, so a `behind` below that came from a head that has wrapped since
        // the reader's cursor was valid: one lap of the stamp space is added, the
        // fewest it can be. The cursor lands on the oldest retained entry's
        // residue whatever the number of laps; only the count is then short.
        let behind = if behind < SLOTS as u64 {
            behind.wrapping_add(Self::SPACE)
        } else {
            behind
        };
        let skipped = behind.saturating_sub(Self::RETENTION);
        debug_assert!(skipped >= 1, "a lag skips at least one entry");
        reader.cursor = Self::advance(reader.cursor, skipped);
        reader.lost = reader.lost.saturating_add(skipped);
        Read::Lagged { skipped }
    }
}

impl<const W: usize, const SLOTS: usize, L: Lane> Default for BroadcastLog<W, SLOTS, L> {
    fn default() -> Self {
        Self::new()
    }
}

/// The publishing side of a [`BroadcastLog`]. There is one at a time.
pub struct Publisher<'a, const W: usize, const SLOTS: usize, L: Lane> {
    log: &'a BroadcastLog<W, SLOTS, L>,
    /// The sequence number of the next entry, modulo `2^w`: this side's own
    /// count, so publishing reads no shared state.
    next: u64,
}

impl<'a, const W: usize, const SLOTS: usize, L: Lane> Publisher<'a, W, SLOTS, L> {
    /// The publisher that continues the sequence the head says was published.
    fn resume(log: &'a BroadcastLog<W, SLOTS, L>) -> Self {
        Self {
            log,
            next: log.head(),
        }
    }

    /// Publish `entry` and return the sequence number it took (modulo `2^w`).
    /// Never fails and never waits.
    ///
    /// # Panics
    ///
    /// Past [`BroadcastLog::HORIZON`] entries of the 64-bit row, which is not
    /// reached in practice.
    pub fn publish(&mut self, entry: &[L::Word; W]) -> u64 {
        let n = self.next;
        assert!(
            n < BroadcastLog::<W, SLOTS, L>::HORIZON,
            "the log has published its horizon of entries"
        );
        let slot = &self.log.slots[(n % SLOTS as u64) as usize];
        // Release, for the reader that sees this stamp and then reloads `head`:
        // it must find `head` at `n` or later, and the store of `head` to `n`
        // ended the previous publish, in this thread's order before this one. A
        // relaxed store here lets that reload return an older `head`, and the
        // reader then reports a lag that skipped nothing (found by the loom
        // model `two_readers_each_hold_their_own_cursor`, which the sequentially
        // consistent TLA+ model cannot reach). The fence below is still needed:
        // a release store orders what came before it, and this stamp must also
        // come before the words.
        L::store(
            &slot.stamp,
            BroadcastLog::<W, SLOTS, L>::busy(n),
            Ordering::Release,
        );
        fence(Ordering::Release);
        for (cell, word) in slot.words.iter().zip(entry.iter()) {
            L::store(cell, L::word_to_bits(*word), Ordering::Relaxed);
        }
        L::store(
            &slot.stamp,
            BroadcastLog::<W, SLOTS, L>::done(n),
            Ordering::Release,
        );
        // The linearization point: a reader that loads this index (acquire) sees
        // the slot's stamp and words.
        let next = BroadcastLog::<W, SLOTS, L>::advance(n, 1);
        L::store(&self.log.head.0, next, Ordering::Release);
        self.next = next;
        n
    }
}

#[cfg(target_has_atomic = "8")]
impl<const W: usize, const SLOTS: usize, L: Lane> Drop for Publisher<'_, W, SLOTS, L> {
    fn drop(&mut self) {
        self.log.publisher_taken.store(false, Ordering::Release);
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    /// A 3-bit stamp, so the stamp space is `M = 8` values and `K = 4`; 2 slots,
    /// so the retention is 1; one word an entry. An entry's word is its true
    /// count (not reduced), so a copy of the wrong lap shows.
    type Log = BroadcastLog<1, 2, Narrow<3>>;
    /// Four slots over the same stamp space: the slot count equals `K`.
    type Log4 = BroadcastLog<1, 4, Narrow<3>>;

    fn read(log: &Log, reader: &mut Reader) -> (Read, u32) {
        let mut out = [u32::MAX; 1];
        (log.read(reader, &mut out), out[0])
    }

    fn publish_all(publisher: &mut Publisher<'_, 1, 2, Narrow<3>>, counts: core::ops::Range<u32>) {
        for count in counts {
            publisher.publish(&[count]);
        }
    }

    #[test]
    fn the_stamp_space_and_its_half_follow_from_the_width() {
        assert_eq!(Log::MASK, 7, "2^3 - 1");
        assert_eq!(Log::SPACE, 8, "2^3");
        assert_eq!(Log::HALF, 4, "2^(3 - 1)");
        assert_eq!(Log::RETENTION, 1, "2 slots keep 1");
        assert_eq!(Log4::RETENTION, 3, "4 slots keep 3");
    }

    #[test]
    fn a_stamp_tells_apart_exactly_half_the_stamp_space() {
        // done(n) = 2n + 2 mod 8, so n and n + 4 give the same stamp (2n + 10 =
        // 2n + 2 mod 8) and n and n + j for j = 1, 2, 3 differ by 2j = 2, 4, 6.
        for n in 0..8u64 {
            assert_eq!(
                Log::done(n),
                Log::done(n + 4),
                "done({n}) one half-space on"
            );
            assert_eq!(
                Log::busy(n),
                Log::busy(n + 4),
                "busy({n}) one half-space on"
            );
            for j in 1..4u64 {
                assert_ne!(Log::done(n), Log::done(n + j), "done({n}) against +{j}");
                assert_ne!(Log::busy(n), Log::busy(n + j), "busy({n}) against +{j}");
            }
            assert_ne!(Log::busy(n), Log::done(n), "busy is odd and done is even");
        }
        // The 32-bit lane of a row: entries 2^31 - 1 and 2^32 - 1 (half a stamp
        // space apart) have the same stamps. done: 2 * (2^31 - 1) + 2 = 2^32 and
        // 2 * (2^32 - 1) + 2 = 2^33, both 0 modulo 2^32. busy: 2 * (2^31 - 1) + 1 =
        // 2^32 - 1 and 2 * (2^32 - 1) + 1 = 2^33 - 1, both 2^32 - 1 modulo 2^32.
        type Row = BroadcastLog<1, 4, Narrow<32>>;
        assert_eq!(Row::done(0x7FFF_FFFF), 0);
        assert_eq!(Row::done(0xFFFF_FFFF), 0);
        assert_eq!(Row::done(0), 2);
        assert_eq!(Row::busy(0x7FFF_FFFF), 0xFFFF_FFFF);
        assert_eq!(Row::busy(0xFFFF_FFFF), 0xFFFF_FFFF);
    }

    /// Every entry the last slot ever holds, over four laps of the stamp space (the
    /// count is reduced as a publisher reduces it), is stamped with neither value
    /// that stands for an unwritten slot.
    fn the_last_slot_is_never_stamped_unwritten<const SLOTS: usize, const B: u32>() {
        let space = 1u64 << B;
        let unwritten = <Narrow<B> as Lane>::UNWRITTEN;
        for count in 0..4 * space {
            let n = count & (space - 1);
            if n % SLOTS as u64 == SLOTS as u64 - 1 {
                let busy = BroadcastLog::<1, SLOTS, Narrow<B>>::busy(n);
                let done = BroadcastLog::<1, SLOTS, Narrow<B>>::done(n);
                assert_ne!(busy, unwritten, "busy({n}), B {B}, S {SLOTS}");
                assert_ne!(done, unwritten, "done({n}), B {B}, S {SLOTS}");
            }
        }
    }

    #[test]
    fn the_unwritten_stamp_is_never_one_the_protocol_writes_into_the_last_slot() {
        the_last_slot_is_never_stamped_unwritten::<2, 3>();
        the_last_slot_is_never_stamped_unwritten::<4, 3>();
        the_last_slot_is_never_stamped_unwritten::<2, 4>();
        the_last_slot_is_never_stamped_unwritten::<4, 4>();
        the_last_slot_is_never_stamped_unwritten::<8, 4>();
        the_last_slot_is_never_stamped_unwritten::<2, 2>();
    }

    #[test]
    fn a_stamp_at_the_top_of_the_space_is_what_the_arithmetic_says() {
        assert_eq!(Log::done(7), 0, "2 * 7 + 2 = 16, 0 modulo 8");
        assert_eq!(Log::busy(7), 7, "2 * 7 + 1 = 15, 7 modulo 8");
    }

    #[test]
    fn a_narrow_head_wraps_and_the_reader_still_reads_every_entry_in_order() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        let mut reader = log.reader_head();
        assert_eq!(reader.cursor(), 0);
        // Twelve entries cross the head's wrap at 8 once. The reader keeps up, so
        // after publish `n` it finds entry `n` and no other: its sequence number is
        // `n mod 8`, and the stored head is `(n + 1) mod 8`.
        for n in 0..12u32 {
            assert_eq!(publisher.publish(&[n]), u64::from(n % 8), "publish {n}");
            assert_eq!(log.head(), u64::from((n + 1) % 8), "head after {n}");
            assert_eq!(
                read(log, &mut reader),
                (
                    Read::Item {
                        seq: u64::from(n % 8)
                    },
                    n
                ),
                "entry {n}"
            );
            assert_eq!(read(log, &mut reader).0, Read::Empty);
        }
        assert_eq!(reader.lost(), 0);
    }

    #[test]
    fn a_reader_lapped_across_the_wrap_is_told_the_exact_count() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        // Six entries (0 to 5): the reader opened now is at 6.
        publish_all(&mut publisher, 0..6);
        let mut reader = log.reader_head();
        assert_eq!(reader.cursor(), 6);
        // Three more (6, 7, 8): the head is 9, stored as 9 mod 8 = 1. The reader is
        // (1 - 6) mod 8 = 3 behind, which is the window (1 entry) and 2 more.
        publish_all(&mut publisher, 6..9);
        assert_eq!(log.head(), 1);
        assert_eq!(read(log, &mut reader).0, Read::Lagged { skipped: 2 });
        // 6 + 2 = 8, which is 0 modulo 8: the oldest retained entry is 8.
        assert_eq!(reader.cursor(), 0);
        assert_eq!(reader.lost(), 2);
        assert_eq!(
            read(log, &mut reader),
            (Read::Item { seq: 0 }, 8),
            "the entry of count 8, whose stamp is the one of count 0 a lap before"
        );
        assert_eq!(read(log, &mut reader).0, Read::Empty);
    }

    #[test]
    fn a_reader_one_stamp_space_behind_is_told_empty_and_keeps_its_place() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        let mut reader = log.reader_head();
        // Eight entries: the head is 8 mod 8 = 0, where the reader is. It cannot
        // tell that from nothing published, which is outside the bound, and it is
        // told what the arithmetic says.
        publish_all(&mut publisher, 0..8);
        assert_eq!(read(log, &mut reader).0, Read::Empty);
        assert_eq!(reader.cursor(), 0);
        assert_eq!(reader.lost(), 0, "the 8 entries it missed are not counted");
        // Its place is right: the next entry, count 8, is the one it reads.
        publish_all(&mut publisher, 8..9);
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 0 }, 8));
    }

    #[test]
    fn a_reader_more_than_a_stamp_space_behind_is_lagged_by_the_count_modulo_the_space() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        let mut reader = log.reader_head();
        // Ten entries (0 to 9): the head is 10 mod 8 = 2, so the reader is 2 behind,
        // which is the window (1) and 1 more: skipped is 1, where the truth is
        // 10 - 1 = 9, short by 8.
        publish_all(&mut publisher, 0..10);
        assert_eq!(read(log, &mut reader).0, Read::Lagged { skipped: 1 });
        assert_eq!(reader.lost(), 1);
        // The place is right all the same: 0 + 1 = 1 is the residue of count 9, the
        // oldest retained, and it is read whole.
        assert_eq!(reader.cursor(), 1);
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 1 }, 9));
    }

    #[test]
    fn a_reader_a_stamp_space_and_one_behind_is_handed_a_later_lap_as_its_own() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        let mut reader = log.reader_head();
        // Nine entries (0 to 8): the head is 9 mod 8 = 1, one ahead of the reader,
        // inside the window; the slot of entry 0 holds entry 8, whose stamp
        // done(8 mod 8) = 2 is done(0): the copy validates and is handed as the
        // entry the reader wanted. This is what the bound exists to keep from
        // happening, and why the deploy states it.
        publish_all(&mut publisher, 0..9);
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 0 }, 8));
    }

    #[test]
    fn a_head_that_wrapped_during_a_mismatch_still_gives_a_lag_of_at_least_one() {
        // A mismatch proves the publisher is `SLOTS` or more ahead. A head that
        // says the reader is only 1 behind is a head one lap round: 1 + 8 = 9 behind,
        // less the retention 1, is 8 skipped, and the cursor (5 + 8) mod 8 = 5 is
        // where it was.
        let mut reader = Reader { cursor: 5, lost: 0 };
        assert_eq!(Log::lagged(&mut reader, 1), Read::Lagged { skipped: 8 });
        assert_eq!(reader.cursor(), 5);
        assert_eq!(reader.lost(), 8);
        // A head that already says `SLOTS` or more is taken as it is: 3 - 1 = 2.
        let mut reader = Reader { cursor: 5, lost: 0 };
        assert_eq!(Log::lagged(&mut reader, 3), Read::Lagged { skipped: 2 });
        assert_eq!(reader.cursor(), 7, "(5 + 2) mod 8");
    }

    #[test]
    fn the_oldest_reader_knows_a_young_log_from_one_that_wrapped() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        // Nothing published: the last slot (slot 1) is unwritten, the oldest
        // retained entry is 0 and the reader is at 0.
        let mut reader = log.reader_oldest();
        assert_eq!(
            (reader.cursor(), read(log, &mut reader).0),
            (0, Read::Empty)
        );
        // One entry: still `h <= RETENTION = 1`, so the oldest is 0.
        publisher.publish(&[0]);
        let mut reader = log.reader_oldest();
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 0 }, 0));
        // Two entries: slot 1 is written, so the oldest retained is 2 - 1 = 1.
        publisher.publish(&[1]);
        let mut reader = log.reader_oldest();
        assert_eq!(reader.cursor(), 1);
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 1 }, 1));
        // After the wrap, with the head at a small residue: 8 entries leave the
        // head at 0, and the oldest retained is count 7, residue (0 - 1) mod 8 = 7.
        publish_all(&mut publisher, 2..8);
        let mut reader = log.reader_oldest();
        assert_eq!(reader.cursor(), 7);
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 7 }, 7));
        // Nine: the head is 1, the oldest is count 8, residue 0.
        publish_all(&mut publisher, 8..9);
        let mut reader = log.reader_oldest();
        assert_eq!(reader.cursor(), 0);
        assert_eq!(read(log, &mut reader), (Read::Item { seq: 0 }, 8));
    }

    #[test]
    fn a_position_is_read_modulo_the_stamp_space_and_one_ahead_is_the_head() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        publish_all(&mut publisher, 0..6);
        // The head is 6; K = 4. A position fewer than 4 behind is taken as it is.
        assert_eq!(log.reader_at(3).cursor(), 3, "(6 - 3) mod 8 = 3 < 4");
        assert_eq!(log.reader_at(6).cursor(), 6, "at the head");
        // 4 or more behind is what one ahead of the head looks like, and is the head:
        // (6 - 2) mod 8 = 4, and (6 - 7) mod 8 = 7.
        assert_eq!(log.reader_at(2).cursor(), 6);
        assert_eq!(log.reader_at(7).cursor(), 6);
        // A count past the stamp space is reduced: 14 mod 8 = 6.
        assert_eq!(log.reader_at(14).cursor(), 6);
    }

    #[test]
    fn a_log_with_as_many_slots_as_half_the_stamp_space_keeps_its_window_across_the_wrap() {
        let mut log = Log4::new();
        let (mut publisher, log) = log.split();
        let mut reader = log.reader_head();
        // 3 retained of 4 slots over 8 stamp values: ten entries, read one at a
        // time, then a reader that was held back for 3.
        let mut out = [0u32; 1];
        for n in 0..10u32 {
            publisher.publish(&[n]);
            assert_eq!(
                log.read(&mut reader, &mut out),
                Read::Item {
                    seq: u64::from(n % 8)
                }
            );
            assert_eq!(out[0], n);
        }
        publisher.publish(&[10]);
        publisher.publish(&[11]);
        publisher.publish(&[12]);
        // A reader that waited through 3 publishes is 3 behind the head, inside the
        // window of 3, and reads the oldest retained entry, count 10, first.
        let mut held = log.reader_head();
        assert_eq!(held.cursor(), u64::from(13u32 % 8));
        let mut behind = Reader {
            cursor: 10 % 8,
            lost: 0,
        };
        assert_eq!(
            log.read(&mut behind, &mut out),
            Read::Item { seq: 2 },
            "(5 - 2) mod 8 = 3 behind: in the window"
        );
        assert_eq!(out[0], 10);
        assert_eq!(log.read(&mut held, &mut out), Read::Empty);
    }

    #[test]
    fn a_split_publisher_holds_the_place_and_its_successor_continues_the_sequence() {
        let mut log = Log::new();
        {
            let (mut publisher, shared) = log.split();
            assert!(
                shared.publisher().is_none(),
                "the shared view cannot raise a second publisher"
            );
            assert_eq!(publisher.publish(&[0]), 0);
            assert_eq!(publisher.publish(&[1]), 1);
        }
        // Both dropped: the borrow is over and the place is free.
        {
            let (mut publisher, _) = log.split();
            assert_eq!(publisher.publish(&[2]), 2, "the sequence continues");
        }
        let first = log.publisher().expect("the place was freed");
        assert!(log.publisher().is_none(), "a second publisher is refused");
        drop(first);
        assert!(log.publisher().is_some());
    }

    #[test]
    fn a_log_in_a_static_takes_its_publisher_on_the_callers_word() {
        static LOG: Log = Log::new();
        // SAFETY: this is the only publisher of `LOG` and the test makes no other.
        let mut publisher = unsafe { LOG.publisher_unchecked() };
        assert!(LOG.publisher().is_none(), "the flag is raised for it");
        assert_eq!(publisher.publish(&[0]), 0);
        let mut reader = LOG.reader_oldest();
        assert_eq!(read(&LOG, &mut reader), (Read::Item { seq: 0 }, 0));
    }

    #[test]
    fn publishing_forever_does_not_reach_a_horizon_in_a_narrow_log() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        let mut reader = log.reader_head();
        // 100 entries are 12 laps of the stamp space and 4 entries; the reader that
        // keeps up reads each by its residue.
        for n in 0..100u32 {
            publisher.publish(&[n]);
            assert_eq!(
                read(log, &mut reader),
                (
                    Read::Item {
                        seq: u64::from(n % 8)
                    },
                    n
                )
            );
        }
    }
}
