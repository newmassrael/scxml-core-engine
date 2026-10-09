// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust arm of the `queue` kind's verification (SCE Protocol-Synthesis
//! RFC §synth-5-P), layers 1 and 2:
//!
//! - **Contract scenarios.** Every scenario in
//!   `tests/forge/conformance/queue_contract.json` runs against the runtime
//!   queue its storage row names. A row this arm has no runtime for is
//!   refused by name, never skipped.
//! - **Linearizability.** Real runs of the one-producer, one-consumer queue
//!   on two threads, and of the SCQ queue with several producers and several
//!   consumers, record what each participant observed, and
//!   `sce_build::queue_history::check` judges the record.
//!
//! The checker is tested here as well, against histories built by hand,
//! because a checker that accepted everything would make the second layer
//! pass whatever the queue did.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sce_build::queue_history::{check, Call, History, Operation, Outcome, Refusal, Verdict};
use sce_forge_runtime::queue::scq::Scq;
use sce_forge_runtime::queue::spsc::Spsc;
use sce_forge_runtime::queue::PushError;
use serde_json::Value;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Layer 1 — contract scenarios
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn contract() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/forge/conformance/queue_contract.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

/// An element that records its own destruction, so a scenario can say
/// which elements a destroyed queue destroyed.
struct Tracked {
    value: u64,
    destroyed: Rc<RefCell<Vec<u64>>>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.destroyed.borrow_mut().push(self.value);
    }
}

fn field<'a>(value: &'a Value, key: &str, context: &str) -> &'a Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("{context}: missing \"{key}\""))
}

/// What a scenario asks of a queue, so one reading of the steps serves every
/// runtime this arm has. Each runtime reaches its producer and consumer its
/// own way; the scenarios do not know how.
trait Subject {
    const CAPACITY: usize;
    fn new() -> Self;
    fn capacity(&self) -> usize;
    fn push(&mut self, element: Tracked) -> Result<(), PushError<Tracked>>;
    fn pop(&mut self) -> Option<Tracked>;
}

impl<const N: usize> Subject for Spsc<Tracked, N> {
    const CAPACITY: usize = Spsc::<Tracked, N>::CAPACITY;

    fn new() -> Self {
        Spsc::new()
    }

    fn capacity(&self) -> usize {
        Spsc::capacity(self)
    }

    fn push(&mut self, element: Tracked) -> Result<(), PushError<Tracked>> {
        let (mut producer, _consumer) = self.split();
        producer.try_push(element)
    }

    fn pop(&mut self) -> Option<Tracked> {
        let (_producer, mut consumer) = self.split();
        consumer.try_pop()
    }
}

impl<const N: usize, const R: usize> Subject for Scq<Tracked, N, R> {
    const CAPACITY: usize = Scq::<Tracked, N, R>::CAPACITY;

    fn new() -> Self {
        Scq::new()
    }

    fn capacity(&self) -> usize {
        Scq::capacity(self)
    }

    fn push(&mut self, element: Tracked) -> Result<(), PushError<Tracked>> {
        self.producer()
            .expect("a scenario holds one handle at a time")
            .try_push(element)
    }

    fn pop(&mut self) -> Option<Tracked> {
        self.consumer()
            .expect("a scenario holds one handle at a time")
            .try_pop()
    }
}

