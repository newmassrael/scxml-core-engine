// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The specialised linearizability checker of the single-producer broadcast
//! log (SCE Protocol-Synthesis RFC §synth-5-S; ADR 0062 section 5): a decision
//! procedure that does not search.
//!
//! The general search of [`super::check_general`] is the definition of what
//! linearizable means here, and in the worst case it visits every frontier of
//! the participants' operations. With one producer there is a structure it
//! does not use: the publishes are chained by program order, so the number of
//! entries published, `h`, is a step function of time, and each other
//! operation constrains only the value of `h` at the one instant it takes
//! effect. This module turns that into a theorem and a procedure of
//! `O(n log n)` for `n` operations, and says exactly when it declines.
//!
//! # Notation
//!
//! Take a *well-formed* history, one that `well_formed` accepts: a
//! retention of at least 1; every operation returned after it was invoked;
//! every operation of a participant invoked after its previous one returned;
//! the publisher only publishes (taking the sequence numbers 0, 1, ...) and
//! reads the head; a reader only opens, reads and reads the head, and opens
//! before it reads. Let `R` be the retention.
//!
//! - `x < y` ("`x` precedes `y`") means `x.returned <= y.invoked`. This is the
//!   order the general search keeps: it places an operation next only while no
//!   unplaced operation has returned at or before the instant it was invoked
//!   (`may_come_next`). It is transitive, since
//!   `x.returned <= y.invoked < y.returned <= z.invoked`, and in a well-formed
//!   history it contains each participant's program order.
//! - The publisher's publishes, in program order, are publish `0` to `P - 1`,
//!   and publish `k` is invoked at `a_k` and returned at `b_k`. In a
//!   well-formed history `a_0 < b_0 < a_1 < b_1 < ...`, so the publishes are
//!   chained by `<`.
//! - Every other operation is an *observer*: a reader's open, read or head,
//!   or the publisher's head. Each observer `x` has the cursor `c(x)` the
//!   general search gives it (`Sequential::of`): an open sets the cursor to
//!   the number it recorded, an item moves it to `seq + 1`, a lagged moves it
//!   on by `skipped`, and an empty or a head leaves it where it was. The
//!   cursor is a number the history fixes and not an unknown: in a well-formed
//!   history a read follows an open of the same reader, and every reading
//!   that the sequential log admits leaves the cursor exactly where the
//!   reading records it (an item at `seq` leaves it at `seq + 1`; a lagged
//!   that skipped `k` from `c` leaves it at the start of the window, `c + k`).
//! - `A(x)` is the set of `h` in `0..=P` at which the sequential log of
//!   section 2 gives `x` the result it was observed to give, that is, at which
//!   `Sequential::admits` accepts it. It is always an interval, empty or
//!   `[lo, hi]`, and `admitted_heads` gives it in closed form:
//!
//!   | the operation and what it recorded | `h` admitted |
//!   |---|---|
//!   | `head` that returned `s` | `h = s` |
//!   | `open(oldest)` that set the cursor to `0` | `0 <= h <= R` |
//!   | `open(oldest)` that set the cursor to `c > 0` | `h = c + R` |
//!   | `open(head)` that set `c` | `h = c` |
//!   | `open(at n)` that set `c < n` | `h = c` |
//!   | `open(at n)` that set `c = n` | `h >= n` |
//!   | `open(at n)` that set `c > n` | none |
//!   | `item(s, v)`, `c(x) = s`, `v` the value of publish `s` | `s + 1 <= h <= s + R` |
//!   | any other `item` | none |
//!   | `empty` | `h = c(x)` |
//!   | `lagged(k)` with `k >= 1` | `h = c(x) + k + R` |
//!   | `lagged(0)` | none |
//!
//!   each cut to `0..=P`. A unit test holds every row equal to
//!   `Sequential::admits` at every `h`, for small numbers and for numbers at
//!   the edge of `u64`.
//! - `c_x` is the number of publishes that precede `x`, the `k` with
//!   `b_k <= x.invoked`, and `d_x` the number of publishes that `x` does not
//!   precede, the `k` with `a_k < x.returned`. Both are counted on a prefix of
//!   the publishes because `a` and `b` increase, and `c_x <= d_x` because
//!   `a_k < b_k <= x.invoked < x.returned`.
//! - `L_x = max(lo_x, c_x)` and `U_x = min(hi_x, d_x)`, where `A(x)` is
//!   `[lo_x, hi_x]`; when `A(x)` is empty `x` has no `L_x` and no `U_x`, and
//!   condition (I) below fails for it.
//!
//! # What the general search decides
//!
//! A *linearization* is a total order of all operations that extends `<` and
//! in which every operation is admitted at `h` equal to the number of publishes
//! before it. The general search answers `Linearizable` exactly when one
//! exists. It builds such an order one operation at a time, each next
//! operation being the next of one participant, and no placed operation ever
//! needs to be revisited, because what the rest of the order can do depends
//! only on how many operations of each participant are placed (`h` is the
//! number of placed publishes, a cursor is a function of the readings), which
//! is the state Lowe's memo records. Since the publishes are chained by `<`,
//! every linearization lists them in program order, so the publishes before an
//! observer `x` are publish `0` to `g(x) - 1`: `g(x)` is the *gap* of `x`, and
//! `h = g(x)` at `x`.
//!
//! # Theorem
//!
//! A well-formed history has a linearization if and only if
//!
//! - (I) `L_x <= U_x` for every observer `x`, and
//! - (II) `L_x <= U_y` for every pair of observers `x < y`.
//!
//! *Proof, necessity.* Take a linearization and the gaps `g` it gives.
//! Every publish `k` with `b_k <= x.invoked` precedes `x`, so it is before `x`
//! in the linearization, and the publishes before `x` are a prefix: `g(x) >=
//! c_x`. The publishes before `x` are among those that `x` does not precede,
//! also a prefix, so `g(x) <= d_x`. The operation is admitted at `h = g(x)`, so
//! `g(x)` is in `A(x)`. Hence `L_x <= g(x) <= U_x`, which is (I). If `x < y`
//! then `x` is before `y` and the publishes before `x` are among those before
//! `y`, so `g(x) <= g(y)`, and `L_x <= g(x) <= g(y) <= U_y`, which is (II).
//!
//! *Proof, sufficiency.* Assume (I) and (II) and set, for each observer `y`,
//! `g(y)` to the greatest of `L_y` and of every `L_x` with `x < y`. Then
//!
//! 1. `L_y <= g(y) <= U_y`: the first by the definition, the second by (I) for
//!    `L_y` and by (II) for every `L_x` with `x < y`.
//! 2. `x < y` implies `g(x) <= g(y)`: `x < y` is itself one of the terms of
//!    `g(y)`, and every `z < x` has `z < y` because `<` is transitive, so every
//!    term of `g(x)` is a term of `g(y)`.
//!
//! Build the order: for `j = 0, 1, ..., P`, the observers with `g = j` in
//! increasing order of `returned`, ties in any order, and then, for `j < P`,
//! publish `j`. It is a total order of all operations, with `h = g(x)` at
//! every observer. It extends `<`: for two observers `x < y`, `g(x) <= g(y)`
//! by 2, and when the gaps are equal `x.returned <= y.invoked < y.returned`
//! puts `x` first; for publish `k` and an observer `x`, if `k < x` then
//! `b_k <= x.invoked`, so `c_x >= k + 1` and `g(x) >= k + 1`, which puts `x`
//! after publish `k`; if `x < k` then `x.returned <= a_k`, so no publish from
//! `k` on is counted in `d_x`, `d_x <= k`, `g(x) <= k`, and `x` is before
//! publish `k`; two publishes are in program order. And every operation is
//! admitted: an observer at `h = g(x)`, which lies in `[L_x, U_x]` and so in
//! `A(x)`, and a publish always. The publisher's own head operations are
//! observers like any other: being between two publishes in program order they
//! have `c_x = d_x`, which fixes their gap, and `A(x)` then has to contain it.
//! This completes the proof.
//!
//! The theorem is the whole procedure. (I) is a test of each observer alone.
//! (II) asks, for each observer `y`, the greatest `L_x` among the observers
//! that returned at or before `y` was invoked, and that is one sweep over the
//! observers in order of `invoked` with a pointer into the same observers in
//! order of `returned`: the pointer only moves forward, so the cost is the two
//! sorts, `O(n log n)`, and no search of permutations or of frontiers.
//!
//! # What is reported when there is none
//!
//! [`Unplaceable`] names the first observer, in order of `invoked`, that
//! fails: either it fails (I) on its own, and the obstruction says whether
//! that is because no `h` gives it its result at all ([`Obstruction::NeverAdmitted`])
//! or because the results and the clock do not meet ([`Obstruction::ClockWindow`]),
//! or an observer that returned before it was invoked needs more entries than
//! it can see ([`Obstruction::Overtaken`]). An observer whose own window is
//! empty is not counted among the observers (II) draws its maxima from, since
//! the history is not linearizable for it already, so an operation is only
//! ever said to be overtaken by one that could be placed on its own.
//!
//! # When it declines
//!
//! [`Decision::Declined`] is returned, and the general search left to decide,
//! in exactly two situations, and in no other:
//!
//! - [`Declined::NotWellFormed`]: the history is not well formed, which is the
//!   hypothesis of the theorem. The general search answers `Malformed`, with
//!   the same message, since both run the one `well_formed`.
//! - [`Declined::Unmodelled`]: an operation whose call and outcome have no row
//!   in the table above. A well-formed history has none, since `well_formed`
//!   admits only the pairings the table covers; the case exists so that a call
//!   or an outcome added to the history type later is declined until the table
//!   is extended, and the checker never answers about what it has no argument
//!   for.
//!
//! No other shape needs declining. Several readers are the same argument with
//! more observers. A reader whose cursor is "not determined" does not exist:
//! the general search takes the cursor from the readings, as above, and a
//! read before any open is `Malformed`. `head` operations are observers with a
//! one-point `A(x)`. A second publisher is not a history of this type, and a
//! publish in a reader's list is `Malformed`.
//!
//! # What this shows and does not
//!
//! The theorem is proved here for the definition the general search
//! implements, and the closed forms of `A(x)` are held equal to that
//! implementation by a unit test, not by a proof. The code of the sweep is
//! held equal to the general search by
//! `sce-build/tests/broadcast_history_differential.rs`: a test shows the two
//! agree on the histories it tries, and the proof above is what says they
//! agree on the rest.

