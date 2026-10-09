// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A linearizability checker for queue histories (SCE Protocol-Synthesis
//! RFC §synth-5-P, verification layer 2), and the one JSON format a history
//! is written in.
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
//! in this shape is judged by this one search, whichever backend ran: the
//! stress run of each backend writes the JSON form ([`History::to_json`],
//! read by [`History::from_json`], described by
//! `tests/forge/conformance/queue_history.schema.json`), and
//! `sce-codegen check-queue-history` judges the files. The checker lives in
//! this crate, and not in a backend's tests, because that command is the one
//! binary every backend's job already has.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

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

/// When a queue may answer a pop with "empty".
///
/// The sequential queue answers empty exactly when it holds nothing. The Vyukov
/// intrusive list does not keep that (SCE Protocol-Synthesis RFC §synth-5-P,
/// Algorithm selection): a producer swaps itself into the tail and only then
/// links its predecessor to itself, and until that second step every element
/// pushed after it is unreachable from the head, so a pop that begins in the
/// window answers empty while elements whose pushes have completed sit behind
/// the stalled one. That is why the row's pop is `blocking`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyPops {
    /// A pop answers empty only when the queue holds nothing.
    #[default]
    Exact,
    /// A pop is also excused for answering empty when it overlaps a push: a
    /// push whose interval has not ended before the pop began is a producer that
    /// may be stalled between its two steps, and what it hides is exactly what a
    /// completed push behind it left unreachable. An empty answer that no
    /// overlapping push could explain is still a violation.
    WhileAPushIsInFlight,
}

/// Everything one run observed.
#[derive(Debug, Clone)]
pub struct History {
    /// The capacity of the sequential queue the run is judged against.
    pub capacity: usize,
    /// When a push may be refused.
    pub refusal: Refusal,
    /// When a pop may answer empty.
    pub empty_pops: EmptyPops,
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

