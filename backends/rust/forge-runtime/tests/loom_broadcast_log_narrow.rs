// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Loom models of the narrow broadcast log (ADR 0062 section 4, "Stamp width"),
//! the memory-model layer of the proof that a stamp narrower than 64 bits, with its
//! head and cursors taken modulo the stamp space, still gives the answers of the
//! specification while a reader keeps the wrap bound.
//!
//! `docs/formal/BroadcastLogWrapped.tla` proves the protocol with the wrap under
//! sequential consistency, and the wrap bound necessary. These models run the
//! algorithm text that ships (`sce_forge_runtime::broadcast_log_core`) under every
//! interleaving and every outcome the C11 memory model allows within three
//! preemptions, over a log whose stamp is **3 bits** (`Narrow<3>`: the stamp space
//! is 8 values, a stamp tells 4 entries of a slot apart): a 32-bit stamp does not
//! wrap in a model, so a model over it would say nothing about the wrap.
//!
//! The log has two slots (retention 1), an entry is two words, both its count, and
//! the models start from a log that has already published `WARM` entries (on one
//! thread, where every access is deterministic), so that a few publishes cross the
//! wrap of the head (at 8) and the wrap of the stamps (at 4 entries), which a run
//! from an empty log cannot reach in so few steps.
//!
//! **What each model checks.** A reader's answer is held against the *exact*
//! specification. The publisher counts what it has published in a plain atomic after
//! each publish (`ghost`), which the readers read before and after each call: the
//! number of entries completed at any instant of a call is between the two readings
//! (the second plus one, as the counter is raised after the publish returns). An
//! `Item` is the whole entry at the reader's own full count, and a count the window
//! of some head in that range holds; an `Empty` is at some head in that range; a
//! `Lagged` moves the reader to the oldest retained entry of some head in that range,
//! with a count of at least one. The reader keeps its own count (`abs`, a full
//! `u64`, as the ADR's argument has each reader keep its own count in 64 bits) beside
//! the log's cursor, which holds it modulo 8, and every answer must keep them
//! equal. None of this is "no crash": a reader told the wrong count, or the wrong
//! entry, fails.
//!
//! The premise of the theorem (a reader is never `K = 4` or more behind the newest
//! publish begun, during a read) holds in every model below by construction, since
//! at most 3 entries are published after a reader's cursor is set; the model
//! `outside_the_bound_a_later_lap_is_mistaken_for_the_readers_own` breaks it
//! and must fail, which shows that the bound is not decorative.
//!
//! **Bounded, and said so.** What a passing model establishes is "no violation
//! within three preemptions". The target has `test = false` and builds only under
//! `--cfg loom` (`backends/rust/forge-runtime/Cargo.toml` gives the command).

#[cfg(not(loom))]
compile_error!(
    "the loom models run only under `--cfg loom`; see backends/rust/forge-runtime/Cargo.toml"
);

#[cfg(loom)]
mod models {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::sync::Arc;
    use loom::thread;
    use sce_forge_runtime::broadcast_log_core::{BroadcastLog, Narrow, Read, Reader};

    /// Two slots over a 3-bit stamp: the stamp space is 8, `K` is 4, the retention
    /// is 1.
    type Log = BroadcastLog<2, 2, Narrow<3>>;

    /// The low 3 bits: a count modulo the stamp space.
    const MASK: u64 = 7;

    /// Run `body` under every interleaving within three preemptions.
    fn model(body: impl Fn() + Sync + Send + 'static) {
        let mut builder = loom::model::Builder::new();
        builder.preemption_bound = Some(3);
        builder.check(body);
    }

    /// The entry whose full count is `count`: both words are it, so a copy that
    /// mixes two entries shows as words that differ, and a copy of the entry a lap
    /// later shows as words that are not the reader's count.
    fn entry(count: u64) -> [u32; 2] {
        [count as u32, count as u32]
    }

    /// Publish the entries `from..to` into the log, on this thread.
    fn publish_entries(log: &Log, from: u64, to: u64, ghost: &AtomicUsize) {
        let mut publisher = log.publisher().expect("the only publisher");
        for count in from..to {
            // The log's sequence number is the count modulo the stamp space.
            assert_eq!(publisher.publish(&entry(count)), count & MASK);
            ghost.store(count as usize + 1, Ordering::SeqCst);
        }
    }

