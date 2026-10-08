// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A linearizability checker for queue histories (SCE Protocol-Synthesis
//! RFC §synth-5-P, verification layer 2).
//!
//! A history is what the participants of one run observed: for each
//! participant, its operations in program order, each with the instant it
//! was invoked and the instant it returned, both read from one clock every
//! participant shares. The history is linearizable when its operations can
//! be put in one sequence that
//!
//! - keeps every operation that returned before another was invoked ahead
//!   of it, and
//! - is a run of the sequential bounded FIFO of the queue's capacity, in
//!   which each operation gives the result it was observed to give, a
//!   refused push judged as the history's [`Refusal`] says.
//!
//! [`check`] searches for that sequence — Wing and Gong's search, with the
//! memo of states already explored that Lowe added — and says which
//! operations it could not get past when there is none.
//!
//! Nothing here knows an implementation or a language. A history recorded
//! in this shape is judged by this one search, whichever backend ran.

use std::collections::{HashSet, VecDeque};

/// What an operation asked of the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Call {
    /// Push this value.
    Push(u64),
    /// Pop the oldest value.
    Pop,
}

/// What an operation was observed to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The push took its value.
    Pushed,
    /// The push was refused because the queue held its capacity.
    Full,
    /// The pop returned this value.
    Popped(u64),
    /// The pop found the queue empty.
    Empty,
}

/// One observed operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub call: Call,
    pub outcome: Outcome,
    /// Clock reading taken immediately before the call.
    pub invoked: u64,
    /// Clock reading taken immediately after it returned.
    pub returned: u64,
}

/// When a queue may refuse a push.
///
/// The sequential queue refuses a push exactly when it holds its capacity.
/// A concurrent queue whose slots are handed back one at a time cannot always
/// do the same: a pop that has taken its element out of the queue still holds
/// the slot it was in until it has handed the slot back, and a push that has
/// taken a free slot holds it until it has published its element. A push
/// that looks for a slot in that window finds none, and refusing it is all a
/// lock-free queue can do, since waiting for a participant that may be
/// stopped is not lock-free. The element count the refusal can be blamed on
/// is therefore short by at most the slots those other operations hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A push is refused only when the queue holds its capacity. The one
    /// producer and one consumer of a Lamport ring keep this: its slot is
    /// handed back by the same store that retires the element.
    AtCapacity,
    /// A push is also refused while operations of other participants that
    /// overlap it hold the slots the queue is short of: at most one slot per
    /// other participant, since a participant has one operation in flight.
    /// Capacity is exact at rest, and a refusal that no overlapping
    /// operation could explain is still a violation.
    WhileSlotsAreHeld,
}

/// Everything one run observed.
#[derive(Debug, Clone)]
pub struct History {
    /// The capacity of the sequential queue the run is judged against.
    pub capacity: usize,
    /// When a push may be refused.
    pub refusal: Refusal,
    /// One list per participant, in that participant's program order.
    pub participants: Vec<Vec<Operation>>,
}

/// The checker's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// A sequence exists.
    Linearizable,
    /// No sequence exists. `deepest` is the furthest the search got: how
    /// many of each participant's operations it had placed, which puts the
    /// operations that could not be placed next to each other.
    NotLinearizable {
        explored: usize,
        deepest: Vec<usize>,
    },
    /// The history cannot have come from a run: an operation returned
    /// before it was invoked, or a participant's operations overlap.
    Malformed(String),
}

/// Judge `history`.
pub fn check(history: &History) -> Verdict {
    if let Err(why) = well_formed(history) {
        return Verdict::Malformed(why);
    }

    let participants = &history.participants;
    let held = slots_other_participants_may_hold(history);
    let start = State {
        frontier: vec![0; participants.len()],
        queue: VecDeque::new(),
    };
    let mut seen: HashSet<State> = HashSet::new();
    seen.insert(start.clone());
    let mut deepest = start.frontier.clone();
    let mut stack = vec![Frame {
        state: start,
        next_choice: 0,
    }];

    while let Some(frame) = stack.last_mut() {
        if frame
            .state
            .frontier
            .iter()
            .zip(participants)
            .all(|(placed, ops)| *placed == ops.len())
        {
            return Verdict::Linearizable;
        }

        let mut successor = None;
        while frame.next_choice < participants.len() {
            let who = frame.next_choice;
            frame.next_choice += 1;
            let Some(op) = participants[who].get(frame.state.frontier[who]) else {
                continue;
            };
            if !may_come_next(participants, &frame.state.frontier, who, op) {
                continue;
            }
            let slack = held[who][frame.state.frontier[who]];
            let Some(queue) = apply(&frame.state.queue, op, history.capacity, slack) else {
                continue;
            };
            let mut frontier = frame.state.frontier.clone();
            frontier[who] += 1;
            let next = State { frontier, queue };
            if seen.insert(next.clone()) {
                successor = Some(next);
                break;
            }
        }

        match successor {
            Some(state) => {
                if state.frontier.iter().sum::<usize>() > deepest.iter().sum::<usize>() {
                    deepest = state.frontier.clone();
                }
                stack.push(Frame {
                    state,
                    next_choice: 0,
                });
            }
            None => {
                stack.pop();
            }
        }
    }

    Verdict::NotLinearizable {
        explored: seen.len(),
        deepest,
    }
}

