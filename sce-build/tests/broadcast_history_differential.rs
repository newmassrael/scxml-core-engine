// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The specialised broadcast-log checker held equal to the general search
//! (ADR 0062 section 5).
//!
//! The general search is the definition of what is linearizable; the
//! specialised checker is admitted only while it gives the same verdict class
//! (linearizable, not linearizable, malformed) on every history, and the proof
//! in `sce-build/src/broadcast_history/specialised.rs` says why it does. This
//! file is the other half: what a test can show of the proof's code. It tries
//!
//! - histories produced by simulating the sequential log under random
//!   interleavings with random interval widths, which are linearizable by
//!   construction;
//! - the same histories after one corrupting change each, of eleven kinds,
//!   among them a wrong item value, a cursor, a lag count, an interval put
//!   at the instant another operation returned or was invoked, and a publish
//!   duplicated;
//! - histories shrunk one operation at a time by a delta-debugging shrinker,
//!   with the two checkers compared on every intermediate history, and a
//!   deliberately wrong checker to show that a disagreement is found and shrunk;
//! - histories of thousands of operations, which only the specialised checker
//!   could finish;
//! - every fixture history, through both checkers and through the command
//!   under each `--method`.
//!
//! What it does not show is what the generator does not write: it writes
//! histories of at most fifteen operations and at most three readers, a
//! retention of 1 to 4, and intervals that overlap their neighbours by a few
//! steps of a clock that moves four per operation. The proof covers the rest.
//!
//! Everything is deterministic: one seeded generator, written below, and no
//! clock.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::broadcast_history::{
    check, check_general, check_specialised, judge, Ask, Call, Decision, Declined, History, Method,
    Obstruction, OpenFrom, Operation, Outcome,
};
use sce_build::history_common::Verdict;

// ───────────────────────────────── the generator ─────────────────────────────

/// SplitMix64 (Steele, Lea and Flood, "Fast splittable pseudorandom number
/// generators", OOPSLA 2014): a state that moves by an odd constant and a
/// finaliser. Any seed is a good seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`; the bound is not zero.
    fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "a bound of nothing leaves no number to draw");
        self.next_u64() % bound
    }

    /// True with probability `numerator / denominator`.
    fn chance(&mut self, numerator: u64, denominator: u64) -> bool {
        self.below(denominator) < numerator
    }
}

/// What a simulated history is like.
#[derive(Debug, Clone, Copy)]
struct Shape {
    retention: u64,
    readers: usize,
    operations: usize,
    /// How far an interval may reach on each side of the instant its
    /// operation took effect. The clock moves 4 per operation, so a spread of
    /// 24 lets an interval overlap about six neighbours.
    spread: u64,
    /// How many readers' worth of turns the publisher gets: 1 is the same as
    /// one reader, 6 is most of the operations.
    publisher_weight: u64,
}

/// An open at a random position, with the cursor the sequential log gives it
/// when `h` entries are published, and that cursor.
fn open_at(rng: &mut Rng, h: u64, retention: u64) -> (Call, Outcome, u64) {
    let from = match rng.below(3) {
        0 => OpenFrom::Oldest,
        1 => OpenFrom::Head,
        _ => OpenFrom::At(rng.below(h + 3)),
    };
    let cursor = match from {
        OpenFrom::Oldest => h.saturating_sub(retention),
        OpenFrom::Head => h,
        OpenFrom::At(n) => n.min(h),
    };
    (Call::Open(from), Outcome::Opened(cursor), cursor)
}

/// What the sequential log of section 2 gives a read at `cursor`, and the
/// cursor after it. The cursor is never past `h`.
fn read_at(entries: &[u64], retention: u64, cursor: u64) -> (Outcome, u64) {
    let h = entries.len() as u64;
    if h > retention && cursor < h - retention {
        let start = h - retention;
        (
            Outcome::Lagged {
                skipped: start - cursor,
            },
            start,
        )
    } else if cursor == h {
        (Outcome::Empty, cursor)
    } else {
        (
            Outcome::Item {
                seq: cursor,
                value: entries[cursor as usize],
            },
            cursor + 1,
        )
    }
}