use std::cmp::Ordering;
use std::fmt;

use super::{well_formed, Call, History, OpenFrom, Operation, Outcome, Sequential};

/// What the specialised checker made of a history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// A linearization exists.
    Linearizable,
    /// None does, and this is the first operation that cannot be placed.
    NotLinearizable(Unplaceable),
    /// The checker has no argument for this history. The general search is
    /// the one that decides it.
    Declined(Declined),
}

/// Why the specialised checker does not decide a history. Both reasons are
/// stated, with the proof that nothing else needs declining, in the module
/// documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declined {
    /// The history cannot have come from a run; the text is the one the
    /// general search gives as `Verdict::Malformed`.
    NotWellFormed(String),
    /// An operation has a pairing of call and outcome the argument does not
    /// cover: operation `at` of `participant` (the publisher is 0, reader `i`
    /// is `i + 1`, as in the general search).
    Unmodelled { participant: usize, at: usize },
}

/// The first operation, in order of invocation, that cannot be placed, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unplaceable {
    /// The participant: the publisher is 0, reader `i` is `i + 1`.
    pub participant: usize,
    /// The position of the operation in that participant's program order.
    pub at: usize,
    pub obstruction: Obstruction,
    /// For each participant, the position of its first operation that cannot
    /// be placed, or the number of its operations when it has none. This is
    /// the meaning `deepest` has in the general search's verdict: the
    /// operations that could not be placed sit at these positions.
    pub deepest: Vec<usize>,
}

