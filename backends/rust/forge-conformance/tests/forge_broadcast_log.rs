//! The Rust broadcast log on real threads, judged for linearizability (RFC
//! §synth-5-S; ADR 0062 section 6, layer 3).
//!
//! One publisher and a few readers run against a log whose slots are reused many
//! times over, and every call is recorded with the instants it began and ended,
//! read from one shared clock. The record is sent through the JSON form every
//! backend writes (`tests/forge/conformance/broadcast_history.schema.json`),
//! read back, and judged by `sce_build::broadcast_history::check`, the checker the
//! command `sce-codegen check-broadcast-history` runs.
//!
//! When `SCE_BROADCAST_HISTORY_DIR` names a directory the text is also written
//! there as `<name>.json`, which is how the Rust gate has the shared command judge
//! this arm's runs as it judges the other backends'. A name is a shape, so a shape
//! recorded many times leaves its last recording.
//!
//! The runs are short on purpose. The checker's general search is the definition of
//! linearizability and is exponential in the readers' operations in the worst case
//! (ADR 0062 section 5, "What step 1 landed"), so a history is kept to a few dozen
//! operations a reader.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use sce_build::broadcast_history::{
    check, Call, History, OpenFrom, Operation, Outcome as Observed,
};
use sce_build::history_common::Verdict;
use sce_forge_runtime::broadcast_log::{BroadcastLog, Read};

/// How many entries each run publishes.
const ENTRIES: u64 = 24;

/// The most reads each reader makes. A reader that is ahead of the publisher
/// reads nothing but `Empty`, so this bounds the history and not the run.
const READS_PER_READER: usize = 36;

/// What an entry holds for the sequence number `seq`: both words, so that a copy
/// mixing two entries shows as two words that differ.
fn value_of(seq: u64) -> u64 {
    1_000 + seq
}

fn op(call: Call, outcome: Observed, invoked: u64, returned: u64) -> Operation {
    Operation {
        call,
        outcome,
        invoked,
        returned,
    }
}

/// A recorded history, sent through the JSON form and read back, so that what is
/// judged is what a file carries.
fn through_the_wire(history: History, name: &str) -> History {
    let text = history.to_json();
    let read = History::from_json(&text)
        .unwrap_or_else(|e| panic!("{name}: the writer's own output does not read: {e}"));
    assert_eq!(
        read.to_json(),
        text,
        "{name}: writing what was read gives the same text"
    );
    if let Some(dir) = std::env::var_os("SCE_BROADCAST_HISTORY_DIR") {
        let path = std::path::Path::new(&dir).join(format!("{name}.json"));
        std::fs::write(&path, &text)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
    read
}

/// Run one publisher and `readers` readers on their own threads over a log of
/// `SLOTS` slots, recording every call.
///
/// The clock is one SeqCst counter every thread reads before and after each call.
/// A read-modify-write chain on one atomic orders the readings with the calls
/// between them, so "returned before invoked" in the record means it in the run.
fn record_run<const SLOTS: usize>(readers: usize, opening: OpenFrom) -> History {
    let log = BroadcastLog::<2, SLOTS>::new();
    let clock = AtomicU64::new(1);
    let tick = || clock.fetch_add(1, Ordering::SeqCst);

    let (publisher_ops, reader_ops) = thread::scope(|scope| {
        let log = &log;
        let tick = &tick;
        let publishing = scope.spawn(move || {
            let mut publisher = log.publisher().expect("the only publisher");
            let mut ops = Vec::new();
            for seq in 0..ENTRIES {
                let value = value_of(seq);
                let invoked = tick();
                let took = publisher.publish(&[value, value]);
                let returned = tick();
                assert_eq!(took, seq, "the publisher takes sequence numbers in order");
                ops.push(op(
                    Call::Publish(value),
                    Observed::Published(took),
                    invoked,
                    returned,
                ));
                // Give a reader a chance between publishes, so the runs mix.
                thread::yield_now();
            }
            ops
        });

        let reading: Vec<_> = (0..readers)
            .map(|_| {
                scope.spawn(move || {
                    let mut ops = Vec::new();
                    let invoked = tick();
                    let mut reader = match opening {
                        OpenFrom::Oldest => log.reader_oldest(),
                        OpenFrom::Head => log.reader_head(),
                        OpenFrom::At(n) => log.reader_at(n),
                    };
                    let returned = tick();
                    ops.push(op(
                        Call::Open(opening),
                        Observed::Opened(reader.cursor()),
                        invoked,
                        returned,
                    ));
                    let mut last_seen = None;
                    for _ in 0..READS_PER_READER {
                        let mut out = [0u64; 2];
                        let invoked = tick();
                        let answer = log.read(&mut reader, &mut out);
                        let returned = tick();
                        let outcome = match answer {
                            Read::Item { seq } => {
                                assert_eq!(out[0], out[1], "an item is one whole entry");
                                assert_eq!(out[0], value_of(seq), "and the entry at its cursor");
                                assert!(
                                    last_seen.is_none_or(|last| seq > last),
                                    "a reader's sequence numbers only grow"
                                );
                                last_seen = Some(seq);
                                Observed::Item { seq, value: out[0] }
                            }
                            Read::Empty => {
                                thread::yield_now();
                                Observed::Empty
                            }
                            Read::Lagged { skipped } => Observed::Lagged { skipped },
                        };
                        ops.push(op(Call::Read, outcome, invoked, returned));
                    }
                    ops
                })
            })
            .collect();

        (
            publishing.join().unwrap(),
            reading
                .into_iter()
                .map(|r| r.join().unwrap())
                .collect::<Vec<_>>(),
        )
    });

    History {
        retention: BroadcastLog::<2, SLOTS>::RETENTION,
        publisher: publisher_ops,
        readers: reader_ops,
    }
}

fn assert_run_is_linearizable<const SLOTS: usize>(readers: usize, opening: OpenFrom, name: &str) {
    // A run is scheduled by the machine, so a shape is run several times: each
    // recording is judged, and the last one is the file the gate judges again.
    for round in 0..8 {
        let history = through_the_wire(record_run::<SLOTS>(readers, opening), name);
        assert_eq!(
            check(&history),
            Verdict::Linearizable,
            "{name}, round {round}"
        );
    }
}

#[test]
fn one_reader_over_a_log_of_two_slots_is_linearizable() {
    assert_run_is_linearizable::<2>(1, OpenFrom::Oldest, "rust_broadcast_r1_readers1");
}

#[test]
fn two_readers_over_a_log_of_two_slots_are_linearizable() {
    assert_run_is_linearizable::<2>(2, OpenFrom::Oldest, "rust_broadcast_r1_readers2");
}

#[test]
fn two_readers_over_a_log_of_four_slots_are_linearizable() {
    assert_run_is_linearizable::<4>(2, OpenFrom::Oldest, "rust_broadcast_r3_readers2");
}

#[test]
fn readers_opened_at_the_head_are_linearizable() {
    assert_run_is_linearizable::<3>(2, OpenFrom::Head, "rust_broadcast_r2_head");
}

#[test]
fn a_reader_opened_at_the_first_entry_is_told_how_far_it_lagged() {
    assert_run_is_linearizable::<2>(2, OpenFrom::At(0), "rust_broadcast_r1_at0");
}