    // An empty answer the history's `EmptyPops` excuses constrains nothing, so
    // it is taken out before the search; the others are judged as written.
    let excused;
    let participants = match history.empty_pops {
        EmptyPops::Exact => &history.participants,
        EmptyPops::WhileAPushIsInFlight => {
            excused = without_excused_empty_pops(&history.participants);
            &excused
        }
    };
    let held = slots_other_participants_may_hold(participants, history.refusal);
    let required = pushes_each_push_must_follow(participants);
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
            if !required[who][frame.state.frontier[who]]
                .iter()
                .zip(&frame.state.frontier)
                .all(|(needed, placed)| placed >= needed)
            {
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
fn slots_other_participants_may_hold(
    participants: &[Vec<Operation>],
    refusal: Refusal,
) -> Vec<Vec<usize>> {
    participants
        .iter()
        .enumerate()
        .map(|(who, ops)| {
            ops.iter()
                .map(|op| {
                    if refusal != Refusal::WhileSlotsAreHeld || op.outcome != Outcome::Full {
                        return 0;
                    }
                    participants
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

/// For each push, how many of each participant's operations must already be
/// placed before it may be: the pushes of every value whose pop returned before
/// the pop of this push's value was invoked.
///
/// A sequential queue returns values in the order it took them, so when one pop
/// returned before another was invoked the first pop's value was pushed before
/// the second's, whatever the real-time order of the two pushes. The search would
/// otherwise place overlapping pushes in either order and learn that it chose
/// wrongly only when the pops are reached, and a queue whose capacity does not
/// bound its contents (the intrusive list) lets it choose wrongly exponentially
/// many times. Only a value pushed once and popped once is used: with a repeated
/// value the pop does not say which push it took, and the rule would be a guess.
/// Every entry is zero for an operation that is not such a push.
fn pushes_each_push_must_follow(participants: &[Vec<Operation>]) -> Vec<Vec<Vec<usize>>> {
    let mut pushes: HashMap<u64, Option<(usize, usize)>> = HashMap::new();
    let mut pops: HashMap<u64, Option<(u64, u64)>> = HashMap::new();
    for (who, ops) in participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            match (op.call, op.outcome) {
                (Call::Push(value), Outcome::Pushed) => {
                    let entry = pushes.entry(value).or_insert(Some((who, at)));
                    if *entry != Some((who, at)) {
                        *entry = None;
                    }
                }
                (Call::Pop, Outcome::Popped(value)) => {
                    let seen = pops.contains_key(&value);
                    pops.insert(
                        value,
                        if seen {
                            None
                        } else {
                            Some((op.invoked, op.returned))
                        },
                    );
                }
                _ => {}
            }
        }
    }
    // A popped value and where it was pushed, for the values the rule can use.
    let usable: Vec<(u64, u64, u64, (usize, usize))> = pops
        .iter()
        .filter_map(|(value, pop)| match (pop, pushes.get(value)) {
            (Some((invoked, returned)), Some(Some(at))) => Some((*value, *invoked, *returned, *at)),
            _ => None,
        })
        .collect();
    participants
        .iter()
        .map(|ops| {
            ops.iter()
                .map(|op| {
                    let mut needed = vec![0; participants.len()];
                    if let (Call::Push(value), Outcome::Pushed) = (op.call, op.outcome) {
                        if let Some((_, invoked, _, _)) = usable.iter().find(|u| u.0 == value) {
                            for (_, _, returned, (who, at)) in &usable {
                                if returned < invoked {
                                    needed[*who] = needed[*who].max(at + 1);
                                }
                            }
                        }
                    }
                    needed
                })
                .collect()
        })
        .collect()
}

/// `participants` without the pops that answered empty and overlap a push
/// ([`EmptyPops::WhileAPushIsInFlight`]): a push whose interval does not end
/// before the pop began and does not begin after the pop ended. Each
/// participant's other operations keep their order.
fn without_excused_empty_pops(participants: &[Vec<Operation>]) -> Vec<Vec<Operation>> {
    participants
        .iter()
        .map(|ops| {
            ops.iter()
                .copied()
                .filter(|op| {
                    !(op.outcome == Outcome::Empty
                        && participants.iter().flatten().any(|other| {
                            matches!(other.call, Call::Push(_))
                                && other.invoked < op.returned
                                && op.invoked < other.returned
                        }))
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

// ─────────────────────────────── The JSON form ───────────────────────────────

/// The version of the JSON form this module reads and writes. A reader that
/// meets another refuses the file instead of guessing at it.
pub const HISTORY_FORMAT_VERSION: u64 = 1;

/// Why a file is not a history of this format: the place in the document and
/// what is wrong there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryError(pub String);

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HistoryError {}

/// The document as it sits in a file. Unknown fields are refused, so a field
/// a writer adds that this reader would drop is an error and not a silent
/// loss of what the writer meant.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryDocument {
    version: u64,
    capacity: usize,
    refusal: RefusalWire,
    /// When a pop may answer empty. Absent means `exact`, the sequential
    /// queue's rule, so a document written before the field existed reads the
    /// same.
    #[serde(default, skip_serializing_if = "EmptyPopsWire::is_exact")]
    empty_pops: EmptyPopsWire,
    participants: Vec<Vec<OperationWire>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "kebab-case")]
enum EmptyPopsWire {
    #[default]
    Exact,
    WhileAPushIsInFlight,
}

impl EmptyPopsWire {
    fn is_exact(&self) -> bool {
        matches!(self, EmptyPopsWire::Exact)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum RefusalWire {
    AtCapacity,
    WhileSlotsAreHeld,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CallWire {
    Push,
    Pop,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OutcomeWire {
    Pushed,
    Full,
    Popped,
    Empty,
}

/// One operation. `value` is the value pushed (a push) or the value returned
/// (a pop that popped), and is absent from a pop that found the queue empty.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct OperationWire {
    call: CallWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<u64>,
    outcome: OutcomeWire,
    invoked: u64,
    returned: u64,
}

impl History {
    /// Read a history from its JSON form.
    ///
    /// Refuses a document that is not JSON, names another version, has a field
    /// this format does not, or states an operation that cannot be one: a push
    /// without its value, a pop that popped without its value, a pop that
    /// found the queue empty and names one, or a call paired with an outcome
    /// the other call has. What this checks is the shape; whether the times
    /// are those of a run is [`check`]'s `Malformed`.
    pub fn from_json(text: &str) -> Result<History, HistoryError> {
        let document: HistoryDocument = serde_json::from_str(text)
            .map_err(|e| HistoryError(format!("not a queue history: {e}")))?;
        if document.version != HISTORY_FORMAT_VERSION {
            return Err(HistoryError(format!(
                "version {} is not the version this reader reads ({HISTORY_FORMAT_VERSION})",
                document.version
            )));
        }
        let mut participants = Vec::with_capacity(document.participants.len());
        for (who, operations) in document.participants.iter().enumerate() {
            let mut observed = Vec::with_capacity(operations.len());
            for (at, operation) in operations.iter().enumerate() {
                let place = format!("participants[{who}][{at}]");
                observed.push(operation.observed(&place)?);
            }
            participants.push(observed);
        }
        Ok(History {
            capacity: document.capacity,
            refusal: match document.refusal {
                RefusalWire::AtCapacity => Refusal::AtCapacity,
                RefusalWire::WhileSlotsAreHeld => Refusal::WhileSlotsAreHeld,
            },
            empty_pops: match document.empty_pops {
                EmptyPopsWire::Exact => EmptyPops::Exact,
                EmptyPopsWire::WhileAPushIsInFlight => EmptyPops::WhileAPushIsInFlight,
            },
            participants,
        })
    }

    /// Write the history in its JSON form, as [`History::from_json`] reads it.
    pub fn to_json(&self) -> String {
        let document = HistoryDocument {
            version: HISTORY_FORMAT_VERSION,
            capacity: self.capacity,
            refusal: match self.refusal {
                Refusal::AtCapacity => RefusalWire::AtCapacity,
                Refusal::WhileSlotsAreHeld => RefusalWire::WhileSlotsAreHeld,
            },
            empty_pops: match self.empty_pops {
                EmptyPops::Exact => EmptyPopsWire::Exact,
                EmptyPops::WhileAPushIsInFlight => EmptyPopsWire::WhileAPushIsInFlight,
            },
            participants: self
                .participants
                .iter()
                .map(|operations| operations.iter().map(OperationWire::of).collect())
                .collect(),
        };
        serde_json::to_string_pretty(&document).expect("a history serialises")
    }
}

impl OperationWire {
    fn of(operation: &Operation) -> OperationWire {
        let (call, pushed) = match operation.call {
            Call::Push(value) => (CallWire::Push, Some(value)),
            Call::Pop => (CallWire::Pop, None),
        };
        let (outcome, popped) = match operation.outcome {
            Outcome::Pushed => (OutcomeWire::Pushed, None),
            Outcome::Full => (OutcomeWire::Full, None),
            Outcome::Popped(value) => (OutcomeWire::Popped, Some(value)),
            Outcome::Empty => (OutcomeWire::Empty, None),
        };
        OperationWire {
            call,
            value: pushed.or(popped),
            outcome,
            invoked: operation.invoked,
            returned: operation.returned,
        }
    }

    fn observed(&self, place: &str) -> Result<Operation, HistoryError> {
        let refuse = |reason: &str| HistoryError(format!("{place}: {reason}"));
        let (call, outcome) = match (self.call, self.outcome, self.value) {
            (CallWire::Push, OutcomeWire::Pushed, Some(value)) => {
                (Call::Push(value), Outcome::Pushed)
            }
            (CallWire::Push, OutcomeWire::Full, Some(value)) => (Call::Push(value), Outcome::Full),
            (CallWire::Push, OutcomeWire::Pushed | OutcomeWire::Full, None) => {
                return Err(refuse("a push names the value it pushed"));
            }
            (CallWire::Push, OutcomeWire::Popped | OutcomeWire::Empty, _) => {
                return Err(refuse("a push is pushed or full, not popped or empty"));
            }
            (CallWire::Pop, OutcomeWire::Popped, Some(value)) => {
                (Call::Pop, Outcome::Popped(value))
            }
            (CallWire::Pop, OutcomeWire::Popped, None) => {
                return Err(refuse("a pop that popped names the value it returned"));
            }
            (CallWire::Pop, OutcomeWire::Empty, None) => (Call::Pop, Outcome::Empty),
            (CallWire::Pop, OutcomeWire::Empty, Some(_)) => {
                return Err(refuse("a pop that found the queue empty names no value"));
            }
            (CallWire::Pop, OutcomeWire::Pushed | OutcomeWire::Full, _) => {
                return Err(refuse("a pop is popped or empty, not pushed or full"));
            }
        };
        Ok(Operation {
            call,
            outcome,
            invoked: self.invoked,
            returned: self.returned,
        })
    }
}