/// Why an operation cannot be placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Obstruction {
    /// No number of published entries gives the operation the result it was
    /// observed to give.
    NeverAdmitted,
    /// The result needs a number of entries published within `admitted`, and
    /// the clock allows only a number within `[at_least, at_most]`: at least
    /// `at_least` publishes returned before the operation was invoked, and at
    /// most `at_most` were invoked before it returned. The two do not meet.
    ClockWindow {
        admitted: (u64, u64),
        at_least: u64,
        at_most: u64,
    },
    /// An operation that returned before this one was invoked needs at least
    /// `needs_at_least` entries published, and this one can have seen at most
    /// `at_most`. `by` is that operation: participant and position.
    Overtaken {
        by: (usize, usize),
        needs_at_least: u64,
        at_most: u64,
    },
}

/// Whose operation, as a possessive: the publisher's, or a reader's.
fn whose(participant: usize) -> String {
    match participant.checked_sub(1) {
        None => "the publisher's".to_string(),
        Some(reader) => format!("reader {reader}'s"),
    }
}

impl fmt::Display for Declined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Declined::NotWellFormed(why) => write!(f, "the history is not well formed: {why}"),
            Declined::Unmodelled { participant, at } => write!(
                f,
                "{} operation {at} is one the specialised checker has no argument for",
                whose(*participant)
            ),
        }
    }
}

