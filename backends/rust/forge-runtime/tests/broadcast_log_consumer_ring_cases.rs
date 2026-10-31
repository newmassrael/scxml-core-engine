// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounded broadcast log against the cases of a downstream consumer's own ring
//! (ADR 0062 section 6, layer 5).
//!
//! The consumer keeps an event ring that is the same shape as this kind: one
//! producer, any number of readers that free nothing, overwrite-oldest, and a
//! reader told how many entries it missed. Its unit tests are cases written by
//! someone who needed that behaviour, so the kind is held to them: the
//! deterministic vectors are run here with the ring's slot count read as the
//! retention (a ring of `S` slots keeps `S` records between writes, and a log of
//! retention `S` has `S + 1` slots), and its generated test is run with the same
//! stream, seeds and step rule.
//!
//! Where the two differ, a test says so and pins the behaviour of the log, so a
//! change to either side is seen here. The differences found are three, listed
//! at the end of this file as `deviation:` tests. The first, a reader that binds
//! at its first poll, is answered by `reader_deferred()`, which is the ring's own
//! `reader()`, and the generated test runs here for both kinds of reader.
//!
//! What this file does not do: it does not link the consumer's crate, and it does
//! not step through the middle of a publish (the log's `publish` is one call; the
//! consumer's test steps open, fill and close separately, and here only the close
//! is an action, which keeps its random stream and its reader-to-writer ratio).
//! The ring compares 32-bit words modulo `2^32` and asks its readers to read at
//! least once per `2^31` records; that is the narrow row's stamp of ADR 0062
//! section 4 at 32 bits, whose bound is stated there and held by its own tests,
//! and a run of `2^31` records is not one this file makes.

#![cfg(all(not(loom), target_has_atomic = "64", target_has_atomic = "8"))]

use sce_forge_runtime::broadcast_log::{BroadcastLog, Read, Reader};

/// Four words an entry, each a function of the entry's sequence number, so that an
/// entry put together from two writes is seen as one.
const WORDS: usize = 4;

fn entry(n: u64) -> [u64; WORDS] {
    [
        n,
        n.wrapping_mul(7),
        n.wrapping_mul(0x9E37_79B9_7F4A_7C15),
        !n ^ 0xAAAA_AAAA_AAAA_AAAA,
    ]
}

/// A ring of `S` slots holds `S` records between two writes, which is a retention of
/// `S`, which the log keeps in `S + 1` slots.
type Ring4 = BroadcastLog<WORDS, 5>;
type Ring5 = BroadcastLog<WORDS, 6>;

fn publish_n<const S: usize>(log: &BroadcastLog<WORDS, S>, count: u64) {
    let mut publisher = log.publisher().expect("the only publisher");
    for _ in 0..count {
        let n = log.head();
        assert_eq!(publisher.publish(&entry(n)), n);
    }
}

/// What a read answered, as the consumer's tests write it: the sequence number of a
/// record, or the number of records missed, negative.
fn drain<const S: usize>(log: &BroadcastLog<WORDS, S>, reader: &mut Reader) -> Vec<i64> {
    let mut seen = Vec::new();
    loop {
        let mut out = [0u64; WORDS];
        match log.read(reader, &mut out) {
            Read::Item { seq } => {
                assert_eq!(out, entry(seq), "entry {seq} is one whole write");
                seen.push(seq as i64);
            }
            Read::Lagged { skipped } => seen.push(-(skipped as i64)),
            Read::Empty => return seen,
        }
    }
}

#[test]
fn overwrite_oldest_keeps_the_latest_and_a_reader_that_fell_behind_is_told_how_many() {
    let log = Ring4::new();
    let mut early = log.reader_oldest();
    let mut publisher = log.publisher().expect("the only publisher");
    assert_eq!(publisher.publish(&entry(0)), 0);
    assert_eq!(drain(&log, &mut early), [0]);
    for n in 1..=10 {
        assert_eq!(publisher.publish(&entry(n)), n);
    }
    // 11 stored, 4 alive (7..=10); the reader has read 0 and so missed 1..=6.
    assert_eq!(drain(&log, &mut early), [-6, 7, 8, 9, 10]);
    // A reader that starts now starts at the oldest alive.
    assert_eq!(drain(&log, &mut log.reader_oldest()), [7, 8, 9, 10]);
}