/// Run the sequential log through `shape.operations` operations in one random
/// order, each by a random participant, and give each operation an interval
/// that contains the instant it took effect.
///
/// Operation number `step` takes effect at `100 + 4 * step`. Its interval
/// reaches `before` to the left of that instant (0 to `spread`) and `after`
/// to the right (1 to `spread`), so it is invoked at or before and returned
/// after. A participant's own intervals are then pulled apart at the midpoint
/// between their instants, which are an even distance apart, so that a
/// participant's next operation is invoked strictly after its previous one
/// returned. An operation that precedes another by the real-time order
/// (`returned <= invoked`) took effect before it, so the order of the
/// instants is a linearization, and the history is linearizable by
/// construction.
fn simulate(rng: &mut Rng, shape: Shape) -> History {
    let mut entries: Vec<u64> = Vec::new();
    let mut cursors: Vec<Option<u64>> = vec![None; shape.readers];
    let mut lists: Vec<Vec<(u64, Operation)>> = vec![Vec::new(); shape.readers + 1];
    for step in 0..shape.operations {
        let turn = rng.below(shape.publisher_weight + shape.readers as u64);
        let who = if turn < shape.publisher_weight {
            0
        } else {
            1 + (turn - shape.publisher_weight) as usize
        };
        let h = entries.len() as u64;
        let (call, outcome) = if who == 0 {
            if rng.chance(1, 8) {
                (Call::Head, Outcome::Head(h))
            } else {
                let value = (h + 1) * 11;
                entries.push(value);
                (Call::Publish(value), Outcome::Published(h))
            }
        } else {
            let reader = who - 1;
            let roll = rng.below(10);
            let held = cursors[reader];
            match held {
                Some(cursor) if roll >= 2 => {
                    let (outcome, next) = read_at(&entries, shape.retention, cursor);
                    cursors[reader] = Some(next);
                    (Call::Read, outcome)
                }
                Some(_) if roll == 1 => (Call::Head, Outcome::Head(h)),
                _ => {
                    let (call, outcome, cursor) = open_at(rng, h, shape.retention);
                    cursors[reader] = Some(cursor);
                    (call, outcome)
                }
            }
        };
        let effect = 100 + 4 * step as u64;
        let before = rng.below(shape.spread + 1);
        let after = 1 + rng.below(shape.spread);
        lists[who].push((
            effect,
            Operation {
                call,
                outcome,
                invoked: effect - before,
                returned: effect + after,
            },
        ));
    }
    for list in &mut lists {
        let limits: Vec<u64> = list
            .windows(2)
            .map(|pair| pair[0].0 + (pair[1].0 - pair[0].0) / 2)
            .collect();
        for (at, limit) in limits.iter().enumerate() {
            list[at].1.returned = list[at].1.returned.min(*limit);
            list[at + 1].1.invoked = list[at + 1].1.invoked.max(limit + 1);
        }
    }
    let mut lists = lists.into_iter().map(|list| {
        list.into_iter()
            .map(|(_, operation)| operation)
            .collect::<Vec<_>>()
    });
    let publisher = lists.next().expect("the publisher's list");
    History {
        retention: shape.retention,
        publisher,
        readers: lists.collect(),
    }
}

// ───────────────────────────────── the verdict classes ───────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Linearizable,
    NotLinearizable,
    Malformed,
}

fn class_of(verdict: &Verdict) -> Class {
    match verdict {
        Verdict::Linearizable => Class::Linearizable,
        Verdict::NotLinearizable { .. } => Class::NotLinearizable,
        Verdict::Malformed(_) => Class::Malformed,
    }
}

/// The specialised checker's answer as a class. It declines a history that
/// is not well formed, which the general search calls malformed; any other
/// decline is a failure here, since every history this file writes is inside
/// the theorem.
fn specialised_class(decision: &Decision) -> Class {
    match decision {
        Decision::Linearizable => Class::Linearizable,
        Decision::NotLinearizable(_) => Class::NotLinearizable,
        Decision::Declined(Declined::NotWellFormed(_)) => Class::Malformed,
        Decision::Declined(other) => {
            panic!("the specialised checker declined a history it should decide: {other}")
        }
    }
}

/// The general search's class and the specialised checker's.
fn classes(history: &History) -> (Class, Class) {
    (
        class_of(&check_general(history)),
        specialised_class(&check_specialised(history)),
    )
}

// ───────────────────────────────── the history as a list ─────────────────────

fn operations_in(history: &History) -> usize {
    history.publisher.len() + history.readers.iter().map(Vec::len).sum::<usize>()
}

/// The participants' lists: the publisher is 0, reader `i` is `i + 1`.
fn lists_of(history: &History) -> Vec<&Vec<Operation>> {
    std::iter::once(&history.publisher)
        .chain(&history.readers)
        .collect()
}

fn list_mut(history: &mut History, participant: usize) -> &mut Vec<Operation> {
    if participant == 0 {
        &mut history.publisher
    } else {
        &mut history.readers[participant - 1]
    }
}

fn operation_at(history: &History, participant: usize, at: usize) -> Operation {
    lists_of(history)[participant][at]
}

/// Where, as (participant, position), the operations `wanted` accepts are.
fn positions(history: &History, wanted: impl Fn(&Operation) -> bool) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for (participant, list) in lists_of(history).into_iter().enumerate() {
        for (at, op) in list.iter().enumerate() {
            if wanted(op) {
                found.push((participant, at));
            }
        }
    }
    found
}

fn choose(rng: &mut Rng, found: &[(usize, usize)]) -> Option<(usize, usize)> {
    if found.is_empty() {
        None
    } else {
        Some(found[rng.below(found.len() as u64) as usize])
    }
}

/// The history without one operation. The publishes after a removed publish
/// take one sequence number less, so that removing a publish leaves a history
/// that is still in sequence; what the reads then say is the shrinker's
/// predicate's business.
fn without(history: &History, participant: usize, at: usize) -> History {
    let mut smaller = history.clone();
    let removed = list_mut(&mut smaller, participant).remove(at);
    if participant == 0 && matches!(removed.call, Call::Publish(_)) {
        for op in &mut smaller.publisher[at..] {
            if let Outcome::Published(seq) = &mut op.outcome {
                *seq = seq.saturating_sub(1);
            }
        }
    }
    smaller
}

/// Every history one operation smaller.
fn removals(history: &History) -> Vec<History> {
    positions(history, |_| true)
        .into_iter()
        .map(|(participant, at)| without(history, participant, at))
        .collect()
}

