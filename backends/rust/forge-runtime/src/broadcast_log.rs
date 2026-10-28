// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounded broadcast log: one publisher, any number of readers, the oldest
//! entry forgotten when the log is full (ADR 0062) — the runtime half of
//! `sce:kind="broadcast-log"`.
//!
//! The contract is the sequential log of ADR 0062 section 2. `h` entries have
//! been published, only the last [`BroadcastLog::RETENTION`] are retained, and a
//! reader owns a cursor: [`BroadcastLog::read`] returns the entry at the cursor,
//! or says the log has nothing new, or says how many entries the reader missed
//! and moves it to the oldest retained. Publishing never fails and never waits,
//! and neither does a read: both are wait-free, in a number of steps that
//! depends on the entry's width and on nothing the other side does.
//!
//! **The protocol** (section 4). A slot holds a stamp and the entry's words.
//! The publisher writes a slot as `stamp := busy(n)` (release), a release fence, the words,
//! `stamp := done(n)` (release), then `head := n + 1` (release). A reader holding
//! cursor `r` loads `head` (acquire) and answers `Empty` at the head and `Lagged`
//! outside the window; otherwise it loads the stamp of slot `r mod S`, copies the
//! words, an acquire fence, and loads the stamp again: if it is still `done(r)`
//! the copy is entry `r`; if not, the publisher has reused the slot and the
//! reader reloads `head` and answers `Lagged`. This is a sequence lock in the
//! form Boehm showed correct under the C++11 model ("Can seqlocks get along with
//! programming language memory models?", MSPC 2012): the words are atomics
//! accessed relaxed, so there is no data race, and the two fences carry the
//! order.
//!
//! **The slot count is `RETENTION + 1`.** The slot being overwritten is in flux
//! and cannot be promised to a reader. With `RETENTION` slots a reader could find
//! the entry the specification retains already being overwritten and be told it
//! lagged by nothing; `docs/formal/BroadcastLog.tla` finds that state, and so
//! does its rejected configuration.
//!
//! **The stamp is a sequence number, not a flag.** It carries `n`, so a reader
//! comparing against `done(r)` cannot mistake a later lap for the entry it wants.
//! It is 64 bits, which is why this module is absent on a target without 64-bit
//! atomics: the sequence stays below [`HORIZON`], 292 years at a billion
//! publishes per second.
//!
//! No global state, no threads, no allocation (SCE_FORGE.md section 2.1, C1/C2):
//! the log owns its storage and where it lives is the caller's choice.

use crate::queue::sync::{fence, AtomicBool, AtomicU64, Ordering};
use crate::queue::Padded;

/// The most entries a log can publish: the stamps are `2n + 2`, which must fit
/// a `u64`. Past it a publish would corrupt the stamps, so [`Publisher::publish`]
/// stops there with a panic instead of publishing.
pub const HORIZON: u64 = (u64::MAX - 2) / 2;

/// The stamp a slot holds while the entry `n` is being written into it.
const fn busy(n: u64) -> u64 {
    2 * n + 1
}

/// The stamp a slot holds once the entry `n` is in it.
const fn done(n: u64) -> u64 {
    2 * n + 2
}

/// What a read found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// The entry `seq` was copied into the caller's buffer, and the reader now
    /// holds `seq + 1`.
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
    /// The sequence number this reader reads next.
    pub const fn cursor(&self) -> u64 {
        self.cursor
    }

    /// The entries this reader has been told it missed, summed over its reads.
    pub const fn lost(&self) -> u64 {
        self.lost
    }
}

struct Slot<const W: usize> {
    stamp: AtomicU64,
    words: [AtomicU64; W],
}

/// A bounded log whose entries are `W` 64-bit words, kept in `SLOTS` slots and
/// retaining the last `SLOTS - 1` entries.
pub struct BroadcastLog<const W: usize, const SLOTS: usize> {
    /// The number of entries published. Written only by the publisher, loaded by
    /// every reader, so it has a cache line of its own.
    head: Padded<AtomicU64>,
    /// Whether a [`Publisher`] exists.
    publisher_taken: AtomicBool,
    slots: [Slot<W>; SLOTS],
}

impl<const W: usize, const SLOTS: usize> BroadcastLog<W, SLOTS> {
    /// The entries a reader may rely on.
    pub const RETENTION: u64 = (SLOTS - 1) as u64;

    /// Evaluated wherever a log of this size is constructed, so a size the
    /// protocol cannot represent is a compile error and not a runtime one.
    const CHECKED: () = assert!(
        SLOTS >= 2,
        "a log retains at least one entry, so it has at least two slots"
    );