#[test]
fn a_reader_started_at_a_number_reads_from_there_and_is_told_of_a_gap_before_it() {
    let log = Ring4::new();
    publish_n(&log, 10);
    // Alive: 6..=9 of the ten; a reader at 8 has lost nothing.
    assert_eq!(drain(&log, &mut log.reader_at(8)), [8, 9]);
    // A reader at 2 is told that 2..=5 are gone, then reads the rest.
    assert_eq!(drain(&log, &mut log.reader_at(2)), [-4, 6, 7, 8, 9]);
    // A reader at the head reads nothing, and is not behind when the log gets there.
    let mut at_head = log.reader_at(10);
    assert_eq!(drain(&log, &mut at_head), Vec::<i64>::new());
    let mut publisher = log.publisher().expect("the only publisher");
    assert_eq!(publisher.publish(&entry(10)), 10);
    assert_eq!(drain(&log, &mut at_head), [10]);
}

#[test]
fn readers_are_independent_and_none_of_them_frees_anything() {
    let log = Ring4::new();
    let mut a = log.reader_oldest();
    let mut b = log.reader_oldest();
    publish_n(&log, 3);
    assert_eq!(drain(&log, &mut a), [0, 1, 2]);
    // A reader that has read everything has not made room for anyone: the other still
    // reads all three, and the log is no fuller or emptier for it.
    assert_eq!(drain(&log, &mut b), [0, 1, 2]);
    assert_eq!(log.head(), 3);
    assert_eq!(a.lost(), 0);
    assert_eq!(b.lost(), 0);
}

/// A small deterministic generator, so a failing seed is a failing seed everywhere. The
/// shifts are the consumer's.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// What one reader has been told, held against the cursor it keeps. A reader's cursor must
/// move by exactly one for an entry and by exactly `skipped` for a lag, so an entry counted
/// twice or a lag that hides one is a cursor that disagrees.
#[derive(Default)]
struct Watch {
    start: Option<u64>,
    seen: u64,
    lost: u64,
}

impl Watch {
    fn read<const S: usize>(&mut self, log: &BroadcastLog<WORDS, S>, reader: &mut Reader) -> Read {
        // A deferred reader has no cursor before its first read, so there is nothing to
        // hold the first answer against: its start is what that read bound it to.
        let held = reader.is_bound();
        let before = reader.cursor();
        let mut out = [0u64; WORDS];
        let answer = log.read(reader, &mut out);
        let after = reader.cursor();
        match answer {
            Read::Item { seq } => {
                assert_eq!(out, entry(seq), "entry {seq} is not one write");
                if held {
                    assert_eq!(seq, before, "a reader sees the sequence in order");
                }
                assert_eq!(after, seq + 1);
                self.start.get_or_insert(seq);
                self.seen += 1;
            }
            Read::Lagged { skipped } => {
                assert!(skipped >= 1);
                if held {
                    assert_eq!(after, before + skipped, "a lag is exactly what was skipped");
                }
                self.start.get_or_insert(after - skipped);
                self.lost += skipped;
            }
            Read::Empty => {
                if held {
                    assert_eq!(after, before, "nothing read, nothing moved");
                }
                self.start.get_or_insert(after);
            }
        }
        answer
    }
}

/// The consumer's generated test: one thread plays the producer and three readers in a random
/// order, 200 records, seeds 1 to 300, a ring of 5 slots. Whatever the order, a reader sees
/// entries in sequence, each whole, and a lag exactly where entries were overwritten. The
/// producer's three micro-steps (open, fill, close) are kept as three random draws and only the
/// third publishes, so the reader-to-writer ratio and the stream are the consumer's. It runs
/// twice: with readers opened by `reader_oldest()`, and with readers opened by
/// `reader_deferred()`, which is the ring's own `reader()` (no cursor until the first poll).
#[test]
fn whatever_the_interleaving_a_reader_sees_whole_entries_in_order_with_lags_where_they_were_lost() {
    whatever_the_interleaving(false);
}

#[test]
fn whatever_the_interleaving_a_deferred_reader_sees_whole_entries_in_order_with_lags_where_they_were_lost(
) {
    whatever_the_interleaving(true);
}