/// Delta debugging at its simplest (Zeller and Hildebrandt, "Simplifying and
/// Isolating Failure-Inducing Input", IEEE TSE 28(2), 2002): remove one
/// operation at a time while `keeps` still holds of what is left, until no
/// single removal is kept. The result is 1-minimal: `keeps` holds of it and of
/// no history one operation smaller. `keeps` holds of `history` to begin with.
fn shrink(history: &History, keeps: &dyn Fn(&History) -> bool) -> History {
    let mut current = history.clone();
    'again: loop {
        for (participant, at) in positions(&current, |_| true) {
            let candidate = without(&current, participant, at);
            if keeps(&candidate) {
                current = candidate;
                continue 'again;
            }
        }
        return current;
    }
}

/// Both checkers on `history`, which they must give the same class; when they
/// do not, the smallest history that shows it is what the failure reports.
/// Also: `check` gives that class, a malformed history is declined with the
/// message the general search gives, and what the specialised checker says it
/// could not place is an operation of the history, and not a publish.
fn assert_agreement(history: &History, what: &str) -> Class {
    let verdict = check_general(history);
    let decision = check_specialised(history);
    let general = class_of(&verdict);
    let specialised = specialised_class(&decision);
    if general != specialised {
        let minimal = shrink(history, &|candidate: &History| {
            let (g, s) = classes(candidate);
            g != s
        });
        let (g, s) = classes(&minimal);
        panic!(
            "{what}: the general search says {general:?} and the specialised checker \
             {specialised:?}.\nThe smallest history that shows a disagreement has {} operations \
             (general {g:?}, specialised {s:?}):\n{}",
            operations_in(&minimal),
            minimal.to_json()
        );
    }
    assert_eq!(
        class_of(&check(history)),
        general,
        "{what}: check must give the general search's class\n{}",
        history.to_json()
    );
    if let (
        Verdict::Malformed(general_why),
        Decision::Declined(Declined::NotWellFormed(specialised_why)),
    ) = (&verdict, &decision)
    {
        assert_eq!(general_why, specialised_why, "{what}: the messages differ");
    }
    if let Decision::NotLinearizable(found) = &decision {
        let lists = lists_of(history);
        assert!(found.participant < lists.len(), "{what}: {found:?}");
        assert!(
            found.at < lists[found.participant].len(),
            "{what}: {found:?}"
        );
        assert_eq!(found.deepest.len(), lists.len(), "{what}: {found:?}");
        assert!(
            found.deepest[found.participant] <= found.at,
            "{what}: {found:?}"
        );
        assert!(
            !matches!(
                operation_at(history, found.participant, found.at).call,
                Call::Publish(_)
            ),
            "{what}: a publish is never what cannot be placed: {found:?}"
        );
    }
    general
}

// ───────────────────────────────── the corrupting changes ────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mutation {
    /// An item read returns another value than the entry held.
    WrongItemValue,
    /// An item read names another entry than its cursor was at.
    WrongItemSeq,
    /// An open records another cursor than the position gives.
    WrongOpenCursor,
    /// A lagged read is off by one entry from the window.
    LagOffByOne,
    /// An operation returned at or before the instant it was invoked.
    ReturnedBeforeInvoked,
    /// A read that found an entry or a lag found nothing.
    EmptyWhereNotEmpty,
    /// A read that was lagged returned the entry its cursor was at, which had
    /// left the window.
    ItemOutsideWindow,
    /// A publish appears twice, once beside itself with its own sequence
    /// number, once at the end with the next.
    PublishDuplicated,
    /// A head returned a number one off.
    WrongHead,
    /// An operation is missing.
    OperationDropped,
    /// An operation's interval is moved to another place between its
    /// neighbours, half the time with its ends on the instants of other
    /// operations, where the order of two operations changes.
    Retimed,
}

const MUTATIONS: [Mutation; 11] = [
    Mutation::WrongItemValue,
    Mutation::WrongItemSeq,
    Mutation::WrongOpenCursor,
    Mutation::LagOffByOne,
    Mutation::ReturnedBeforeInvoked,
    Mutation::EmptyWhereNotEmpty,
    Mutation::ItemOutsideWindow,
    Mutation::PublishDuplicated,
    Mutation::WrongHead,
    Mutation::OperationDropped,
    Mutation::Retimed,
];

/// The cursor a reader holds before its operation `at`, from the readings it
/// recorded, as the checkers read it.
fn cursor_before(list: &[Operation], at: usize) -> Option<u64> {
    let mut held = None;
    for op in &list[..at] {
        held = match op.outcome {
            Outcome::Opened(cursor) => Some(cursor),
            Outcome::Item { seq, .. } => Some(seq.saturating_add(1)),
            Outcome::Lagged { skipped } => held.map(|cursor: u64| cursor.saturating_add(skipped)),
            _ => held,
        };
    }
    held
}