    /// The one count in `lo..=hi` that is `cursor` modulo 8. The range is narrower
    /// than 8, so there is exactly one.
    fn recover(cursor: u64, lo: u64, hi: u64) -> u64 {
        let found: Vec<u64> = (lo..=hi).filter(|count| count & MASK == cursor).collect();
        assert_eq!(found.len(), 1, "cursor {cursor} in {lo}..={hi}: {found:?}");
        found[0]
    }

    /// One read, held to the specification of ADR 0062 section 2 as the module
    /// documentation says. `abs` is the reader's own full count.
    fn read_and_check(log: &Log, reader: &mut Reader, abs: &mut u64, ghost: &AtomicUsize) -> Read {
        assert_eq!(
            reader.cursor(),
            *abs & MASK,
            "the cursor is the count modulo 8"
        );
        let lost_before = reader.lost();
        // The entries completed at any instant of the call are at least these...
        let lo = ghost.load(Ordering::SeqCst) as u64;
        let mut out = [u32::MAX; 2];
        let answer = log.read(reader, &mut out);
        // ...and at most these (the counter is raised after a publish returns).
        let hi = ghost.load(Ordering::SeqCst) as u64 + 1;
        match answer {
            Read::Item { seq } => {
                assert_eq!(seq, *abs & MASK, "an item is the entry at the cursor");
                assert!(
                    *abs < hi && *abs + Log::RETENTION >= lo,
                    "an item is one the window of some head in {lo}..={hi} holds: entry {abs}"
                );
                assert_eq!(out, entry(*abs), "an item is the whole entry at the cursor");
                *abs += 1;
                assert_eq!(reader.lost(), lost_before);
            }
            Read::Empty => {
                assert!(
                    lo <= *abs && *abs <= hi,
                    "an empty read is at the head: entry {abs}, heads {lo}..={hi}"
                );
                assert_eq!(reader.lost(), lost_before);
            }
            Read::Lagged { skipped } => {
                assert!(skipped >= 1, "a lag skips at least one entry");
                *abs += skipped;
                assert!(
                    lo <= *abs + Log::RETENTION && *abs + Log::RETENTION <= hi,
                    "a lag lands on the oldest retained of some head in {lo}..={hi}: entry {abs}"
                );
                assert_eq!(reader.lost(), lost_before + skipped);
            }
        }
        assert_eq!(reader.cursor(), *abs & MASK, "the cursor follows the count");
        answer
    }

    /// A log that has published `warm` entries, and the counter that says so.
    fn warmed(warm: u64) -> (Arc<Log>, Arc<AtomicUsize>) {
        let log = Arc::new(Log::new());
        let ghost = Arc::new(AtomicUsize::new(0));
        publish_entries(&log, 0, warm, &ghost);
        (log, ghost)
    }

    /// One reader against the publisher while the log's head, its stamps and its
    /// second slot's laps wrap.
    #[test]
    fn a_narrow_reader_sees_whole_entries_across_the_wrap() {
        model(|| {
            // Six entries: the head is 6, and entries 6, 7 and 8 are published
            // next, which takes the head through 8 (stored as 0) and the stamps
            // through the repeat at 4.
            let (log, ghost) = warmed(6);
            let mut reader = log.reader_head();
            let mut abs = 6;
            assert_eq!(reader.cursor(), 6);

            let far = log.clone();
            let counted = ghost.clone();
            let publisher = thread::spawn(move || publish_entries(&far, 6, 9, &counted));

            for _ in 0..3 {
                read_and_check(&log, &mut reader, &mut abs, &ghost);
            }
            publisher.join().unwrap();

            // Nine entries are published: the head is stored as 9 mod 8 = 1, the
            // window is the last entry (8), and a reader at the oldest finds exactly
            // it, at the residue 0, and then the head.
            let mut late = log.reader_oldest();
            let mut late_abs = 8;
            assert_eq!(late.cursor(), 8 & MASK);
            assert_eq!(
                read_and_check(&log, &mut late, &mut late_abs, &ghost),
                Read::Item { seq: 0 }
            );
            assert_eq!(
                read_and_check(&log, &mut late, &mut late_abs, &ghost),
                Read::Empty
            );
        });
    }

