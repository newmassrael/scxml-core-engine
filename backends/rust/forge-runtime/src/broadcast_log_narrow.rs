// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounded broadcast log, narrow row: the log of [`crate::broadcast_log`] for a
//! target without 64-bit atomics (SCE Protocol-Synthesis RFC §synth-5-S; ADR 0062
//! section 4, "Stamp width"). The stamp,
//! the head and every word are pointer-width atomics holding 32 bits, so the row
//! needs no 64-bit atomic and no read-modify-write instruction: loads and
//! stores alone, as the Lamport ring of [`crate::queue::spsc`] needs. It is the
//! same implementation as the 64-bit row ([`crate::broadcast_log_core`]) over the
//! lane [`crate::broadcast_log_core::Narrow`]; what differs is arithmetic.
//!
//! **Sequence numbers count modulo `2^32`.** The publisher keeps its own count and
//! stores the low 32 bits of `2n + 1` (busy) and `2n + 2` (done) as the stamp and
//! the low 32 bits of `n + 1` as the head. A reader extends the stored head to its
//! own frame by taking its distance from its cursor modulo `2^32`, which is the
//! true distance while it is below `2^32`, and a stamp tells `2^31` consecutive
//! entries of one slot apart. So:
//!
//! - `head()`, `Reader::cursor()`, `Read::Item { seq }` and the number `publish`
//!   returns are counts modulo `2^32`. They agree across handles and are not
//!   ordered across a wrap.
//! - **The wrap bound.** A reader gets the answers of the specification while it is
//!   fewer than `2^31` publishes behind the newest begun, during each read
//!   ([`crate::broadcast_log_core`] states and proves it). That is a promise of the
//!   deploy: it states `machines.<m>.queues.<log>.min_wrap_ops`, the longest pause
//!   in publishes between two reads, and the generator refuses a log whose
//!   bound, `2^31 - slots`, does not cover it. A reader that breaks it is not
//!   told; what it is answered is defined (see [`BroadcastLog::read`]).
//! - **The slot count is a power of two** (`RETENTION + 1 = 2^s`, at most `2^30`), so
//!   that the slot index `n mod S` of a reader, who knows `n` only modulo `2^32`,
//!   is the publisher's. A log of any other size is a compile error at its
//!   `new`, and the generator refuses the document that asks for one.
//!
//! **The publisher is one by ownership where the target has no swap.** A target
//! with loads and stores only (Cortex-M0, RISC-V 32 without `A`) has no
//! read-modify-write instruction, so the 8-bit swap that `publisher()` takes its
//! place with cannot exist; the algorithm itself needs none. [`BroadcastLog::split`]
//! hands out the one publisher against an exclusive borrow, and a log in a
//! `static` takes it on the caller's word with the `unsafe`
//! [`BroadcastLog::publisher_unchecked`]. Where the target has the swap
//! (`target_has_atomic = "8"`), `publisher()` is there too and all three are one
//! place: a publisher of any of them refuses another of the others.
//!
//! A pointer of 16 bits has neither this row nor the 64-bit one: its words would not
//! fit a cell, and the module is absent there.
//!
//! | Target | Row | Publisher |
//! |---|---|---|
//! | 64-bit atomics (`target_has_atomic = "64"`) | [`crate::broadcast_log`] (this row is there too, for a model or a log that wants it) | `publisher()`, `split` |
//! | 32-bit with compare-and-swap | this module | `publisher()`, `split` |
//! | loads and stores only | this module | `split`, `publisher_unchecked` |
//! | a 16-bit pointer | none | |

use crate::broadcast_log_core as generic;

pub use crate::broadcast_log_core::{Read, Reader};

/// The stamp's width in bits. The generator judges a log against this number
/// (`judge_broadcast_log_stamp`), and a test holds the two equal.
pub const STAMP_BITS: u32 = 32;

/// A bounded log whose entries are `W` 32-bit words, kept in `SLOTS` slots (a power
/// of two) and retaining the last `SLOTS - 1` entries.
pub type BroadcastLog<const W: usize, const SLOTS: usize> =
    generic::BroadcastLog<W, SLOTS, generic::Narrow<STAMP_BITS>>;

/// The publishing side of a [`BroadcastLog`]. There is one at a time.
pub type Publisher<'a, const W: usize, const SLOTS: usize> =
    generic::Publisher<'a, W, SLOTS, generic::Narrow<STAMP_BITS>>;

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    /// Two words an entry and four slots, so the retention is 3.
    type Log = BroadcastLog<2, 4>;

    fn entry(n: u32) -> [u32; 2] {
        [n, n]
    }

    fn read(log: &Log, reader: &mut Reader) -> (Read, [u32; 2]) {
        let mut out = [u32::MAX; 2];
        (log.read(reader, &mut out), out)
    }

    #[test]
    fn the_row_keeps_a_stamp_of_32_bits_and_four_byte_words() {
        assert_eq!(STAMP_BITS, 32);
        assert_eq!(Log::RETENTION, 3, "4 slots keep 3");
        let log = Log::new();
        let mut publisher = log.publisher().expect("the publisher");
        publisher.publish(&[0xDEAD_BEEF, 0x0102_0304]);
        let mut reader = log.reader_oldest();
        assert_eq!(
            read(&log, &mut reader),
            (Read::Item { seq: 0 }, [0xDEAD_BEEF, 0x0102_0304]),
            "a word keeps all of its 32 bits"
        );
    }

    #[test]
    fn entries_come_back_in_order_and_a_lapped_reader_is_told_how_many_it_missed() {
        let mut log = Log::new();
        let (mut publisher, log) = log.split();
        let mut slow = log.reader_oldest();
        // Six entries (0 to 5) into a log that retains three: the window is
        // [6 - 3, 6) = [3, 6), so the reader at 0 missed 3 - 0 = 3.
        for n in 0..6 {
            assert_eq!(publisher.publish(&entry(n)), u64::from(n));
        }
        assert_eq!(read(log, &mut slow).0, Read::Lagged { skipped: 3 });
        assert_eq!((slow.cursor(), slow.lost()), (3, 3));
        for n in 3..6 {
            assert_eq!(
                read(log, &mut slow),
                (Read::Item { seq: u64::from(n) }, entry(n))
            );
        }
        assert_eq!(read(log, &mut slow).0, Read::Empty);
    }

    #[test]
    fn the_row_is_the_narrow_lane_at_its_stamp_width() {
        // `new` refuses, at compile time, a slot count that is not a power of two
        // or is above half the stamp space, so the sizes that are accepted are the
        // ones the generator judges; the width it judges against is this one.
        assert_eq!(
            <generic::Narrow<STAMP_BITS> as generic::Lane>::BITS,
            STAMP_BITS
        );
        assert_eq!(
            Log::HORIZON,
            1u64 << 32,
            "a narrow log has no horizon but its stamp space"
        );
    }

    #[test]
    fn there_is_one_publisher_whether_it_is_taken_by_swap_or_by_ownership() {
        let mut log = Log::new();
        {
            let first = log.publisher().expect("the place is free");
            assert!(log.publisher().is_none(), "a second publisher is refused");
            drop(first);
        }
        let (mut publisher, shared) = log.split();
        assert!(shared.publisher().is_none(), "the borrow holds the place");
        assert_eq!(publisher.publish(&entry(7)), 0);
    }
}