/// `history` with one corrupting change of the kind `mutation`, or `None` when
/// the history has nowhere to make it.
fn mutate(rng: &mut Rng, history: &History, mutation: Mutation) -> Option<History> {
    let mut mutated = history.clone();
    let up = rng.chance(1, 2);
    match mutation {
        Mutation::WrongItemValue => {
            let found = positions(history, |op| matches!(op.outcome, Outcome::Item { .. }));
            let (who, at) = choose(rng, &found)?;
            if let Outcome::Item { value, .. } = &mut list_mut(&mut mutated, who)[at].outcome {
                *value = value.wrapping_add(1);
            }
        }
        Mutation::WrongItemSeq => {
            let found = positions(history, |op| matches!(op.outcome, Outcome::Item { .. }));
            let (who, at) = choose(rng, &found)?;
            if let Outcome::Item { seq, .. } = &mut list_mut(&mut mutated, who)[at].outcome {
                *seq = if up || *seq == 0 { *seq + 1 } else { *seq - 1 };
            }
        }
        Mutation::WrongOpenCursor => {
            let found = positions(history, |op| matches!(op.outcome, Outcome::Opened(_)));
            let (who, at) = choose(rng, &found)?;
            if let Outcome::Opened(cursor) = &mut list_mut(&mut mutated, who)[at].outcome {
                *cursor = if up || *cursor == 0 {
                    *cursor + 1
                } else {
                    *cursor - 1
                };
            }
        }
        Mutation::LagOffByOne => {
            let found = positions(history, |op| matches!(op.outcome, Outcome::Lagged { .. }));
            let (who, at) = choose(rng, &found)?;
            if let Outcome::Lagged { skipped } = &mut list_mut(&mut mutated, who)[at].outcome {
                *skipped = if up {
                    *skipped + 1
                } else {
                    skipped.saturating_sub(1)
                };
            }
        }
        Mutation::ReturnedBeforeInvoked => {
            let (who, at) = choose(rng, &positions(history, |_| true))?;
            let op = &mut list_mut(&mut mutated, who)[at];
            op.returned = if up {
                op.invoked
            } else {
                op.invoked.saturating_sub(1)
            };
        }
        Mutation::EmptyWhereNotEmpty => {
            let found = positions(history, |op| {
                matches!(op.outcome, Outcome::Item { .. } | Outcome::Lagged { .. })
            });
            let (who, at) = choose(rng, &found)?;
            list_mut(&mut mutated, who)[at].outcome = Outcome::Empty;
        }
        Mutation::ItemOutsideWindow => {
            let found = positions(history, |op| matches!(op.outcome, Outcome::Lagged { .. }));
            let (who, at) = choose(rng, &found)?;
            let cursor = cursor_before(&history.readers[who - 1], at)?;
            let values: Vec<u64> = history
                .publisher
                .iter()
                .filter_map(|op| match op.call {
                    Call::Publish(value) => Some(value),
                    _ => None,
                })
                .collect();
            let value = *values.get(usize::try_from(cursor).ok()?)?;
            list_mut(&mut mutated, who)[at].outcome = Outcome::Item { seq: cursor, value };
        }
        Mutation::PublishDuplicated => {
            let found = positions(history, |op| matches!(op.call, Call::Publish(_)));
            let (_, at) = choose(rng, &found)?;
            let original = history.publisher[at];
            if up {
                let last = history.publisher.last()?.returned;
                mutated.publisher.push(Operation {
                    outcome: Outcome::Published(found.len() as u64),
                    invoked: last + 1,
                    returned: last + 2,
                    ..original
                });
            } else {
                mutated.publisher.insert(
                    at + 1,
                    Operation {
                        invoked: original.returned + 1,
                        returned: original.returned + 2,
                        ..original
                    },
                );
            }
        }
        Mutation::WrongHead => {
            let found = positions(history, |op| matches!(op.outcome, Outcome::Head(_)));
            let (who, at) = choose(rng, &found)?;
            if let Outcome::Head(seen) = &mut list_mut(&mut mutated, who)[at].outcome {
                *seen = if up || *seen == 0 {
                    *seen + 1
                } else {
                    *seen - 1
                };
            }
        }
        Mutation::OperationDropped => {
            let (who, at) = choose(rng, &positions(history, |_| true))?;
            mutated = without(history, who, at);
        }
        Mutation::Retimed => {
            let all = positions(history, |_| true);
            let (who, at) = choose(rng, &all)?;
            let list = lists_of(history)[who];
            let low = if at == 0 {
                1
            } else {
                list[at - 1].returned + 1
            };
            let ceiling = lists_of(history)
                .into_iter()
                .flatten()
                .map(|op| op.returned)
                .max()?;
            let high = list
                .get(at + 1)
                .map_or(ceiling + 10, |next| next.invoked - 1);
            if low >= high {
                return None;
            }
            let others: Vec<Operation> = all
                .into_iter()
                .filter(|&position| position != (who, at))
                .map(|(other, place)| operation_at(history, other, place))
                .collect();
            let mut invoked = low + rng.below(high - low);
            if rng.chance(1, 2) {
                let touching: Vec<u64> = others
                    .iter()
                    .map(|op| op.returned)
                    .filter(|instant| (low..high).contains(instant))
                    .collect();
                if !touching.is_empty() {
                    invoked = touching[rng.below(touching.len() as u64) as usize];
                }
            }
            let mut returned = invoked + 1 + rng.below(high - invoked);
            if rng.chance(1, 2) {
                let touching: Vec<u64> = others
                    .iter()
                    .map(|op| op.invoked)
                    .filter(|instant| (invoked + 1..=high).contains(instant))
                    .collect();
                if !touching.is_empty() {
                    returned = touching[rng.below(touching.len() as u64) as usize];
                }
            }
            let op = &mut list_mut(&mut mutated, who)[at];
            op.invoked = invoked;
            op.returned = returned;
        }
    }
    Some(mutated)
}

/// A simulated history and the same with one change of kind `mutation`; the
/// base is drawn again until the change has somewhere to be made.
fn mutated_case(rng: &mut Rng, shape: Shape, mutation: Mutation) -> (History, History) {
    for _ in 0..4000 {
        let base = simulate(rng, shape);
        if let Some(mutated) = mutate(rng, &base, mutation) {
            return (base, mutated);
        }
    }
    panic!("{mutation:?} found nowhere to apply in 4000 histories of {shape:?}");
}