fn whatever_the_interleaving(deferred: bool) {
    for seed in 1..=300u64 {
        let mut rng = Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1);
        let log = Ring5::new();
        let mut publisher = log.publisher().expect("the only publisher");
        let mut readers: Vec<(Reader, Watch)> = (0..3)
            .map(|_| {
                let reader = if deferred {
                    log.reader_deferred()
                } else {
                    log.reader_oldest()
                };
                (reader, Watch::default())
            })
            .collect();
        let mut written = 0u64;
        // 0 = idle, 1 = open, 2 = filled
        let mut phase = 0u8;
        let total = 200u64;
        while written < total {
            match rng.next() % 4 {
                0 | 1 => match phase {
                    0 => phase = 1,
                    1 => phase = 2,
                    _ => {
                        assert_eq!(publisher.publish(&entry(written)), written);
                        written += 1;
                        phase = 0;
                    }
                },
                _ => {
                    let pick = (rng.next() % 3) as usize;
                    let (reader, watch) = &mut readers[pick];
                    watch.read(&log, reader);
                }
            }
        }
        for (reader, watch) in &mut readers {
            while watch.read(&log, reader) != Read::Empty {}
            assert_eq!(
                reader.cursor(),
                total,
                "seed {seed}: the reader reached the end"
            );
            let start = watch.start.expect("the reader read at least once");
            assert_eq!(
                watch.seen + watch.lost,
                total - start,
                "seed {seed}: every entry from the start is seen or counted lost"
            );
        }
    }
}

// ---- deviation: where the log and the consumer's ring differ --------------------------------

/// The ring's `reader()` has no cursor until its first poll, and takes the oldest record alive
/// at that moment, so it is told of no gap however much was written between its creation and
/// its first poll. The log's `reader_deferred()` is that reader: it binds at its first read.
#[test]
fn a_deferred_reader_is_the_rings_late_reader() {
    let log = Ring4::new();
    publish_n(&log, 3);
    let mut late = log.reader_deferred();
    publish_n(&log, 10);
    // 13 published, retention 4: the oldest retained is 9, and the reader is told of nothing.
    assert_eq!(drain(&log, &mut late), [9, 10, 11, 12]);
    assert_eq!(late.lost(), 0);
}

/// deviation: `reader_oldest()` is not the ring's `reader()`. It takes the oldest retained
/// entry at the moment it is called, so a reader that waits is told of the lag on its first
/// read. A consumer that wants the ring's behaviour opens its reader with `reader_deferred()`.
#[test]
fn deviation_a_reader_opened_early_is_told_of_the_lag_on_its_first_read() {
    let log = Ring4::new();
    publish_n(&log, 3);
    let mut early = log.reader_oldest();
    publish_n(&log, 10);
    // 13 published, retention 4: the oldest retained is 9, and the reader holds 0.
    assert_eq!(drain(&log, &mut early), [-9, 9, 10, 11, 12]);
    // Opened at its first poll, it is told of nothing.
    assert_eq!(drain(&log, &mut log.reader_oldest()), [9, 10, 11, 12]);
}

/// deviation: a position ahead of the head. The ring keeps the cursor it was given, so
/// the reader is `Empty` until the producer reaches that number and then reads it, which
/// leaves the records between the head and it unread and uncounted. The log's `reader_at`
/// takes a position past the head to be the head, so the reader reads every entry from the
/// head on.
#[test]
fn deviation_a_position_ahead_of_the_head_is_the_head() {
    let log = Ring4::new();
    publish_n(&log, 10);
    let mut ahead = log.reader_at(12);
    assert_eq!(ahead.cursor(), 10, "a position past the head is the head");
    publish_n(&log, 3);
    assert_eq!(drain(&log, &mut ahead), [10, 11, 12]);
}

/// deviation: what is retained while a write is in flight. A ring of `S` slots holds `S`
/// records between writes, and `S - 1` while the producer is writing the slot of the oldest.
/// The log keeps `S + 1` slots for a retention of `S`, so the retention holds at every
/// instant, a guarantee the ring does not make. Between two publishes (the only moments this
/// file can observe) the two agree, which is why the vectors above hold.
#[test]
fn deviation_the_retention_holds_between_publishes_at_the_ring_s_slot_count() {
    let log = Ring4::new();
    publish_n(&log, 9);
    assert_eq!(drain(&log, &mut log.reader_oldest()), [5, 6, 7, 8]);
    assert_eq!(Ring4::RETENTION, 4);
}