    /// Two readers read the same entries across the wrap without taking them from
    /// each other: each holds its own cursor and its own count, and each is told its
    /// own lag. The second opens while the publisher runs, at a head it takes from
    /// the log and recovers the count of from the publisher's.
    #[test]
    fn two_narrow_readers_each_hold_their_own_cursor() {
        model(|| {
            let (log, ghost) = warmed(6);
            let far = log.clone();
            let counted = ghost.clone();
            let publisher = thread::spawn(move || publish_entries(&far, 6, 9, &counted));

            let other = log.clone();
            let other_ghost = ghost.clone();
            let second = thread::spawn(move || {
                let lo = other_ghost.load(Ordering::SeqCst) as u64;
                let mut reader = other.reader_head();
                let hi = other_ghost.load(Ordering::SeqCst) as u64 + 1;
                // The head it opened at is one of the counts the publisher had.
                let mut abs = recover(reader.cursor(), lo, hi);
                for _ in 0..2 {
                    read_and_check(&other, &mut reader, &mut abs, &other_ghost);
                }
            });

            let mut reader = log.reader_head();
            let mut abs = 6;
            for _ in 0..2 {
                read_and_check(&log, &mut reader, &mut abs, &ghost);
            }
            second.join().unwrap();
            publisher.join().unwrap();
        });
    }

    /// A reader opened at the oldest entry while a young log is being written: the
    /// last slot may or may not have been written when it looks, which is how the
    /// log tells a count that has not reached the retention from one that has.
    #[test]
    fn a_narrow_reader_opened_at_the_oldest_of_a_young_log_is_at_the_first_entry() {
        model(|| {
            let log = Arc::new(Log::new());
            let ghost = Arc::new(AtomicUsize::new(0));
            let far = log.clone();
            let counted = ghost.clone();
            let publisher = thread::spawn(move || publish_entries(&far, 0, 3, &counted));

            let lo = ghost.load(Ordering::SeqCst) as u64;
            let mut reader = log.reader_oldest();
            let hi = ghost.load(Ordering::SeqCst) as u64 + 1;
            // The oldest retained entry is max(0, H - 1) for the head H the open
            // took effect at, somewhere in `lo..=hi`.
            let first = lo.saturating_sub(Log::RETENTION);
            let last = hi.saturating_sub(Log::RETENTION);
            let mut abs = recover(reader.cursor(), first, last);
            for _ in 0..3 {
                read_and_check(&log, &mut reader, &mut abs, &ghost);
            }
            publisher.join().unwrap();
        });
    }

    /// The same open after the head has wrapped, when the head's residue is below
    /// the retention and a log that merely counted would take it for a young one.
    #[test]
    fn a_narrow_reader_opened_at_the_oldest_after_the_wrap_is_at_the_last_entry_held() {
        model(|| {
            // Seven entries: the last slot has been written, the head is 7, and
            // entries 7 and 8 are published next, which makes the head 8 (stored 0)
            // and 9 (stored 1).
            let (log, ghost) = warmed(7);
            let far = log.clone();
            let counted = ghost.clone();
            let publisher = thread::spawn(move || publish_entries(&far, 7, 9, &counted));

            let lo = ghost.load(Ordering::SeqCst) as u64;
            let mut reader = log.reader_oldest();
            let hi = ghost.load(Ordering::SeqCst) as u64 + 1;
            let mut abs = recover(reader.cursor(), lo - Log::RETENTION, hi - Log::RETENTION);
            for _ in 0..2 {
                read_and_check(&log, &mut reader, &mut abs, &ghost);
            }
            publisher.join().unwrap();
        });
    }

    /// The bound is not decorative. A reader that loaded the head with the first
    /// entry in the window and was then held while the publisher began entry 4 (one
    /// half of the stamp space on) is shown the entry a lap on as its own: the
    /// slot's stamp is the same value, and the copy validates. Entries 1 to 5 are
    /// published after the reader's cursor is set at 0, so the reader is up to 6
    /// behind, which is above `K = 4`: the premise of the theorem does not hold, and
    /// a model that held it to the specification fails. Were this model to pass,
    /// the others would be proving nothing about the wrap.
    #[test]
    #[should_panic]
    fn outside_the_bound_a_later_lap_is_mistaken_for_the_readers_own() {
        model(|| {
            let log = Arc::new(Log::new());
            let ghost = Arc::new(AtomicUsize::new(0));
            let mut reader = log.reader_head();
            let mut abs = 0;
            // Entry 0 is published here, so the reader is inside the window of the
            // head it will load, and the slot of entry 0 is entry 4's as well.
            publish_entries(&log, 0, 1, &ghost);

            let far = log.clone();
            let counted = ghost.clone();
            let publisher = thread::spawn(move || publish_entries(&far, 1, 6, &counted));

            read_and_check(&log, &mut reader, &mut abs, &ghost);
            publisher.join().unwrap();
        });
    }
}