    /// An empty log.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        let () = Self::CHECKED;
        Self {
            head: Padded(AtomicU64::new(0)),
            publisher_taken: AtomicBool::new(false),
            slots: [const {
                Slot {
                    stamp: AtomicU64::new(0),
                    words: [const { AtomicU64::new(0) }; W],
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
            head: Padded(AtomicU64::new(0)),
            publisher_taken: AtomicBool::new(false),
            slots: core::array::from_fn(|_| Slot {
                stamp: AtomicU64::new(0),
                words: core::array::from_fn(|_| AtomicU64::new(0)),
            }),
        }
    }

    /// The number of entries published so far.
    pub fn head(&self) -> u64 {
        self.head.0.load(Ordering::Acquire)
    }

    /// The publisher, or `None` while another exists. Dropping it frees the
    /// place, and its successor continues the sequence.
    pub fn publisher(&self) -> Option<Publisher<'_, W, SLOTS>> {
        if self.publisher_taken.swap(true, Ordering::Acquire) {
            return None;
        }
        Some(Publisher {
            log: self,
            next: self.head.0.load(Ordering::Acquire),
        })
    }

    /// A reader at the oldest retained entry.
    pub fn reader_oldest(&self) -> Reader {
        Reader {
            cursor: self.head().saturating_sub(Self::RETENTION),
            lost: 0,
        }
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
    pub fn reader_at(&self, seq: u64) -> Reader {
        Reader {
            cursor: seq.min(self.head()),
            lost: 0,
        }
    }

    /// Read the entry at `reader`'s cursor into `out`. A bounded number of
    /// steps whatever the publisher is doing: a copy that the publisher
    /// overwrote is not retried, it is the answer ([`Read::Lagged`]).
    pub fn read(&self, reader: &mut Reader, out: &mut [u64; W]) -> Read {
        let r = reader.cursor;
        let h = self.head.0.load(Ordering::Acquire);
        debug_assert!(r <= h, "a reader's cursor never passes the head");
        if r >= h {
            return Read::Empty;
        }
        if h - r >= SLOTS as u64 {
            return Self::lagged(reader, h);
        }
        // `r` was in the window when `head` was loaded, so its slot was filled
        // before that load. It holds `r` still unless the publisher has since
        // begun writing `r + SLOTS`.
        let slot = &self.slots[(r % SLOTS as u64) as usize];
        let first = slot.stamp.load(Ordering::Acquire);
        if first == done(r) {
            for (word, cell) in out.iter_mut().zip(slot.words.iter()) {
                *word = cell.load(Ordering::Relaxed);
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
            if slot.stamp.load(Ordering::Acquire) == first {
                reader.cursor = r + 1;
                return Read::Item { seq: r };
            }
        }
        // The slot was reused while it was being read, so the publisher has
        // published through `r + RETENTION` and `head` shows it.
        let h = self.head.0.load(Ordering::Acquire);
        Self::lagged(reader, h)
    }

    /// Move `reader` to the oldest entry retained when the head is `h`, and say
    /// how far that was.
    fn lagged(reader: &mut Reader, h: u64) -> Read {
        let oldest = h.saturating_sub(Self::RETENTION);
        let skipped = oldest.saturating_sub(reader.cursor);
        debug_assert!(skipped >= 1, "a lag skips at least one entry");
        reader.cursor = oldest;
        reader.lost = reader.lost.saturating_add(skipped);
        Read::Lagged { skipped }
    }
}

impl<const W: usize, const SLOTS: usize> Default for BroadcastLog<W, SLOTS> {
    fn default() -> Self {
        Self::new()
    }
}

/// The publishing side of a [`BroadcastLog`]. There is one at a time.
pub struct Publisher<'a, const W: usize, const SLOTS: usize> {
    log: &'a BroadcastLog<W, SLOTS>,
    /// The sequence number of the next entry: this side's own count, so
    /// publishing reads no shared state.
    next: u64,
}

impl<const W: usize, const SLOTS: usize> Publisher<'_, W, SLOTS> {
    /// Publish `entry` and return the sequence number it took. Never fails and
    /// never waits.
    ///
    /// # Panics
    ///
    /// Past [`HORIZON`] entries, which is not reached in practice.
    pub fn publish(&mut self, entry: &[u64; W]) -> u64 {
        let n = self.next;
        assert!(n < HORIZON, "the log has published its horizon of entries");
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
        slot.stamp.store(busy(n), Ordering::Release);
        fence(Ordering::Release);
        for (cell, word) in slot.words.iter().zip(entry.iter()) {
            cell.store(*word, Ordering::Relaxed);
        }
        slot.stamp.store(done(n), Ordering::Release);
        // The linearization point: a reader that loads this index (acquire) sees
        // the slot's stamp and words.
        self.log.head.0.store(n + 1, Ordering::Release);
        self.next = n + 1;
        n
    }
}

impl<const W: usize, const SLOTS: usize> Drop for Publisher<'_, W, SLOTS> {
    fn drop(&mut self) {
        self.log.publisher_taken.store(false, Ordering::Release);
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    type Log = BroadcastLog<2, 3>;

    fn entry(n: u64) -> [u64; 2] {
        [n, n]
    }

    fn read(log: &Log, reader: &mut Reader) -> (Read, [u64; 2]) {
        let mut out = [u64::MAX; 2];
        (log.read(reader, &mut out), out)
    }

    #[test]
    fn an_empty_log_is_empty_to_every_reader() {
        let log = Log::new();
        let mut reader = log.reader_oldest();
        assert_eq!(read(&log, &mut reader).0, Read::Empty);
        assert_eq!(reader.cursor(), 0);
        assert_eq!(log.head(), 0);
    }

    #[test]
    fn entries_come_back_in_order_with_their_words() {
        let log = Log::new();
        let mut publisher = log.publisher().expect("the first publisher");
        let mut reader = log.reader_oldest();
        assert_eq!(publisher.publish(&entry(10)), 0);
        assert_eq!(publisher.publish(&entry(11)), 1);
        assert_eq!(read(&log, &mut reader), (Read::Item { seq: 0 }, entry(10)));
        assert_eq!(read(&log, &mut reader), (Read::Item { seq: 1 }, entry(11)));
        assert_eq!(read(&log, &mut reader).0, Read::Empty);
        assert_eq!(reader.lost(), 0);
    }

    #[test]
    fn the_window_is_exactly_the_retention() {
        // 3 slots retain 2 entries.
        assert_eq!(Log::RETENTION, 2);
        let log = Log::new();
        let mut publisher = log.publisher().expect("the publisher");
        for n in 0..3 {
            publisher.publish(&entry(n));
        }
        // h = 3: the window is [1, 3), so a reader at 0 is one behind it.
        let mut behind = log.reader_at(0);
        assert_eq!(read(&log, &mut behind).0, Read::Lagged { skipped: 1 });
        assert_eq!(behind.cursor(), 1);
        assert_eq!(behind.lost(), 1);
        assert_eq!(read(&log, &mut behind), (Read::Item { seq: 1 }, entry(1)));
        // A reader at the oldest retained entry lost nothing.
        let mut oldest = log.reader_oldest();
        assert_eq!(oldest.cursor(), 1);
        assert_eq!(read(&log, &mut oldest), (Read::Item { seq: 1 }, entry(1)));
    }

    #[test]
    fn readers_do_not_share_a_cursor_or_a_loss() {
        let log = Log::new();
        let mut publisher = log.publisher().expect("the publisher");
        for n in 0..5 {
            publisher.publish(&entry(n));
        }
        let mut a = log.reader_at(0);
        let mut b = log.reader_at(3);
        assert_eq!(read(&log, &mut a).0, Read::Lagged { skipped: 3 });
        assert_eq!(read(&log, &mut b), (Read::Item { seq: 3 }, entry(3)));
        assert_eq!((a.lost(), b.lost()), (3, 0));
        assert_eq!(b.cursor(), 4);
    }

    #[test]
    fn a_reader_at_the_head_sees_only_what_comes_after() {
        let log = Log::new();
        let mut publisher = log.publisher().expect("the publisher");
        publisher.publish(&entry(0));
        let mut reader = log.reader_head();
        assert_eq!(read(&log, &mut reader).0, Read::Empty);
        publisher.publish(&entry(1));
        assert_eq!(read(&log, &mut reader), (Read::Item { seq: 1 }, entry(1)));
        // A position past the head is clamped to it.
        assert_eq!(log.reader_at(99).cursor(), 2);
    }

    #[test]
    fn there_is_one_publisher_and_its_successor_continues_the_sequence() {
        let log = Log::new();
        let mut first = log.publisher().expect("the first publisher");
        assert!(log.publisher().is_none(), "a second publisher is refused");
        first.publish(&entry(0));
        drop(first);
        let mut second = log.publisher().expect("the place was freed");
        assert_eq!(second.publish(&entry(1)), 1);
    }

    #[test]
    fn publishing_past_the_window_keeps_the_newest_entries() {
        let log = Log::new();
        let mut publisher = log.publisher().expect("the publisher");
        for n in 0..100 {
            publisher.publish(&entry(n));
        }
        let mut reader = log.reader_oldest();
        assert_eq!(read(&log, &mut reader), (Read::Item { seq: 98 }, entry(98)));
        assert_eq!(read(&log, &mut reader), (Read::Item { seq: 99 }, entry(99)));
        assert_eq!(read(&log, &mut reader).0, Read::Empty);
    }
}