impl fmt::Display for Obstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Obstruction::NeverAdmitted => f.write_str(
                "no number of published entries gives it the result it was observed to give",
            ),
            Obstruction::ClockWindow {
                admitted,
                at_least,
                at_most,
            } => write!(
                f,
                "its result needs between {} and {} entries published, but at least {at_least} \
                 publishes returned before it was invoked and at most {at_most} were invoked \
                 before it returned",
                admitted.0, admitted.1
            ),
            Obstruction::Overtaken {
                by,
                needs_at_least,
                at_most,
            } => write!(
                f,
                "{} operation {} returned before it was invoked and needs at least \
                 {needs_at_least} entries published, but it can have seen at most {at_most}",
                whose(by.0),
                by.1
            ),
        }
    }
}

impl fmt::Display for Unplaceable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} operation {} cannot be placed: {}",
            whose(self.participant),
            self.at,
            self.obstruction
        )
    }
}

/// An observer, with what the theorem needs of it.
struct Slot {
    participant: usize,
    at: usize,
    invoked: u64,
    returned: u64,
    /// `[L_x, U_x]` when it is non-empty (condition I), and the reason it is
    /// not otherwise.
    window: Result<(u64, u64), Obstruction>,
}

/// Decide `history` by the theorem of the module documentation, or decline.
pub fn check(history: &History) -> Decision {
    let participants = history.participants();
    if let Err(why) = well_formed(history, &participants) {
        return Decision::Declined(Declined::NotWellFormed(why));
    }
    let sequential = Sequential::of(history.retention, &participants);
    let total = sequential.values.len() as u64;

    // The publishes' intervals: `a_k` and `b_k`, each increasing in `k`.
    let (invoked_at, returned_at): (Vec<u64>, Vec<u64>) = participants[0]
        .iter()
        .filter(|op| matches!(op.call, Call::Publish(_)))
        .map(|op| (op.invoked, op.returned))
        .unzip();

    let mut slots: Vec<Slot> = Vec::new();
    for (who, ops) in participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            if who == 0 && matches!(op.call, Call::Publish(_)) {
                continue;
            }
            let admitted = match admitted_heads(&sequential, who, at, op, total) {
                Ok(admitted) => admitted,
                Err(declined) => return Decision::Declined(declined),
            };
            // `c_x`: the publishes with `b_k <= x.invoked`. `d_x`: those with
            // `a_k < x.returned`.
            let at_least = returned_at.partition_point(|&b| b <= op.invoked) as u64;
            let at_most = invoked_at.partition_point(|&a| a < op.returned) as u64;
            let window = match admitted {
                None => Err(Obstruction::NeverAdmitted),
                Some((from, to)) => {
                    let lower = from.max(at_least);
                    let upper = to.min(at_most);
                    if lower <= upper {
                        Ok((lower, upper))
                    } else {
                        Err(Obstruction::ClockWindow {
                            admitted: (from, to),
                            at_least,
                            at_most,
                        })
                    }
                }
            };
            slots.push(Slot {
                participant: who,
                at,
                invoked: op.invoked,
                returned: op.returned,
                window,
            });
        }
    }

    // Condition (II) as one sweep. `by_invoked` visits the observers `y` in
    // order of `invoked`; `by_returned` is the same observers in order of
    // `returned`, and `released` of them have `returned <= y.invoked`, which
    // are exactly the observers that precede `y`. `forcing` is the greatest
    // `L_x` among the released ones that satisfy (I) on their own, with the
    // observer it came from.
    let mut by_invoked: Vec<usize> = (0..slots.len()).collect();
    by_invoked.sort_by_key(|&s| (slots[s].invoked, s));
    let mut by_returned: Vec<usize> = (0..slots.len()).collect();
    by_returned.sort_by_key(|&s| (slots[s].returned, s));

    let mut deepest: Vec<usize> = participants.iter().map(Vec::len).collect();
    let mut culprit: Option<(usize, Obstruction)> = None;
    let mut released = 0usize;
    let mut forcing: Option<(u64, usize)> = None;
    for &y in &by_invoked {
        while let Some(&x) = by_returned.get(released) {
            if slots[x].returned > slots[y].invoked {
                break;
            }
            if let Ok((lower, _)) = &slots[x].window {
                if forcing.is_none_or(|(best, _)| *lower > best) {
                    forcing = Some((*lower, x));
                }
            }
            released += 1;
        }
        let obstruction = match &slots[y].window {
            Err(own) => Some(own.clone()),
            Ok((_, upper)) => match forcing {
                Some((needs, x)) if needs > *upper => Some(Obstruction::Overtaken {
                    by: (slots[x].participant, slots[x].at),
                    needs_at_least: needs,
                    at_most: *upper,
                }),
                _ => None,
            },
        };
        if let Some(obstruction) = obstruction {
            let slot = &slots[y];
            deepest[slot.participant] = deepest[slot.participant].min(slot.at);
            if culprit.is_none() {
                culprit = Some((y, obstruction));
            }
        }
    }

    match culprit {
        None => Decision::Linearizable,
        Some((y, obstruction)) => Decision::NotLinearizable(Unplaceable {
            participant: slots[y].participant,
            at: slots[y].at,
            obstruction,
            deepest,
        }),
    }
}

