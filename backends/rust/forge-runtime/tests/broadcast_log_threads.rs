// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounded broadcast log on real threads (ADR 0062 section 6).
//!
//! These are ordinary tests and they run in the ordinary suite. Their job is to
//! be the program Miri watches: Miri interprets the run, checks every access for
//! undefined behaviour and, given several seeds, explores the weak-memory
//! outcomes a relaxed load may return. Each test asserts what the log must
//! deliver, so a run with no tool still checks the contract, and a run under
//! Miri checks that the contract holds under the outcomes the memory model
//! allows and not only the ones a machine happens to give.
//!
//! **ThreadSanitizer is deliberately not pointed at this file.** Every access to
//! a slot is an atomic one (the payload words are relaxed atomics, which is the
//! whole point of the sequence lock's form, ADR 0062 section 4), so there is no
//! plain access for the sanitizer to find a race on; it would report nothing
//! about a weakened order, and a gate that ran it would read as a watch it is
//! not. What a weakened order does to this log is a torn copy that validates, or
//! a lag that skipped nothing, and those are assertions: the loom models reach
//! them within their bound, and Miri here reaches the schedules its emulation
//! produces. Which of the log's orders Miri can see weakened is measured in the
//! ADR ("What the thread tests under Miri establish") and not assumed.
//!
//! The counts are small under Miri, which is about a hundred times slower than
//! native execution; `cfg(miri)` picks them.

#![cfg(all(not(loom), target_has_atomic = "64", target_has_atomic = "8"))]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Barrier;
use std::thread;

use sce_forge_runtime::broadcast_log::{BroadcastLog, Read, Reader};

/// Three words an entry: its sequence number, its complement and three times it.
/// A copy that mixes two entries cannot satisfy all three at once.
const WORDS: usize = 3;

/// Four slots, so a retention of three.
type Log = BroadcastLog<WORDS, 4>;

/// How many entries the publisher publishes.
const TOTAL: u64 = if cfg!(miri) { 40 } else { 20_000 };

fn entry(n: u64) -> [u64; WORDS] {
    [n, !n, n.wrapping_mul(3)]
}

fn is_entry(n: u64, words: &[u64; WORDS]) -> bool {
    *words == entry(n)
}

/// One read held to the specification of ADR 0062 section 2, as the loom models
/// hold it: the cursor moves exactly as the answer says, an item is the whole
/// entry at the cursor and one the window holds, and a lag skipped at least one.
fn read_and_check(log: &Log, reader: &mut Reader) -> Read {
    let before = reader.cursor();
    let lost_before = reader.lost();
    // The head only grows, so the head the read linearizes at is no lower than
    // this one, and an entry more than `RETENTION` behind it is outside the window.
    let head_before = log.head();
    let mut out = [u64::MAX; WORDS];
    let answer = log.read(reader, &mut out);
    match answer {
        Read::Item { seq } => {
            assert_eq!(seq, before, "an item is the entry at the cursor");
            assert!(
                seq + Log::RETENTION >= head_before,
                "an item is one the window holds: entry {seq}, head {head_before}"
            );
            assert!(is_entry(seq, &out), "an item is one whole entry: {out:?}");
            assert_eq!(reader.cursor(), seq + 1);
            assert_eq!(reader.lost(), lost_before);
        }
        Read::Empty => {
            assert_eq!(reader.cursor(), before, "an empty read moves nothing");
        }
        Read::Lagged { skipped } => {
            assert!(skipped >= 1, "a lag skips at least one entry");
            assert_eq!(reader.cursor(), before + skipped);
            assert_eq!(reader.lost(), lost_before + skipped);
        }
    }
    answer
}

/// Read until the cursor reaches `TOTAL`, checking every answer, and return how
/// many entries were handed over whole.
fn drain(log: &Log, reader: &mut Reader) -> u64 {
    let mut items = 0;
    while reader.cursor() < TOTAL {
        match read_and_check(log, reader) {
            Read::Item { .. } => items += 1,
            Read::Lagged { .. } => {}
            Read::Empty => thread::yield_now(),
        }
    }
    items
}

/// One publisher laps two readers over a log of three retained entries. Every
/// entry is either handed to a reader whole or counted as lost by it, so the two
/// add to the number published, and no read ever returns a mix of two entries.
#[test]
fn readers_see_whole_entries_in_order_while_the_publisher_laps_them() {
    let log = Log::new();
    thread::scope(|scope| {
        scope.spawn(|| {
            let mut publisher = log.publisher().expect("the only publisher");
            for n in 0..TOTAL {
                assert_eq!(publisher.publish(&entry(n)), n);
            }
        });
        for _ in 0..2 {
            scope.spawn(|| {
                // A reader opened at the oldest starts `RETENTION` behind the head
                // as it is when the thread gets to run, which is not 0 if the
                // publisher is already ahead; the entries before that are not its.
                let mut reader = log.reader_oldest();
                let start = reader.cursor();
                let items = drain(&log, &mut reader);
                assert_eq!(reader.cursor(), TOTAL);
                assert_eq!(
                    items + reader.lost(),
                    TOTAL - start,
                    "every entry from the reader's start is read or counted as lost"
                );
            });
        }
    });
    assert_eq!(log.head(), TOTAL);
}

/// A reader that has stopped never holds the publisher up, and it is told how far
/// it was left behind when it comes back: the first read is the lag, the next are
/// the entries the window still holds, in order.
#[test]
fn a_reader_that_stopped_is_told_how_far_and_resumes_at_the_oldest() {
    let log = Log::new();
    let mut idle = log.reader_oldest();
    thread::scope(|scope| {
        scope.spawn(|| {
            let mut publisher = log.publisher().expect("the only publisher");
            for n in 0..TOTAL {
                publisher.publish(&entry(n));
            }
        });
    });
    // `TOTAL` entries published and a window of `RETENTION` (three): the reader at
    // 0 is `TOTAL - 3` behind the oldest retained entry.
    let skipped = TOTAL - Log::RETENTION;
    assert_eq!(
        read_and_check(&log, &mut idle),
        Read::Lagged { skipped },
        "the reader that stopped is told how many it missed"
    );
    for n in skipped..TOTAL {
        assert_eq!(read_and_check(&log, &mut idle), Read::Item { seq: n });
    }
    assert_eq!(read_and_check(&log, &mut idle), Read::Empty);
    assert_eq!(idle.lost(), skipped);
}

/// Two threads ask for the publisher at once and exactly one is given it, whichever
/// asks first, and the place is free again when the winner drops its handle.
#[test]
fn the_publisher_is_given_to_one_thread_at_a_time() {
    let log = Log::new();
    let won = AtomicUsize::new(0);
    let asked = Barrier::new(2);
    thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                let handle = log.publisher();
                if handle.is_some() {
                    won.fetch_add(1, Ordering::Relaxed);
                }
                // The winner keeps its handle until the other has asked, so the
                // place cannot be seen free by a thread that asks late.
                asked.wait();
                drop(handle);
            });
        }
    });
    assert_eq!(
        won.load(Ordering::Relaxed),
        1,
        "exactly one thread is the publisher"
    );
    assert!(
        log.publisher().is_some(),
        "the place is free once the winner has dropped it"
    );
}
