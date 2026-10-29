// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Loom models of the bounded broadcast log (RFC §synth-5-S; ADR 0062 section 6,
//! layer 2: the memory-model checker that proves the ordering the TLA+ model assumes).
//!
//! `docs/formal/BroadcastLog.tla` proves the protocol under sequential
//! consistency. These models run the same algorithm text that ships under every
//! interleaving of the threads and every outcome the C11 memory model allows
//! their atomics, and fail when a reader sees a copy mixing two entries, an entry
//! other than the one at its cursor, or a lag that skipped nothing. Weaken one of
//! the log's orderings or fences and a model here fails (the casefile
//! `sce-build/tests/mutations/a_broadcast_log_orders_every_slot_hand_over.cases`
//! does exactly that, one decision at a time).
//!
//! **Bounded, and said so.** Each model runs under three preemptions, exhaustively
//! within that bound (the bound the context-bounding literature found enough for
//! the bugs that matter), so what a passing model establishes is "no violation
//! within three preemptions", not "no violation". The recorded histories judged by
//! `sce-codegen check-broadcast-history` and the sanitizers reach the schedules a
//! machine happens to give.
//!
//! An entry is two words, both its sequence number, so a copy that mixes two
//! entries shows as two words that differ. The log has two slots and so retains
//! one entry, and three entries are published, so the second slot is reused.
//!
//! The target has `test = false` and builds only under `--cfg loom`
//! (`backends/rust/forge-runtime/Cargo.toml` gives the command). Built without
//! it, it refuses to compile: a model file that compiled to nothing would pass.

#[cfg(not(loom))]
compile_error!(
    "the loom models run only under `--cfg loom`; see backends/rust/forge-runtime/Cargo.toml"
);

#[cfg(loom)]
mod models {
    use loom::sync::Arc;
    use loom::thread;
    use sce_forge_runtime::broadcast_log::{BroadcastLog, Read, Reader};

    type Log = BroadcastLog<2, 2>;

    /// Run `body` under every interleaving within three preemptions.
    fn model(body: impl Fn() + Sync + Send + 'static) {
        let mut builder = loom::model::Builder::new();
        builder.preemption_bound = Some(3);
        builder.check(body);
    }

    fn entry(n: u64) -> [u64; 2] {
        [n, n]
    }

    /// One read, held to the specification of ADR 0062 section 2: the cursor
    /// moves exactly as the answer says, an item is the whole entry at the cursor
    /// and one the window holds, and a lag skipped at least one entry and put the
    /// cursor at or before the head.
    fn read_and_check(log: &Log, reader: &mut Reader) -> Read {
        let before = reader.cursor();
        let lost_before = reader.lost();
        // The head only grows, so the head the read linearizes at is no lower than
        // this one, and an entry more than `RETENTION` behind it is outside the
        // window at that point whatever the publisher does meanwhile.
        let head_before = log.head();
        let mut out = [u64::MAX; 2];
        let answer = log.read(reader, &mut out);
        match answer {
            Read::Item { seq } => {
                assert_eq!(seq, before, "an item is the entry at the cursor");
                assert!(
                    seq + Log::RETENTION >= head_before,
                    "an item is one the window holds: entry {seq}, head {head_before}"
                );
                assert_eq!(out, entry(seq), "an item is one whole entry, not a mix");
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

    /// One reader against the publisher while the second slot is reused.
    #[test]
    fn a_reader_sees_whole_entries_at_its_cursor() {
        model(|| {
            let log = Arc::new(Log::new());
            let far = log.clone();
            let publisher = thread::spawn(move || {
                let mut publisher = far.publisher().expect("the only publisher");
                for n in 0..3 {
                    assert_eq!(publisher.publish(&entry(n)), n);
                }
            });

            let mut reader = log.reader_oldest();
            for _ in 0..3 {
                read_and_check(&log, &mut reader);
            }
            publisher.join().unwrap();

            // Everything is published: the window is the last entry, and a
            // reader at the oldest finds exactly it and then the head.
            let mut late = log.reader_oldest();
            assert_eq!(read_and_check(&log, &mut late), Read::Item { seq: 2 });
            assert_eq!(read_and_check(&log, &mut late), Read::Empty);
        });
    }

    /// Two readers read the same entries without taking them from each other:
    /// each holds its own cursor, and each is told its own lag.
    #[test]
    fn two_readers_each_hold_their_own_cursor() {
        model(|| {
            let log = Arc::new(Log::new());
            let far = log.clone();
            let publisher = thread::spawn(move || {
                let mut publisher = far.publisher().expect("the only publisher");
                for n in 0..3 {
                    publisher.publish(&entry(n));
                }
            });

            let other = log.clone();
            let second = thread::spawn(move || {
                let mut reader = other.reader_oldest();
                for _ in 0..2 {
                    read_and_check(&other, &mut reader);
                }
            });

            let mut reader = log.reader_oldest();
            for _ in 0..2 {
                read_and_check(&log, &mut reader);
            }
            second.join().unwrap();
            publisher.join().unwrap();
        });
    }

    /// A reader that starts at the first entry while the publisher laps it is
    /// either handed that entry whole or told how far it was left behind.
    #[test]
    fn a_reader_the_publisher_laps_is_told_how_far() {
        model(|| {
            let log = Arc::new(Log::new());
            let far = log.clone();
            let publisher = thread::spawn(move || {
                let mut publisher = far.publisher().expect("the only publisher");
                for n in 0..3 {
                    publisher.publish(&entry(n));
                }
            });

            let mut reader = log.reader_at(0);
            let first = read_and_check(&log, &mut reader);
            assert!(
                matches!(
                    first,
                    Read::Item { seq: 0 } | Read::Empty | Read::Lagged { .. }
                ),
                "the first read from entry 0 is that entry, nothing yet, or a lag: {first:?}"
            );
            read_and_check(&log, &mut reader);
            publisher.join().unwrap();
        });
    }
}