fn run_scenario<S: Subject>(id: &str, steps: &[Value]) {
    let destroyed = Rc::new(RefCell::new(Vec::new()));
    let mut queue = Some(S::new());

    for (index, step) in steps.iter().enumerate() {
        let context = format!("scenario {id} step {index}");
        let op = field(step, "op", &context)
            .as_str()
            .unwrap_or_else(|| panic!("{context}: \"op\" is not a string"));
        let expect = field(step, "expect", &context);
        let live = queue
            .as_mut()
            .unwrap_or_else(|| panic!("{context}: a step follows \"destroy\""));

        match op {
            "capacity" => {
                assert_eq!(Some(live.capacity() as u64), expect.as_u64(), "{context}");
                assert_eq!(S::CAPACITY, live.capacity(), "{context}");
            }
            "push" => {
                let value = field(step, "value", &context)
                    .as_u64()
                    .unwrap_or_else(|| panic!("{context}: \"value\" is not an unsigned integer"));
                let element = Tracked {
                    value,
                    destroyed: destroyed.clone(),
                };
                match (live.push(element), expect.as_str()) {
                    (Ok(()), Some("ok")) => {}
                    (Err(PushError::Full(back)), Some("full")) => {
                        assert_eq!(
                            back.value, value,
                            "{context}: a refused push hands back its own value"
                        );
                    }
                    (Ok(()), _) => panic!("{context}: push succeeded, expected {expect}"),
                    (Err(PushError::Full(_)), _) => {
                        panic!("{context}: push was refused, expected {expect}")
                    }
                    (Err(PushError::OutOfMemory(_)), _) => {
                        panic!("{context}: a bounded queue reported OutOfMemory")
                    }
                }
            }
            "pop" => {
                let popped = live.pop().map(|element| element.value);
                match expect {
                    Value::String(s) if s == "empty" => {
                        assert_eq!(popped, None, "{context}: expected the queue to be empty")
                    }
                    Value::Number(n) => assert_eq!(popped, n.as_u64(), "{context}"),
                    other => panic!("{context}: a pop cannot expect {other}"),
                }
            }
            "destroy" => {
                let mut expected: Vec<u64> = expect
                    .as_array()
                    .unwrap_or_else(|| panic!("{context}: \"destroy\" expects a list"))
                    .iter()
                    .map(|v| v.as_u64().expect("destroyed values are unsigned integers"))
                    .collect();
                destroyed.borrow_mut().clear();
                drop(queue.take());
                let mut actual = destroyed.borrow().clone();
                expected.sort_unstable();
                actual.sort_unstable();
                assert_eq!(
                    actual, expected,
                    "{context}: the queue destroyed a different set"
                );
            }
            other => panic!("{context}: unknown op \"{other}\""),
        }
    }
}

/// The capacities the fixture uses. A const generic needs its value at
/// compile time; a scenario naming another capacity stops here by name.
fn dispatch_bounded_spsc(id: &str, capacity: u64, steps: &[Value]) {
    match capacity {
        1 => run_scenario::<Spsc<Tracked, 1>>(id, steps),
        2 => run_scenario::<Spsc<Tracked, 2>>(id, steps),
        3 => run_scenario::<Spsc<Tracked, 3>>(id, steps),
        4 => run_scenario::<Spsc<Tracked, 4>>(id, steps),
        other => panic!("scenario {id}: capacity {other} is not in this arm's dispatch; add it"),
    }
}

/// The same for the SCQ row, whose ring is the capacity rounded up to a
/// power of two, so each capacity names both.
fn dispatch_bounded_scq(id: &str, capacity: u64, steps: &[Value]) {
    match capacity {
        1 => run_scenario::<Scq<Tracked, 1, 1>>(id, steps),
        2 => run_scenario::<Scq<Tracked, 2, 2>>(id, steps),
        3 => run_scenario::<Scq<Tracked, 3, 4>>(id, steps),
        4 => run_scenario::<Scq<Tracked, 4, 4>>(id, steps),
        5 => run_scenario::<Scq<Tracked, 5, 8>>(id, steps),
        other => panic!("scenario {id}: capacity {other} is not in this arm's dispatch; add it"),
    }
}