/// The shape of the `case`-th history of a run: the retention, the readers,
/// the spread and the publisher's weight each cycle at their own pace, so
/// that every combination of retention and readers comes up in each twelve.
fn shape_of(rng: &mut Rng, case: usize, operations: usize) -> Shape {
    Shape {
        retention: 1 + (case % 4) as u64,
        readers: 1 + (case / 4) % 3,
        operations,
        spread: [2, 6, 12, 24][(case / 12) % 4],
        publisher_weight: [1, 3, 6][(case / 48) % 3] + rng.below(2),
    }
}

const CASES_PER_KIND: usize = 250;

// ───────────────────────────────── the tests ─────────────────────────────────

/// What the generated histories contain, so that agreement on them is not
/// agreement on a corner of the space.
#[derive(Default)]
struct Coverage {
    items: usize,
    empties: usize,
    laggeds: usize,
    reader_heads: usize,
    publisher_heads: usize,
    reopened: usize,
    touching: usize,
}

impl Coverage {
    fn note(&mut self, history: &History) {
        let any = |wanted: &dyn Fn(&Operation) -> bool| !positions(history, wanted).is_empty();
        self.items += usize::from(any(&|op| matches!(op.outcome, Outcome::Item { .. })));
        self.empties += usize::from(any(&|op| op.outcome == Outcome::Empty));
        self.laggeds += usize::from(any(&|op| matches!(op.outcome, Outcome::Lagged { .. })));
        self.publisher_heads += usize::from(
            history
                .publisher
                .iter()
                .any(|op| matches!(op.call, Call::Head)),
        );
        self.reader_heads += usize::from(
            history
                .readers
                .iter()
                .flatten()
                .any(|op| matches!(op.call, Call::Head)),
        );
        self.reopened += usize::from(history.readers.iter().any(|list| {
            list.iter()
                .filter(|op| matches!(op.call, Call::Open(_)))
                .count()
                > 1
        }));
        let lists = lists_of(history);
        self.touching += usize::from(lists.iter().enumerate().any(|(i, a)| {
            lists
                .iter()
                .enumerate()
                .any(|(j, b)| i != j && a.iter().any(|x| b.iter().any(|y| x.returned == y.invoked)))
        }));
    }
}

