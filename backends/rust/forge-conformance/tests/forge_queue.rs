// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust arm of the `queue` kind's verification (SCE Protocol-Synthesis
//! RFC §synth-5-P), layers 1 and 2:
//!
//! - **Contract scenarios.** Every scenario in
//!   `tests/forge/conformance/queue_contract.json` runs against the runtime
//!   queue its storage row names. A row this arm has no runtime for is
//!   refused by name, never skipped.
//! - **Linearizability.** Real two-thread runs of the one-producer,
//!   one-consumer queue record what each side observed, and
//!   `sce_forge_conformance::queue_history::check` judges the record.
//!
//! The checker is tested here as well, against histories built by hand,
//! because a checker that accepted everything would make the second layer
//! pass whatever the queue did.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use sce_forge_conformance::queue_history::{check, Call, History, Operation, Outcome, Verdict};
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

fn run_bounded_spsc<const N: usize>(id: &str, steps: &[Value]) {
    let destroyed = Rc::new(RefCell::new(Vec::new()));
    let mut queue = Some(Spsc::<Tracked, N>::new());

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
                assert_eq!(Spsc::<Tracked, N>::CAPACITY, live.capacity(), "{context}");
            }
            "push" => {
                let value = field(step, "value", &context)
                    .as_u64()
                    .unwrap_or_else(|| panic!("{context}: \"value\" is not an unsigned integer"));
                let (mut producer, _consumer) = live.split();
                let element = Tracked {
                    value,
                    destroyed: destroyed.clone(),
                };
                match (producer.try_push(element), expect.as_str()) {
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
                let (_producer, mut consumer) = live.split();
                let popped = consumer.try_pop().map(|element| element.value);
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
        1 => run_bounded_spsc::<1>(id, steps),
        2 => run_bounded_spsc::<2>(id, steps),
        3 => run_bounded_spsc::<3>(id, steps),
        4 => run_bounded_spsc::<4>(id, steps),
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
    check(&History {
        capacity,
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