/// A point in the search: how many of each participant's operations are
/// placed, and the sequential queue those placements produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    frontier: Vec<usize>,
    queue: VecDeque<u64>,
}

struct Frame {
    state: State,
    /// The participant whose next operation the search tries next from here.
    next_choice: usize,
}

/// Whether `op`, the next unplaced operation of participant `who`, may be
/// placed next: no unplaced operation may have returned before it was
/// invoked. A participant's operations return in program order, so the
/// earliest-returning unplaced one of each is the one at its frontier.
fn may_come_next(
    participants: &[Vec<Operation>],
    frontier: &[usize],
    who: usize,
    op: &Operation,
) -> bool {
    participants.iter().enumerate().all(|(other, ops)| {
        other == who
            || match ops.get(frontier[other]) {
                None => true,
                Some(first) => first.returned > op.invoked,
            }
    })
}

/// For each operation, how many slots the operations of other participants
/// may hold while it runs, which is how short of the capacity the queue may
/// be when that operation is a refused push.
///
/// Zero everywhere under [`Refusal::AtCapacity`]. Under
/// [`Refusal::WhileSlotsAreHeld`] it is, for a push observed refused, the
/// number of other participants with any operation overlapping it, and zero
/// for every other operation: only a refusal has a count to explain.
fn slots_other_participants_may_hold(history: &History) -> Vec<Vec<usize>> {
    history
        .participants
        .iter()
        .enumerate()
        .map(|(who, ops)| {
            ops.iter()
                .map(|op| {
                    if history.refusal != Refusal::WhileSlotsAreHeld || op.outcome != Outcome::Full
                    {
                        return 0;
                    }
                    history
                        .participants
                        .iter()
                        .enumerate()
                        .filter(|(other, others)| {
                            *other != who
                                && others
                                    .iter()
                                    .any(|o| o.invoked < op.returned && op.invoked < o.returned)
                        })
                        .count()
                })
                .collect()
        })
        .collect()
}

/// The sequential queue after `op`, or `None` when the sequential queue
/// would not give the outcome `op` was observed to give. `slack` is how many
/// slots other participants' operations may be holding, which only a refused
/// push can draw on.
fn apply(
    queue: &VecDeque<u64>,
    op: &Operation,
    capacity: usize,
    slack: usize,
) -> Option<VecDeque<u64>> {
    match (op.call, op.outcome) {
        (Call::Push(value), Outcome::Pushed) if queue.len() < capacity => {
            let mut next = queue.clone();
            next.push_back(value);
            Some(next)
        }
        (Call::Push(_), Outcome::Full) if queue.len() + slack >= capacity => Some(queue.clone()),
        (Call::Pop, Outcome::Popped(value)) if queue.front() == Some(&value) => {
            let mut next = queue.clone();
            next.pop_front();
            Some(next)
        }
        (Call::Pop, Outcome::Empty) if queue.is_empty() => Some(queue.clone()),
        _ => None,
    }
}

fn well_formed(history: &History) -> Result<(), String> {
    for (who, ops) in history.participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            if op.returned <= op.invoked {
                return Err(format!(
                    "participant {who} operation {at} returned at {} but was invoked at {}",
                    op.returned, op.invoked
                ));
            }
            if let Some(previous) = at.checked_sub(1).map(|p| &ops[p]) {
                if op.invoked <= previous.returned {
                    return Err(format!(
                        "participant {who} operation {at} was invoked at {} before its previous \
                         operation returned at {}",
                        op.invoked, previous.returned
                    ));
                }
            }
            let consistent = matches!(
                (op.call, op.outcome),
                (Call::Push(_), Outcome::Pushed | Outcome::Full)
                    | (Call::Pop, Outcome::Popped(_) | Outcome::Empty)
            );
            if !consistent {
                return Err(format!(
                    "participant {who} operation {at} is a {:?} that returned {:?}",
                    op.call, op.outcome
                ));
            }
        }
    }
    Ok(())
}
