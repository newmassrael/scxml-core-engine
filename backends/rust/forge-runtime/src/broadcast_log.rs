// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounded broadcast log, 64-bit row: one publisher, any number of readers,
//! the oldest entry forgotten when the log is full (SCE Protocol-Synthesis RFC
//! §synth-5-S, ADR 0062) — the runtime half of `sce:kind="broadcast-log"` for a
//! target with 64-bit atomics.
//!
//! The contract is the sequential log of ADR 0062 section 2. `h` entries have
//! been published, only the last [`BroadcastLog::RETENTION`] are retained, and a
//! reader owns a cursor: [`BroadcastLog::read`] returns the entry at the cursor,
//! or says the log has nothing new, or says how many entries the reader missed
//! and moves it to the oldest retained. Publishing never fails and never waits,
//! and neither does a read: both are wait-free, in a number of steps that
//! depends on the entry's width and on nothing the other side does.
//!
//! **The protocol, its orders and the argument for them are in
//! [`crate::broadcast_log_core`]**, which this row instantiates over 64-bit
//! atomics ([`crate::broadcast_log_core::Wide`]); the row for a target without
//! them, over a stamp of 32 bits in pointer-width atomics, is
//! [`crate::broadcast_log_narrow`]. Which target gets which row:
//!
//! | Target | Atomics | Row | Publisher |
//! |---|---|---|---|
//! | 64-bit atomics and an 8-bit swap (x86-64, AArch64, RISC-V 64 with `A`) | `target_has_atomic = "64"` | this module, a 64-bit stamp | `publisher()`, or `split` |
//! | 32-bit with compare-and-swap (Cortex-M3 and up, RISC-V 32 with `A`, ESP32) | `target_has_atomic = "32"`, not `"64"` | [`crate::broadcast_log_narrow`], a 32-bit stamp | `publisher()`, or `split` |
//! | loads and stores only (Cortex-M0, RISC-V 32 without `A`) | neither | [`crate::broadcast_log_narrow`], a 32-bit stamp | `split`, or `publisher_unchecked` |
//! | a pointer of 16 bits (AVR, MSP430) | | none | |
//!
//! **The stamp is a sequence number, not a flag.** It carries `n`, so a reader
//! comparing against `done(r)` cannot mistake a later lap for the entry it wants.
//! It is 64 bits here, which is why this module is absent on a target without
//! 64-bit atomics: the sequence stays below [`HORIZON`], 292 years at a billion
//! publishes per second, and no wrap bound is asked of a deploy.
//!
//! No global state, no threads, no allocation (SCE_FORGE.md section 2.1, C1/C2):
//! the log owns its storage and where it lives is the caller's choice.

use crate::broadcast_log_core as generic;

pub use crate::broadcast_log_core::{Read, Reader};

/// The most entries a log can publish: the stamps are `2n + 2`, which must fit
/// a `u64`. Past it a publish would corrupt the stamps, so
/// [`Publisher::publish`] stops there with a panic instead of publishing.
pub const HORIZON: u64 = (u64::MAX - 2) / 2;

/// A bounded log whose entries are `W` 64-bit words, kept in `SLOTS` slots and
/// retaining the last `SLOTS - 1` entries.
pub type BroadcastLog<const W: usize, const SLOTS: usize> =
    generic::BroadcastLog<W, SLOTS, generic::Wide>;

/// The publishing side of a [`BroadcastLog`]. There is one at a time.
pub type Publisher<'a, const W: usize, const SLOTS: usize> =
    generic::Publisher<'a, W, SLOTS, generic::Wide>;

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

    #[test]
    fn the_64_bit_row_never_reduces_a_count_and_clamps_a_position_past_the_head() {
        let log = Log::new();
        let mut publisher = log.publisher().expect("the publisher");
        publisher.publish(&entry(0));
        // The largest `u64` is clamped to the head and not read modulo a stamp
        // space: this row has none.
        assert_eq!(log.reader_at(u64::MAX).cursor(), 1);
        // A log's `HORIZON` is the row's.
        assert_eq!(Log::HORIZON, HORIZON);
        // The head is the count itself, not a residue.
        assert_eq!(log.head(), 1);
    }
}