/// `[from, to]` cut to `0..=total`, or `None` when nothing is left.
fn within(total: u64, from: u64, to: u64) -> Option<(u64, u64)> {
    let to = to.min(total);
    (from <= to).then_some((from, to))
}

/// `A(x)`: the numbers of published entries, among `0..=total`, at which the
/// sequential log gives `op` (operation `at` of participant `who`) the result
/// it was observed to give, as the one interval they form, or `None` when
/// there is no such number. The table is in the module documentation.
fn admitted_heads(
    sequential: &Sequential,
    who: usize,
    at: usize,
    op: &Operation,
    total: u64,
) -> Result<Option<(u64, u64)>, Declined> {
    let retention = sequential.retention;
    let unmodelled = Declined::Unmodelled {
        participant: who,
        at,
    };
    match (op.call, op.outcome) {
        (Call::Head, Outcome::Head(seen)) => Ok(within(total, seen, seen)),
        (Call::Open(from), Outcome::Opened(cursor)) => Ok(match from {
            OpenFrom::Oldest if cursor == 0 => within(total, 0, retention),
            OpenFrom::Oldest => cursor
                .checked_add(retention)
                .and_then(|h| within(total, h, h)),
            OpenFrom::Head => within(total, cursor, cursor),
            OpenFrom::At(n) => match cursor.cmp(&n) {
                Ordering::Less => within(total, cursor, cursor),
                Ordering::Equal => within(total, n, u64::MAX),
                Ordering::Greater => None,
            },
        }),
        // Only a reader reads, and `well_formed` has seen to it.
        (Call::Read, _) if who == 0 => Err(unmodelled),
        (Call::Read, outcome) => {
            let Some(cursor) = sequential
                .cursor
                .get(who)
                .and_then(|before| before.get(at))
                .copied()
                .flatten()
            else {
                // A read before any open: no number of entries admits it.
                return Ok(None);
            };
            match outcome {
                Outcome::Item { seq, value } => {
                    let published = usize::try_from(seq)
                        .ok()
                        .and_then(|n| sequential.values.get(n));
                    Ok(if cursor == seq && published == Some(&value) {
                        within(total, seq.saturating_add(1), seq.saturating_add(retention))
                    } else {
                        None
                    })
                }
                Outcome::Empty => Ok(within(total, cursor, cursor)),
                Outcome::Lagged { skipped } => Ok(if skipped == 0 {
                    None
                } else {
                    cursor
                        .checked_add(skipped)
                        .and_then(|start| start.checked_add(retention))
                        .and_then(|h| within(total, h, h))
                }),
                _ => Err(unmodelled),
            }
        }
        _ => Err(unmodelled),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{check_general, Verdict};
    use super::*;

    fn op(call: Call, outcome: Outcome, invoked: u64, returned: u64) -> Operation {
        Operation {
            call,
            outcome,
            invoked,
            returned,
        }
    }

    /// `0..=limit` and the two largest `u64`, the numbers a recorded history
    /// could hold at the edge of the type.
    fn numbers(limit: u64) -> Vec<u64> {
        (0..=limit).chain([u64::MAX - 1, u64::MAX]).collect()
    }

    /// Every row of the table of `A(x)` equals the sequential log's own
    /// `admits` at every number of published entries, for small numbers and at
    /// the edge of `u64`.
    #[test]
    fn the_closed_forms_are_the_sequential_log_at_every_head() {
        let mut compared = 0usize;
        for retention in [1u64, 2, 3, u64::MAX] {
            for total in 0..=4u64 {
                let publisher: Vec<Operation> = (0..total)
                    .map(|k| {
                        op(
                            Call::Publish(100 + k),
                            Outcome::Published(k),
                            10 * k + 1,
                            10 * k + 2,
                        )
                    })
                    .collect();
                let reach = total + 2;
                // Each probe with the operations the reader did before it.
                let mut probes: Vec<(Vec<Operation>, Operation)> = Vec::new();
                for seen in numbers(reach) {
                    probes.push((vec![], op(Call::Head, Outcome::Head(seen), 1, 2)));
                }
                for cursor in numbers(total + 3) {
                    let mut froms = vec![OpenFrom::Oldest, OpenFrom::Head];
                    froms.extend(numbers(reach).into_iter().map(OpenFrom::At));
                    for from in froms {
                        probes.push((vec![], op(Call::Open(from), Outcome::Opened(cursor), 1, 2)));
                    }
                }
                for before in numbers(reach) {
                    let opened = op(Call::Open(OpenFrom::At(0)), Outcome::Opened(before), 1, 2);
                    let mut reads = vec![Outcome::Empty];
                    for seq in numbers(reach) {
                        for value in [seq.wrapping_add(100), 7] {
                            reads.push(Outcome::Item { seq, value });
                        }
                    }
                    for skipped in numbers(total + 4) {
                        reads.push(Outcome::Lagged { skipped });
                    }
                    for read in reads {
                        probes.push((vec![opened], op(Call::Read, read, 3, 4)));
                    }
                }
                for (earlier, probe) in probes {
                    let mut readings = earlier.clone();
                    readings.push(probe);
                    let at = earlier.len();
                    let participants = vec![publisher.clone(), readings];
                    let sequential = Sequential::of(retention, &participants);
                    let interval = admitted_heads(&sequential, 1, at, &probe, total)
                        .expect("a pairing the table covers");
                    if let Some((from, to)) = interval {
                        assert!(from <= to && to <= total, "{probe:?}: {from}..={to}");
                    }
                    for h in 0..=total {
                        let inside = interval.is_some_and(|(from, to)| (from..=to).contains(&h));
                        assert_eq!(
                            sequential.admits(1, at, &probe, h),
                            inside,
                            "retention {retention}, {total} published, {probe:?} after \
                             {earlier:?}, h = {h}, interval {interval:?}"
                        );
                        compared += 1;
                    }
                }
            }
        }
        // Per retention, with `t` published: heads `t + 5`, opens
        // `(t + 6) * (t + 7)`, reads `(t + 5) * (3t + 18)`, each at `t + 1`
        // values of `h`. t = 0: 137 * 1; t = 1: 188 * 2 = 376; t = 2:
        // 247 * 3 = 741; t = 3: 314 * 4 = 1256; t = 4: 389 * 5 = 1945; the
        // sum is 4455, and the four retentions make 17820.
        assert_eq!(compared, 17_820);
    }

    /// A pairing of call and outcome the table has no row for is declined at
    /// the operation, never guessed at; `well_formed` keeps such a pairing from
    /// reaching the checker in a history, so the table is called directly.
    #[test]
    fn a_pairing_the_table_does_not_cover_is_declined_where_it_stands() {
        let nobody: Vec<Vec<Operation>> = vec![vec![], vec![]];
        let sequential = Sequential::of(2, &nobody);
        let stray = op(Call::Publish(1), Outcome::Head(0), 1, 2);
        assert_eq!(
            admitted_heads(&sequential, 1, 4, &stray, 0),
            Err(Declined::Unmodelled {
                participant: 1,
                at: 4
            })
        );
        let publisher_reads = op(Call::Read, Outcome::Empty, 1, 2);
        assert_eq!(
            admitted_heads(&sequential, 0, 0, &publisher_reads, 0),
            Err(Declined::Unmodelled {
                participant: 0,
                at: 0
            })
        );
        let read_that_opened = op(Call::Read, Outcome::Opened(0), 1, 2);
        let opened = op(Call::Open(OpenFrom::Oldest), Outcome::Opened(0), 1, 2);
        let sequential = Sequential::of(2, &[vec![], vec![opened, read_that_opened]]);
        assert_eq!(sequential.cursor[1], vec![None, Some(0)]);
        assert_eq!(
            admitted_heads(&sequential, 1, 1, &read_that_opened, 0),
            Err(Declined::Unmodelled {
                participant: 1,
                at: 1
            })
        );
    }

    /// A history that is not well formed is declined with the message the
    /// general search gives as `Malformed`.
    #[test]
    fn a_history_that_is_not_well_formed_is_declined_with_the_general_message() {
        // Operation 0 of the publisher returned at 1 but was invoked at 2.
        let history = History {
            retention: 2,
            publisher: vec![op(Call::Publish(5), Outcome::Published(0), 2, 1)],
            readers: vec![],
        };
        let Verdict::Malformed(general) = check_general(&history) else {
            panic!("the general search should call this malformed");
        };
        assert_eq!(
            check(&history),
            Decision::Declined(Declined::NotWellFormed(general))
        );
    }

    /// A read that sees nothing of an entry that returned before it began.
    ///
    /// Retention 2, one publish at [1, 2), a reader that opens at [3, 4) with
    /// cursor 0 and reads `empty` at [5, 6). The empty needs `h = 0`, the
    /// cursor. One publish returned at 2 <= 5, so `c = 1`; one publish was
    /// invoked at 1 < 6, so `d = 1`; the window is `[max(0, 1), min(0, 1)] =
    /// [1, 0]`, empty. The open is `open(oldest)` with cursor 0, so `h` in
    /// `0..=2` cut to `0..=1`; `c = 1` (2 <= 3), `d = 1` (1 < 4): `[1, 1]`.
    #[test]
    fn an_empty_after_a_publish_returned_is_a_clock_window_that_does_not_meet() {
        let history = History {
            retention: 2,
            publisher: vec![op(Call::Publish(5), Outcome::Published(0), 1, 2)],
            readers: vec![vec![
                op(Call::Open(OpenFrom::Oldest), Outcome::Opened(0), 3, 4),
                op(Call::Read, Outcome::Empty, 5, 6),
            ]],
        };
        assert_eq!(
            check(&history),
            Decision::NotLinearizable(Unplaceable {
                participant: 1,
                at: 1,
                obstruction: Obstruction::ClockWindow {
                    admitted: (0, 0),
                    at_least: 1,
                    at_most: 1
                },
                // The publisher has 1 operation and none is unplaceable; the
                // reader's operation 1 is.
                deepest: vec![1, 1],
            })
        );
        assert!(matches!(
            check_general(&history),
            Verdict::NotLinearizable { .. }
        ));
    }

    /// A head that goes backwards between two readers.
    ///
    /// One publish at [1, 10), retention 1. Reader 0 reads `head = 1` at
    /// [2, 3): `A = [1, 1]`, `c = 0` (10 > 2), `d = 1` (1 < 3), window
    /// `[1, 1]`. Reader 1 reads `head = 0` at [4, 5): `A = [0, 0]`, `c = 0`,
    /// `d = 1` (1 < 5), window `[0, 0]`. The first returned at 3 <= 4, so it
    /// precedes the second, and it needs 1 where the second can have seen 0.
    #[test]
    fn a_head_that_goes_backwards_is_overtaken() {
        let history = History {
            retention: 1,
            publisher: vec![op(Call::Publish(5), Outcome::Published(0), 1, 10)],
            readers: vec![
                vec![op(Call::Head, Outcome::Head(1), 2, 3)],
                vec![op(Call::Head, Outcome::Head(0), 4, 5)],
            ],
        };
        assert_eq!(
            check(&history),
            Decision::NotLinearizable(Unplaceable {
                participant: 2,
                at: 0,
                obstruction: Obstruction::Overtaken {
                    by: (1, 0),
                    needs_at_least: 1,
                    at_most: 0
                },
                deepest: vec![1, 1, 0],
            })
        );
        assert!(matches!(
            check_general(&history),
            Verdict::NotLinearizable { .. }
        ));
    }

    /// A value that was never published has no number of entries that admits it.
    ///
    /// One publish of 5 at [1, 2), retention 2, a reader that opens at [3, 4)
    /// with cursor 0 and reads the item `(0, 6)` at [5, 6): the entry at 0 held
    /// 5, so `A` is empty.
    #[test]
    fn an_item_of_a_value_never_published_is_never_admitted() {
        let history = History {
            retention: 2,
            publisher: vec![op(Call::Publish(5), Outcome::Published(0), 1, 2)],
            readers: vec![vec![
                op(Call::Open(OpenFrom::Oldest), Outcome::Opened(0), 3, 4),
                op(Call::Read, Outcome::Item { seq: 0, value: 6 }, 5, 6),
            ]],
        };
        let Decision::NotLinearizable(found) = check(&history) else {
            panic!("an item of a value never published is not linearizable");
        };
        assert_eq!(found.obstruction, Obstruction::NeverAdmitted);
        assert_eq!((found.participant, found.at), (1, 1));
    }

    /// An operation that returned at the instant another was invoked precedes
    /// it, and one that overlaps it does not.
    ///
    /// One publish at [1, 3), retention 2. A head of 0 at [3, 4): the publish
    /// returned at 3 <= 3, so `c = 1` and `A = [0, 0]` leaves `[1, 0]`, empty:
    /// not linearizable. The same head at [2, 4): `b_0 = 3 <= 2` is false, so
    /// `c = 0`, and `d = 1` (1 < 4), so the window is `[0, 0]`: linearizable.
    #[test]
    fn touching_intervals_are_ordered_and_overlapping_ones_are_not() {
        let history = |invoked, returned| History {
            retention: 2,
            publisher: vec![op(Call::Publish(5), Outcome::Published(0), 1, 3)],
            readers: vec![vec![op(Call::Head, Outcome::Head(0), invoked, returned)]],
        };
        assert!(matches!(
            check(&history(3, 4)),
            Decision::NotLinearizable(_)
        ));
        assert!(matches!(
            check_general(&history(3, 4)),
            Verdict::NotLinearizable { .. }
        ));
        assert_eq!(check(&history(2, 4)), Decision::Linearizable);
        assert_eq!(check_general(&history(2, 4)), Verdict::Linearizable);
    }

    /// A history with no operation has nothing to place.
    #[test]
    fn a_history_with_no_operation_is_linearizable() {
        let history = History {
            retention: 2,
            publisher: vec![],
            readers: vec![vec![]],
        };
        assert_eq!(check(&history), Decision::Linearizable);
    }

    /// The words an operator reads.
    #[test]
    fn what_cannot_be_placed_is_said_in_words() {
        let found = Unplaceable {
            participant: 2,
            at: 0,
            obstruction: Obstruction::Overtaken {
                by: (1, 3),
                needs_at_least: 4,
                at_most: 2,
            },
            deepest: vec![1, 1, 0],
        };
        assert_eq!(
            found.to_string(),
            "reader 1's operation 0 cannot be placed: reader 0's operation 3 returned before it \
             was invoked and needs at least 4 entries published, but it can have seen at most 2"
        );
        assert_eq!(
            Declined::NotWellFormed("x".to_string()).to_string(),
            "the history is not well formed: x"
        );
        assert_eq!(
            Declined::Unmodelled {
                participant: 0,
                at: 7
            }
            .to_string(),
            "the publisher's operation 7 is one the specialised checker has no argument for"
        );
    }
}