#[test]
fn every_contract_scenario_holds() {
    let contract = contract();
    assert_eq!(
        contract["version"], 1,
        "this arm reads version 1 of the contract"
    );
    let scenarios = contract["scenarios"]
        .as_array()
        .expect("\"scenarios\" is a list");
    assert!(
        !scenarios.is_empty(),
        "a contract with no scenarios checks nothing"
    );

    for scenario in scenarios {
        let id = field(scenario, "id", "scenario")
            .as_str()
            .expect("\"id\" is a string");
        let row = (
            field(scenario, "storage", id).as_str(),
            field(scenario, "producers", id).as_str(),
            field(scenario, "consumers", id).as_str(),
        );
        let steps = field(scenario, "steps", id)
            .as_array()
            .expect("\"steps\" is a list");
        let capacity = field(scenario, "capacity", id)
            .as_u64()
            .expect("\"capacity\" is an integer");
        match row {
            (Some("bounded"), Some("one"), Some("one")) => {
                dispatch_bounded_spsc(id, capacity, steps)
            }
            // Any other cardinality selects the SCQ row (the RFC's selection
            // table), so the three combinations share one runtime.
            (Some("bounded"), Some("many"), Some("many" | "one"))
            | (Some("bounded"), Some("one"), Some("many")) => {
                dispatch_bounded_scq(id, capacity, steps)
            }
            other => panic!("scenario {id}: the Rust arm has no runtime for the row {other:?}"),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Layer 2 — the checker, against histories built by hand
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn push(value: u64, outcome: Outcome, invoked: u64, returned: u64) -> Operation {
    Operation {
        call: Call::Push(value),
        outcome,
        invoked,
        returned,
    }
}

fn pop(outcome: Outcome, invoked: u64, returned: u64) -> Operation {
    Operation {
        call: Call::Pop,
        outcome,
        invoked,
        returned,
    }
}

fn verdict(capacity: usize, participants: Vec<Vec<Operation>>) -> Verdict {
    verdict_under(Refusal::AtCapacity, capacity, participants)
}

fn verdict_under(refusal: Refusal, capacity: usize, participants: Vec<Vec<Operation>>) -> Verdict {
    check(&History {
        capacity,
        refusal,
        participants,
    })
}

fn is_refused(v: &Verdict) -> bool {
    matches!(v, Verdict::NotLinearizable { .. })
}

#[test]
fn the_checker_accepts_a_sequential_fifo_run() {
    let v = verdict(
        2,
        vec![vec![
            push(1, Outcome::Pushed, 1, 2),
            push(2, Outcome::Pushed, 3, 4),
            push(3, Outcome::Full, 5, 6),
            pop(Outcome::Popped(1), 7, 8),
            pop(Outcome::Popped(2), 9, 10),
            pop(Outcome::Empty, 11, 12),
        ]],
    );
    assert_eq!(v, Verdict::Linearizable);
}

#[test]
fn the_checker_refuses_an_order_the_fifo_cannot_give() {
    let v = verdict(
        4,
        vec![
            vec![
                push(1, Outcome::Pushed, 1, 2),
                push(2, Outcome::Pushed, 3, 4),
            ],
            vec![pop(Outcome::Popped(2), 5, 6), pop(Outcome::Popped(1), 7, 8)],
        ],
    );
    assert!(is_refused(&v), "{v:?}");
}

#[test]
fn the_checker_lets_overlapping_operations_take_either_order() {
    // The pop overlaps the push, so it may land before it (empty) ...
    let empty = verdict(
        1,
        vec![
            vec![push(7, Outcome::Pushed, 1, 4)],
            vec![pop(Outcome::Empty, 2, 3)],
        ],
    );
    assert_eq!(empty, Verdict::Linearizable);
    // ... or after it (the value).
    let value = verdict(
        1,
        vec![
            vec![push(7, Outcome::Pushed, 1, 4)],
            vec![pop(Outcome::Popped(7), 2, 3)],
        ],
    );
    assert_eq!(value, Verdict::Linearizable);
}

#[test]
fn the_checker_refuses_empty_after_a_completed_push() {
    let v = verdict(
        1,
        vec![
            vec![push(7, Outcome::Pushed, 1, 2)],
            vec![pop(Outcome::Empty, 3, 4)],
        ],
    );
    assert!(is_refused(&v), "{v:?}");
}

#[test]
fn the_checker_refuses_full_below_capacity_and_a_push_above_it() {
    let early_full = verdict(
        2,
        vec![vec![
            push(1, Outcome::Pushed, 1, 2),
            push(2, Outcome::Full, 3, 4),
        ]],
    );
    assert!(is_refused(&early_full), "{early_full:?}");
    let overfull = verdict(
        1,
        vec![vec![
            push(1, Outcome::Pushed, 1, 2),
            push(2, Outcome::Pushed, 3, 4),
        ]],
    );
    assert!(is_refused(&overfull), "{overfull:?}");
}

#[test]
fn the_checker_refuses_a_value_nobody_pushed() {
    let v = verdict(
        2,
        vec![
            vec![push(1, Outcome::Pushed, 1, 2)],
            vec![pop(Outcome::Popped(9), 3, 4)],
        ],
    );
    assert!(is_refused(&v), "{v:?}");
}

#[test]
fn the_checker_refuses_a_history_no_run_can_produce() {
    let backwards = verdict(1, vec![vec![push(1, Outcome::Pushed, 5, 4)]]);
    assert!(matches!(backwards, Verdict::Malformed(_)), "{backwards:?}");
    let overlapping = verdict(
        1,
        vec![vec![
            push(1, Outcome::Pushed, 1, 4),
            pop(Outcome::Popped(1), 3, 6),
        ]],
    );
    assert!(
        matches!(overlapping, Verdict::Malformed(_)),
        "{overlapping:?}"
    );
    let mismatched = verdict(1, vec![vec![pop(Outcome::Pushed, 1, 2)]]);
    assert!(
        matches!(mismatched, Verdict::Malformed(_)),
        "{mismatched:?}"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Layer 2 — real runs of the one-producer, one-consumer queue
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// How many values each recorded run pushes through the queue.
const VALUES_PER_RUN: u64 = 2_000;

/// A recorded history, sent through the JSON form every backend writes its
/// runs in (`tests/forge/conformance/queue_history.schema.json`) and read
/// back, so what is judged is what a file carries and the writer and the
/// reader are held to real histories and not only to the hand-written ones.
///
/// When `SCE_QUEUE_HISTORY_DIR` names a directory the text is also written
/// there as `<name>.json`, which is how the Rust gate has the shared command,
/// `sce-codegen check-queue-history`, judge this arm's runs as it judges the
/// other backends'. A name is a shape, so a shape recorded many times leaves
/// its last recording.
fn through_the_wire(history: History, name: &str) -> History {
    let text = history.to_json();
    let read = History::from_json(&text)
        .unwrap_or_else(|e| panic!("{name}: the writer's own output does not read: {e}"));
    assert_eq!(
        read.to_json(),
        text,
        "{name}: writing what was read gives the same text"
    );
    if let Some(dir) = std::env::var_os("SCE_QUEUE_HISTORY_DIR") {
        let path = std::path::Path::new(&dir).join(format!("{name}.json"));
        std::fs::write(&path, &text)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
    read
}

/// Run one producer and one consumer on two threads, recording every
/// attempt either side made — the refused pushes and the empty pops too,
/// because those are results the checker must account for.
///
/// The clock is one SeqCst counter both sides read before and after each
/// call. A read-modify-write chain on one atomic orders the readings with
/// the calls between them, so "returned before invoked" in the record means
/// it in the run.
fn record_spsc_run<const N: usize>() -> History {
    let mut queue = Spsc::<u64, N>::new();
    let clock = AtomicU64::new(1);
    let clock = &clock;
    let tick = move || clock.fetch_add(1, Ordering::SeqCst);
    let (mut producer, mut consumer) = queue.split();

    let (pushes, pops) = thread::scope(|scope| {
        let producing = scope.spawn(move || {
            let mut ops = Vec::new();
            for value in 1..=VALUES_PER_RUN {
                loop {
                    let invoked = tick();
                    let result = producer.try_push(value);
                    let returned = tick();
                    match result {
                        Ok(()) => {
                            ops.push(push(value, Outcome::Pushed, invoked, returned));
                            break;
                        }
                        Err(PushError::Full(_)) => {
                            ops.push(push(value, Outcome::Full, invoked, returned));
                            thread::yield_now();
                        }
                        Err(PushError::OutOfMemory(_)) => {
                            panic!("a bounded queue reported OutOfMemory")
                        }
                    }
                }
            }
            ops
        });
        let consuming = scope.spawn(move || {
            let mut ops = Vec::new();
            let mut received = 0;
            while received < VALUES_PER_RUN {
                let invoked = tick();
                let result = consumer.try_pop();
                let returned = tick();
                match result {
                    Some(value) => {
                        ops.push(pop(Outcome::Popped(value), invoked, returned));
                        received += 1;
                    }
                    None => {
                        ops.push(pop(Outcome::Empty, invoked, returned));
                        thread::yield_now();
                    }
                }
            }
            ops
        });
        (producing.join().unwrap(), consuming.join().unwrap())
    });

    History {
        capacity: N,
        refusal: Refusal::AtCapacity,
        participants: vec![pushes, pops],
    }
}

fn popped_values(history: &History) -> Vec<u64> {
    history.participants[1]
        .iter()
        .filter_map(|op| match op.outcome {
            Outcome::Popped(value) => Some(value),
            _ => None,
        })
        .collect()
}

fn assert_run_is_linearizable(history: &History) {
    let history = &through_the_wire(history.clone(), &format!("rust_spsc_n{}", history.capacity));
    assert_eq!(
        popped_values(history),
        (1..=VALUES_PER_RUN).collect::<Vec<_>>(),
        "capacity {}: every value comes out once, in order",
        history.capacity
    );
    assert_eq!(
        check(history),
        Verdict::Linearizable,
        "capacity {}",
        history.capacity
    );
}

#[test]
fn recorded_spsc_runs_are_linearizable() {
    assert_run_is_linearizable(&record_spsc_run::<1>());
    assert_run_is_linearizable(&record_spsc_run::<2>());
    assert_run_is_linearizable(&record_spsc_run::<3>());
    assert_run_is_linearizable(&record_spsc_run::<8>());
}

/// A real record with two delivered values swapped must be refused. This
/// is the check on the check: it shows the search can tell a run the queue
/// produced from one it did not, on a history of the size and shape the
/// test above judges.
#[test]
fn a_recorded_run_with_two_values_swapped_is_refused() {
    let mut history = record_spsc_run::<3>();
    let pops = &mut history.participants[1];
    let delivered: Vec<usize> = pops
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(op.outcome, Outcome::Popped(_)))
        .map(|(at, _)| at)
        .collect();
    let (first, second) = (delivered[10], delivered[11]);
    let (a, b) = (pops[first].outcome, pops[second].outcome);
    pops[first].outcome = b;
    pops[second].outcome = a;
    let v = check(&history);
    assert!(is_refused(&v), "{v:?}");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Layer 2 — real runs of the SCQ queue, many producers and many consumers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// How long a recorded run may take before it is called stuck. A queue that
/// lost an element leaves its consumers waiting for it for ever, and a test
/// that hangs is a failure nobody can read.
const STUCK_AFTER: Duration = Duration::from_secs(120);

/// Run `producers` producer threads and `consumers` consumer threads on one
/// queue, each producer pushing `values_per_producer` values of its own, and
/// record every attempt every thread made — the refused pushes and the empty
/// pops too, because those are results the checker must account for.
///
/// The clock is the one the one-producer run reads: a SeqCst counter each
/// thread reads before and after every call, so "returned before invoked" in
/// the record means it in the run. Every value is unique (a producer's values
/// run from `p * values_per_producer + 1`), which is what lets the checker
/// tell which push a pop took.
fn record_scq_run<const N: usize, const R: usize>(
    producers: usize,
    consumers: usize,
    values_per_producer: u64,
) -> History {
    let queue = Scq::<u64, N, R>::new();
    let clock = AtomicU64::new(1);
    let delivered = AtomicU64::new(0);
    let total = producers as u64 * values_per_producer;
    let started = Instant::now();
    let (queue, clock, delivered) = (&queue, &clock, &delivered);
    let tick = move || clock.fetch_add(1, Ordering::SeqCst);

    let participants = thread::scope(|scope| {
        let mut threads = Vec::new();
        for who in 0..producers {
            threads.push(scope.spawn(move || {
                let producer = queue.producer().expect("a place for every producer");
                let mut ops = Vec::new();
                for k in 1..=values_per_producer {
                    let value = who as u64 * values_per_producer + k;
                    loop {
                        let invoked = tick();
                        let result = producer.try_push(value);
                        let returned = tick();
                        match result {
                            Ok(()) => {
                                ops.push(push(value, Outcome::Pushed, invoked, returned));
                                break;
                            }
                            Err(PushError::Full(_)) => {
                                ops.push(push(value, Outcome::Full, invoked, returned));
                                assert!(started.elapsed() < STUCK_AFTER, "a producer is stuck");
                                thread::yield_now();
                            }
                            Err(PushError::OutOfMemory(_)) => {
                                panic!("a bounded queue reported OutOfMemory")
                            }
                        }
                    }
                }
                ops
            }));
        }
        for _ in 0..consumers {
            threads.push(scope.spawn(move || {
                let consumer = queue.consumer().expect("a place for every consumer");
                let mut ops = Vec::new();
                while delivered.load(Ordering::SeqCst) < total {
                    let invoked = tick();
                    let result = consumer.try_pop();
                    let returned = tick();
                    match result {
                        Some(value) => {
                            delivered.fetch_add(1, Ordering::SeqCst);
                            ops.push(pop(Outcome::Popped(value), invoked, returned));
                        }
                        None => {
                            ops.push(pop(Outcome::Empty, invoked, returned));
                            assert!(started.elapsed() < STUCK_AFTER, "a consumer is stuck");
                            thread::yield_now();
                        }
                    }
                }
                ops
            }));
        }
        threads
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });

    History {
        capacity: N,
        refusal: Refusal::WhileSlotsAreHeld,
        participants,
    }
}

/// Every value any participant popped, in no particular order.
fn every_popped_value(history: &History) -> Vec<u64> {
    history
        .participants
        .iter()
        .flatten()
        .filter_map(|op| match op.outcome {
            Outcome::Popped(value) => Some(value),
            _ => None,
        })
        .collect()
}

fn assert_scq_run_is_linearizable<const N: usize, const R: usize>(
    producers: usize,
    consumers: usize,
    values_per_producer: u64,
) {
    let history = through_the_wire(
        record_scq_run::<N, R>(producers, consumers, values_per_producer),
        &format!("rust_scq_n{N}_r{R}_p{producers}_c{consumers}"),
    );
    let shape = format!(
        "capacity {N}, {producers} producer(s), {consumers} consumer(s), \
         {values_per_producer} values each"
    );
    let mut popped = every_popped_value(&history);
    popped.sort_unstable();
    assert_eq!(
        popped,
        (1..=producers as u64 * values_per_producer).collect::<Vec<_>>(),
        "{shape}: every value comes out exactly once"
    );
    let verdict = check(&history);
    assert!(
        verdict == Verdict::Linearizable,
        "{shape}: {verdict:?}\n{}",
        describe_stall(&history, &verdict)
    );
}

/// What the search could not get past, in words: the next operation each
/// participant had left unplaced when it ran out of choices, and every other
/// operation that overlapped the first of them. A verdict that says only that
/// no sequence exists sends the reader back to re-run a test whose run is
/// gone.
fn describe_stall(history: &History, verdict: &Verdict) -> String {
    let Verdict::NotLinearizable { deepest, .. } = verdict else {
        return String::new();
    };
    let mut text = String::new();
    let mut first: Option<(usize, Operation)> = None;
    for (who, placed) in deepest.iter().enumerate() {
        match history.participants[who].get(*placed) {
            Some(op) => {
                text.push_str(&format!(
                    "  participant {who} stalled at op {placed}: {op:?}\n"
                ));
                let earlier = match first {
                    None => true,
                    Some((_, earliest)) => op.invoked < earliest.invoked,
                };
                if earlier {
                    first = Some((who, *op));
                }
            }
            None => text.push_str(&format!("  participant {who} had placed everything\n")),
        }
    }
    if let Some((who, stalled)) = first {
        text.push_str(&format!("  overlapping participant {who}'s {stalled:?}:\n"));
        for (other, ops) in history.participants.iter().enumerate() {
            for (at, op) in ops.iter().enumerate() {
                if other != who && op.invoked < stalled.returned && stalled.invoked < op.returned {
                    text.push_str(&format!("    participant {other} op {at}: {op:?}\n"));
                }
            }
        }
    }
    text
}

/// How many times each shape is recorded. One recording is a draw from the
/// schedules the machine happens to give, and the schedules that matter (a
/// pop stopped between taking its element and handing its slot back) are a
/// fraction of a percent of them, so the shapes are drawn many times: a
/// thousand-odd recordings take about two seconds.
const RECORDINGS_PER_SHAPE: usize = 150;

#[test]
fn recorded_scq_runs_are_linearizable() {
    for _ in 0..RECORDINGS_PER_SHAPE {
        // A ring is at least as large as the number of participants working
        // it, so the small capacities ride on rings sized for their threads.
        assert_scq_run_is_linearizable::<1, 2>(2, 2, 150);
        assert_scq_run_is_linearizable::<3, 4>(2, 2, 150);
        assert_scq_run_is_linearizable::<8, 8>(2, 2, 150);
        assert_scq_run_is_linearizable::<2, 4>(3, 1, 120);
        assert_scq_run_is_linearizable::<4, 4>(1, 3, 120);
        assert_scq_run_is_linearizable::<5, 8>(2, 2, 150);
    }
}

// The refusal the SCQ rows are held to (`Refusal::WhileSlotsAreHeld`), and the
// limits that keep it from excusing a queue that is simply too small. The
// three histories below share one shape: capacity 2, one slow pop that is
// still in flight while a fast pop and a push complete behind it.

/// Capacity 2. `P0` fills it with 1 and 2; `C0`'s pop of 1 starts at 5 and
/// does not return until `slow_pop_returns`; `C1` pops 2 and returns at 9, so
/// by FIFO both pops precede everything after 9; `P0` pushes 3 and returns at
/// 11; `P1`'s push of 4 runs over 12 to 13 and is refused. The sequential
/// queue holds 1 element there, not 2.
fn a_refused_push_behind_a_slow_pop(slow_pop_returns: u64) -> Vec<Vec<Operation>> {
    vec![
        vec![
            push(1, Outcome::Pushed, 1, 2),
            push(2, Outcome::Pushed, 3, 4),
            push(3, Outcome::Pushed, 10, 11),
        ],
        vec![pop(Outcome::Popped(1), 5, slow_pop_returns)],
        vec![pop(Outcome::Popped(2), 6, 9)],
        vec![push(4, Outcome::Full, 12, 13)],
    ]
}

#[test]
fn a_slot_held_by_a_slow_pop_explains_a_refusal_only_under_the_relaxed_rule() {
    let held = a_refused_push_behind_a_slow_pop(20);
    assert!(
        is_refused(&verdict_under(Refusal::AtCapacity, 2, held.clone())),
        "the sequential queue holds 1 of 2 elements when the push is refused"
    );
    assert_eq!(
        verdict_under(Refusal::WhileSlotsAreHeld, 2, held),
        Verdict::Linearizable,
        "the pop of 1 is still in flight and holds the slot the push needed"
    );
}

#[test]
fn a_refusal_no_overlapping_operation_explains_is_refused_under_both_rules() {
    // The slow pop returns at 8, before the refused push is invoked at 12:
    // nothing holds a slot while it runs, so the queue was not full.
    let at_rest = a_refused_push_behind_a_slow_pop(8);
    for refusal in [Refusal::AtCapacity, Refusal::WhileSlotsAreHeld] {
        let v = verdict_under(refusal, 2, at_rest.clone());
        assert!(is_refused(&v), "{refusal:?}: {v:?}");
    }
}

#[test]
fn one_participant_holds_one_slot_however_many_operations_it_overlaps_with() {
    // Capacity 3, one element left when the push is refused, so two slots
    // would have to be held. `C0` is the only other participant running
    // alongside it, with two operations that both overlap the refused push,
    // but a participant is in one operation at a time and holds one slot.
    let v = verdict_under(
        Refusal::WhileSlotsAreHeld,
        3,
        vec![
            vec![
                push(1, Outcome::Pushed, 1, 2),
                push(2, Outcome::Pushed, 3, 4),
                push(3, Outcome::Pushed, 5, 6),
            ],
            vec![
                pop(Outcome::Popped(1), 7, 8),
                pop(Outcome::Popped(2), 9, 10),
            ],
            vec![pop(Outcome::Popped(3), 11, 13), pop(Outcome::Empty, 14, 16)],
            vec![push(4, Outcome::Full, 12, 15)],
        ],
    );
    assert!(is_refused(&v), "{v:?}");
}

/// A real record with a value nobody pushed in place of one that was popped
/// must be refused. The check on the check, for the many-participant search:
/// it shows the search can tell a run the queue produced from one it did not,
/// on a history of the shape the test above judges.
#[test]
fn a_recorded_scq_run_with_a_foreign_value_is_refused() {
    let mut history = record_scq_run::<3, 4>(2, 2, 40);
    let foreign = 1_000_000;
    let corrupted = history
        .participants
        .iter_mut()
        .flatten()
        .filter(|op| matches!(op.outcome, Outcome::Popped(_)))
        .nth(30)
        .expect("a run of 80 values pops at least 31");
    corrupted.outcome = Outcome::Popped(foreign);
    let v = check(&history);
    assert!(is_refused(&v), "{v:?}");
}