/// A log simulated under one random interleaving is linearizable by
/// construction, and both checkers say so.
///
/// 4 retentions and 3 readers make 12 configurations, 250 histories each:
/// 12 * 250 = 3000 histories of 6 to 14 operations.
#[test]
fn a_log_simulated_under_random_interleavings_is_linearizable_to_both() {
    let mut coverage = Coverage::default();
    let mut cases = 0usize;
    for retention in 1..=4u64 {
        for readers in 1..=3usize {
            let mut rng = Rng::new(0xA11CE ^ (retention << 8) ^ readers as u64);
            for case in 0..CASES_PER_KIND {
                let shape = Shape {
                    retention,
                    readers,
                    operations: 6 + rng.below(9) as usize,
                    spread: [2, 6, 12, 24][case % 4],
                    publisher_weight: [1, 3, 6][(case / 4) % 3],
                };
                let history = simulate(&mut rng, shape);
                coverage.note(&history);
                let class = assert_agreement(&history, &format!("{shape:?}"));
                assert_eq!(
                    class,
                    Class::Linearizable,
                    "{shape:?}: a log run in one order is linearizable by construction\n{}",
                    history.to_json()
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 12 * CASES_PER_KIND);
    assert!(coverage.items > 0, "no history held an item read");
    assert!(coverage.empties > 0, "no history held an empty read");
    assert!(coverage.laggeds > 0, "no history held a lagged read");
    assert!(coverage.reader_heads > 0, "no reader read the head");
    assert!(
        coverage.publisher_heads > 0,
        "the publisher never read the head"
    );
    assert!(coverage.reopened > 0, "no reader opened twice");
    assert!(
        coverage.touching > 0,
        "no history had one operation return at the instant another was invoked"
    );
}

/// One corrupting change, of each of the eleven kinds, and the two checkers
/// give the same class to the result, whatever it is.
///
/// 11 kinds of 250 changes each: 11 * 250 = 2750 histories of 8 to 15
/// operations. Two kinds have a class that cannot vary: a wrong item value
/// is never linearizable (no entry holds it) and a return at or before the
/// invocation is always malformed, so 250 of each is the least that either
/// class has.
#[test]
fn one_corrupting_change_leaves_the_two_checkers_in_agreement() {
    let mut tally = [0usize; 3];
    for (index, mutation) in MUTATIONS.iter().enumerate() {
        let mut rng = Rng::new(0xB0B ^ ((index as u64) << 16));
        for case in 0..CASES_PER_KIND {
            let operations = 8 + rng.below(7) as usize;
            let shape = shape_of(&mut rng, case, operations);
            let (_, mutated) = mutated_case(&mut rng, shape, *mutation);
            let class = assert_agreement(&mutated, &format!("{mutation:?} on {shape:?}"));
            match mutation {
                Mutation::WrongItemValue => assert_eq!(
                    class,
                    Class::NotLinearizable,
                    "an item no entry holds\n{}",
                    mutated.to_json()
                ),
                Mutation::ReturnedBeforeInvoked => assert_eq!(
                    class,
                    Class::Malformed,
                    "a return at or before the invocation\n{}",
                    mutated.to_json()
                ),
                _ => {}
            }
            tally[class as usize] += 1;
        }
    }
    assert_eq!(
        tally.iter().sum::<usize>(),
        MUTATIONS.len() * CASES_PER_KIND
    );
    assert!(
        tally[Class::NotLinearizable as usize] >= CASES_PER_KIND,
        "{tally:?}"
    );
    assert!(
        tally[Class::Malformed as usize] >= CASES_PER_KIND,
        "{tally:?}"
    );
}

/// The shrinker, on a predicate that looks at one operation: it removes every
/// other operation, and what it returns is 1-minimal.
#[test]
fn the_shrinker_reaches_a_history_no_single_removal_keeps() {
    let has_empty =
        |history: &History| !positions(history, |op| op.outcome == Outcome::Empty).is_empty();
    let mut rng = Rng::new(0x5EED);
    let mut shrunk = 0usize;
    for _ in 0..2000 {
        if shrunk == 40 {
            break;
        }
        let shape = Shape {
            retention: 2,
            readers: 2,
            operations: 14,
            spread: 6,
            publisher_weight: 2,
        };
        let history = simulate(&mut rng, shape);
        if !has_empty(&history) {
            continue;
        }
        let minimal = shrink(&history, &has_empty);
        assert!(has_empty(&minimal));
        assert_eq!(
            operations_in(&minimal),
            1,
            "only the empty read is needed\n{}",
            minimal.to_json()
        );
        assert!(removals(&minimal).iter().all(|smaller| !has_empty(smaller)));
        shrunk += 1;
    }
    assert_eq!(shrunk, 40, "too few simulated histories held an empty read");
}

/// The mutated corpus, shrunk to minimal histories that the general search
/// refuses, with the two checkers compared on every history met on the way
/// (the predicate is `assert_agreement`), and on the minimal one.
///
/// 150 histories of 8 to 15 operations that the general search finds not
/// linearizable.
#[test]
fn every_refused_history_of_the_mutated_corpus_shrinks_with_the_checkers_agreeing() {
    let wanted = 150usize;
    let mut rng = Rng::new(0xC0DE);
    let mut refused: Vec<History> = Vec::new();
    for case in 0..4000usize {
        if refused.len() == wanted {
            break;
        }
        let operations = 8 + rng.below(7) as usize;
        let shape = shape_of(&mut rng, case, operations);
        let (_, mutated) = mutated_case(&mut rng, shape, MUTATIONS[case % MUTATIONS.len()]);
        if class_of(&check_general(&mutated)) == Class::NotLinearizable {
            refused.push(mutated);
        }
    }
    assert_eq!(
        refused.len(),
        wanted,
        "too few mutated histories were refused"
    );

    let still_refused = |history: &History| {
        assert_agreement(history, "an intermediate history of a shrink") == Class::NotLinearizable
    };
    let (mut before, mut after) = (0usize, 0usize);
    for history in &refused {
        let minimal = shrink(history, &still_refused);
        assert_eq!(
            classes(&minimal),
            (Class::NotLinearizable, Class::NotLinearizable),
            "{}",
            minimal.to_json()
        );
        assert!(operations_in(&minimal) <= operations_in(history));
        assert!(
            removals(&minimal)
                .iter()
                .all(|smaller| !still_refused(smaller)),
            "not 1-minimal\n{}",
            minimal.to_json()
        );
        before += operations_in(history);
        after += operations_in(&minimal);
    }
    assert!(
        after < before,
        "the shrinker removed nothing from {wanted} histories ({before} operations)"
    );
}

/// `history` with every item read given the value its entry holds: what a
/// checker that did not compare values would see.
fn forgetting_values(history: &History) -> History {
    let values: Vec<u64> = history
        .publisher
        .iter()
        .filter_map(|op| match op.call {
            Call::Publish(value) => Some(value),
            _ => None,
        })
        .collect();
    let mut forgetful = history.clone();
    for op in forgetful.readers.iter_mut().flatten() {
        if let Outcome::Item { seq, value } = &mut op.outcome {
            if let Some(held) = usize::try_from(*seq).ok().and_then(|n| values.get(n)) {
                *value = *held;
            }
        }
    }
    forgetful
}

/// A checker that is wrong, to show the apparatus finds a disagreement and
/// shrinks it. The wrong one is the general search run on the history with
/// its item values made right, so it is blind to a wrong value; it disagrees
/// with the general search on a history that has one, and the smallest such
/// history is a publish, an open and an item.
#[test]
fn a_checker_that_ignores_item_values_is_caught_and_shrunk() {
    let disagree = |history: &History| {
        class_of(&check_general(history)) != class_of(&check_general(&forgetting_values(history)))
    };
    let mut rng = Rng::new(0xF0F0);
    let (mut before, mut after) = (0usize, 0usize);
    for case in 0..20usize {
        let operations = 8 + rng.below(7) as usize;
        let shape = shape_of(&mut rng, case, operations);
        let (_, mutated) = mutated_case(&mut rng, shape, Mutation::WrongItemValue);
        assert!(
            disagree(&mutated),
            "the blind checker should differ\n{}",
            mutated.to_json()
        );
        let minimal = shrink(&mutated, &disagree);
        assert!(disagree(&minimal));
        assert!(
            removals(&minimal).iter().all(|smaller| !disagree(smaller)),
            "not 1-minimal\n{}",
            minimal.to_json()
        );
        assert!(
            operations_in(&minimal) >= 3,
            "a publish, an open and an item are the least that holds a wrong value\n{}",
            minimal.to_json()
        );
        before += operations_in(&mutated);
        after += operations_in(&minimal);
    }
    assert!(
        after < before,
        "nothing was shrunk from {before} operations"
    );
}

/// The positions at which two histories of the same shape differ.
fn differences(a: &History, b: &History) -> Vec<(usize, usize)> {
    let (left, right) = (lists_of(a), lists_of(b));
    assert_eq!(left.len(), right.len());
    let mut found = Vec::new();
    for (participant, (x, y)) in left.iter().zip(&right).enumerate() {
        assert_eq!(x.len(), y.len());
        for (at, (p, q)) in x.iter().zip(y.iter()).enumerate() {
            if p != q {
                found.push((participant, at));
            }
        }
    }
    found
}

/// Histories of thousands of operations, which the general search could not
/// finish (its states are the combinations of how far each participant has
/// got), are decided by the specialised checker, and a single wrong value in
/// them is found and named.
#[test]
fn the_specialised_checker_decides_histories_a_search_could_not_finish() {
    for (operations, readers, retention) in
        [(2_000usize, 1usize, 1u64), (5_000, 2, 3), (10_000, 3, 4)]
    {
        let mut rng = Rng::new(operations as u64);
        let shape = Shape {
            retention,
            readers,
            operations,
            spread: 12,
            publisher_weight: 3,
        };
        let history = simulate(&mut rng, shape);
        assert_eq!(operations_in(&history), operations);
        assert_eq!(check_specialised(&history), Decision::Linearizable);
        assert_eq!(check(&history), Verdict::Linearizable);
        let automatic = judge(&history, Ask::Auto).expect("auto always decides");
        assert_eq!(automatic.method, Method::Specialised);

        let corrupt = mutate(&mut rng, &history, Mutation::WrongItemValue)
            .expect("a history of thousands of operations holds an item read");
        let differing = differences(&history, &corrupt);
        assert_eq!(differing.len(), 1);
        let Decision::NotLinearizable(found) = check_specialised(&corrupt) else {
            panic!("a value no entry holds is not linearizable");
        };
        assert_eq!(found.obstruction, Obstruction::NeverAdmitted);
        assert_eq!((found.participant, found.at), differing[0]);
        for (participant, length) in lists_of(&corrupt).iter().map(|list| list.len()).enumerate() {
            let expected = if participant == found.participant {
                found.at
            } else {
                length
            };
            assert_eq!(found.deepest[participant], expected);
        }
        assert_eq!(class_of(&check(&corrupt)), Class::NotLinearizable);

        // Whatever else is done to it, the specialised checker decides it.
        for mutation in MUTATIONS {
            let Some(other) = mutate(&mut rng, &history, mutation) else {
                continue;
            };
            assert!(
                !matches!(
                    check_specialised(&other),
                    Decision::Declined(Declined::Unmodelled { .. })
                ),
                "{mutation:?} left a history the specialised checker has no argument for"
            );
        }
    }
}

/// A history by hand: one publish at [1, 2), a reader that opens at [3, 4) and
/// reads empty at [5, 6) after the publish returned.
fn empty_after_a_publish(retention: u64) -> History {
    History {
        retention,
        publisher: vec![Operation {
            call: Call::Publish(5),
            outcome: Outcome::Published(0),
            invoked: 1,
            returned: 2,
        }],
        readers: vec![vec![
            Operation {
                call: Call::Open(OpenFrom::Oldest),
                outcome: Outcome::Opened(0),
                invoked: 3,
                returned: 4,
            },
            Operation {
                call: Call::Read,
                outcome: Outcome::Empty,
                invoked: 5,
                returned: 6,
            },
        ]],
    }
}

/// `judge` says which checker decided, and the verdict carries what each
/// method has: the general search the states it explored, the specialised
/// checker the operation it could not place.
#[test]
fn the_entry_points_say_which_method_decided() {
    let refused = empty_after_a_publish(2);

    let general = judge(&refused, Ask::General).expect("the general search decides");
    assert_eq!(general.method, Method::General);
    assert!(general.unplaceable.is_none());
    match general.verdict {
        Verdict::NotLinearizable { explored, .. } => assert!(explored >= 1, "{explored}"),
        other => panic!("expected not linearizable, got {other:?}"),
    }

    // The reader's operation 1 (the empty) cannot be placed; the publisher's
    // one operation and the reader's open can, so the first operation that
    // cannot be placed is at 1 for the publisher (its length) and at 1 for the
    // reader.
    let specialised = judge(&refused, Ask::Specialised).expect("the specialised checker decides");
    assert_eq!(specialised.method, Method::Specialised);
    assert_eq!(
        specialised.verdict,
        Verdict::NotLinearizable {
            explored: 0,
            deepest: vec![1, 1]
        }
    );
    let found = specialised
        .unplaceable
        .expect("it says what it could not place");
    assert_eq!((found.participant, found.at), (1, 1));
    assert_eq!(
        found.to_string(),
        "reader 0's operation 1 cannot be placed: its result needs between 0 and 0 entries \
         published, but at least 1 publishes returned before it was invoked and at most 1 were \
         invoked before it returned"
    );

    let automatic = judge(&refused, Ask::Auto).expect("auto always decides");
    assert_eq!(automatic.method, Method::Specialised);
    assert_eq!(automatic.verdict, specialised.verdict);
    assert_eq!(check(&refused), specialised.verdict);

    // A history that cannot have come from a run is declined by the
    // specialised checker alone and is `Malformed` under the others.
    let mut malformed = empty_after_a_publish(2);
    malformed.publisher[0].returned = malformed.publisher[0].invoked;
    let declined =
        judge(&malformed, Ask::Specialised).expect_err("it declines a malformed history");
    assert!(
        matches!(declined, Declined::NotWellFormed(_)),
        "{declined:?}"
    );
    for ask in [Ask::Auto, Ask::General] {
        let judged = judge(&malformed, ask).expect("these decide a malformed history");
        assert_eq!(judged.method, Method::General);
        assert!(matches!(judged.verdict, Verdict::Malformed(_)));
    }

    // The same history, unrefused: a read of the entry.
    let mut linearizable = empty_after_a_publish(2);
    linearizable.readers[0][1].outcome = Outcome::Item { seq: 0, value: 5 };
    for ask in [Ask::Auto, Ask::General, Ask::Specialised] {
        let judged = judge(&linearizable, ask).expect("a well-formed history is decided");
        assert_eq!(judged.verdict, Verdict::Linearizable, "{ask:?}");
        assert!(judged.unplaceable.is_none());
    }
}

// ───────────────────────────────── the fixtures ──────────────────────────────

fn histories_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/broadcast_histories")
}

fn fixtures() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(histories_dir())
        .expect("the history fixtures exist")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .expect("a UTF-8 file name")
        .to_string()
}

/// What a fixture's name says its class is, for the fixtures that are
/// histories.
fn class_named(name: &str) -> Class {
    if name.starts_with("linearizable_") || name == "records_no_operation.json" {
        Class::Linearizable
    } else if name.starts_with("not_linearizable_") {
        Class::NotLinearizable
    } else if name.starts_with("malformed_") {
        Class::Malformed
    } else {
        panic!("{name} carries no prefix that says how it is judged")
    }
}

/// Every fixture history goes through both checkers, which agree with each
/// other and with the class the fixture's name gives it (as
/// `broadcast_history_format.rs` reads the names).
#[test]
fn every_fixture_is_judged_alike_by_both_checkers() {
    let files = fixtures();
    assert!(
        files.len() >= 12,
        "the fixtures are the checker's own tests; only {} are here",
        files.len()
    );
    let mut judged = 0usize;
    for file in &files {
        let name = name_of(file);
        let text = std::fs::read_to_string(file).expect("read the fixture");
        let read = History::from_json(&text);
        if name.starts_with("not_a_history_") {
            assert!(read.is_err(), "{name} is not a history and is read as one");
            continue;
        }
        let history = read.unwrap_or_else(|e| panic!("{name} should be read: {e}"));
        let class = assert_agreement(&history, &name);
        assert_eq!(class, class_named(&name), "{name}");
        judged += 1;
    }
    assert!(judged >= 12, "only {judged} fixtures were histories");
}

fn judge_with(method: &str, files: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["check-broadcast-history", "--method", method])
        .args(files)
        .output()
        .expect("run sce-codegen check-broadcast-history")
}

/// What a fixture's name says the command prints: a phrase its refusal must
/// contain, and `None` for a history that is accepted.
fn phrase_named(name: &str) -> Option<&'static str> {
    if name.starts_with("linearizable_") {
        None
    } else if name.starts_with("not_linearizable_") {
        Some("is not linearizable")
    } else if name.starts_with("malformed_") {
        Some("cannot have come from a run")
    } else if name.starts_with("not_a_history_") {
        Some("")
    } else if name == "records_no_operation.json" {
        Some("records no operation")
    } else {
        panic!("{name} carries no prefix that says how it is judged")
    }
}

/// The command refuses and accepts what the names say under every `--method`,
/// and says its reason in the words of the method that decided: the general
/// search how many states it explored, the specialised checker which operation
/// it could not place. A malformed history is malformed under all three.
#[test]
fn the_command_judges_every_fixture_the_same_under_each_method() {
    for method in ["auto", "general", "specialised"] {
        for file in fixtures() {
            let name = name_of(&file);
            let run = judge_with(method, &[file.as_path()]);
            let stdout = String::from_utf8_lossy(&run.stdout);
            match phrase_named(&name) {
                None => {
                    assert!(
                        run.status.success(),
                        "{method}: {name} should be linearizable: {stdout}"
                    );
                    assert!(stdout.starts_with("ok   "), "{method}: {name}: {stdout}");
                }
                Some(phrase) => {
                    assert!(
                        !run.status.success(),
                        "{method}: {name} should be refused: {stdout}"
                    );
                    assert!(
                        stdout.starts_with("FAIL ") && stdout.contains(phrase),
                        "{method}: {name} should say {phrase:?}: {stdout}"
                    );
                    if name.starts_with("not_linearizable_") {
                        let how = if method == "general" {
                            "the search explored"
                        } else {
                            "cannot be placed"
                        };
                        assert!(
                            stdout.contains(how),
                            "{method}: {name} should say {how:?}: {stdout}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_method_the_command_does_not_know_is_refused() {
    let file = histories_dir().join("linearizable_sequential.json");
    let run = judge_with("quickest", &[file.as_path()]);
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!run.status.success(), "{said}");
    assert!(
        said.contains("quickest") && said.contains("specialised"),
        "the refusal should name the value and the ones there are: {said}"
    );
}
