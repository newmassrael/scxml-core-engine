// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A linearizability checker for inbox histories (ADR 0147, D3), and the one
//! JSON format a history is written in.
//!
//! ADR 0084's door: every input from another thread enters a session through
//! that session's inbox, and the session's owner thread moves what the inbox
//! holds into the session's external queue. A history is what the threads of
//! one run observed: each poster's operations and the owner's, in program
//! order, each with the instants it was invoked and returned, both read from
//! one clock every thread shares.
//!
//! - A poster's `post(session, input)`, through one of two doors ([`Door`]):
//!   the tree's directory, by the session's id, or the handle of the
//!   session's own inbox. It returned `taken`, or `refused` with its reason:
//!   `unknown`, `ended` or `full` ([`Refusal`]).
//! - The owner's `enqueue(session, item, source)` is recorded for every item
//!   that entered a session's external queue: an input a post took, applied
//!   from the inbox (`posted`), or an event of the owner's own (`own`).
//! - The owner's `end(session)` is the end of that session's run.
//!
//! Beside the operations, a history says what the run was given: the sessions
//! the tree holds ([`History::sessions`]), whether the runtime closes a
//! session's inbox when its run ends ([`History::closes`]), and the capacity
//! of a bounded inbox ([`History::capacity`]).
//!
//! The sequential specification is `docs/formal/InboxService.tla` (ADR 0147,
//! D1): a post is either taken, and its input then enters the queue of the
//! session it names exactly once, in the order the posts to that session and
//! its own events linearized, or refused when it is posted, and its input never
//! enters anything. As a sequential object, each session holds the inputs
//! taken and not yet entered, in the order they were taken:
//!
//! - a post through the directory naming a session the tree does not hold is
//!   refused as `unknown`, and so may be one naming a session whose run has
//!   ended, which leaves the directory then, whether the runtime closes or
//!   not; a post through a handle is never `unknown`, and names a session
//!   the tree holds;
//! - a post naming a session whose run has ended, where the runtime closes, is
//!   refused as `ended`, through either door;
//! - a post to a bounded inbox may be refused as `full` when its session holds
//!   as many inputs as the capacity, and never when it holds fewer;
//! - any other post is taken, and its input joins the back of what its session
//!   holds. A post may be taken when its session holds the capacity: a drain
//!   takes inputs out of the inbox before it enters them, and the room it
//!   frees is not seen by an object that counts what has not entered. That an
//!   inbox never holds more than its capacity is the mechanism's
//!   (`Bounded` in `docs/formal/EngineInbox.tla`), and no poster is told
//!   anything false past it;
//! - an `enqueue` of a posted input needs that input at the front of what its
//!   session holds, and takes it off;
//! - an `enqueue` of an own event is where that event linearizes, so it needs
//!   its session not ended and holding nothing: an own event enters behind
//!   every post taken before it (ADR 0084, D2 as amended at step 9);
//! - an own event enters once, and a session ends once;
//! - once every operation is placed, no session holds anything, so a history
//!   is that of a whole run, whose every taken input has entered.
//!
//! The history is linearizable when its operations can be put in one sequence
//! that keeps every operation that returned before another was invoked ahead
//! of it and is a run of that object. [`check`] searches for it with the
//! search of [`crate::queue_history`] (Wing and Gong's, with the memo of states
//! already explored that Lowe added), and says which operations it could not
//! get past when there is none. Only the specification differs.
//!
//! Each part of the mechanism `docs/formal/EngineInbox.tla` takes out leaves a
//! history this checker refuses: an own event entered before a post that
//! returned before it (the drain before an own event, or the address resolved
//! when posted), two inputs entered in the reverse of their posts (one drain at
//! a time), an input entered twice (a drain takes out what it applies), an
//! input taken and never entered (what an inbox holds at the end is applied),
//! and a post naming a session the tree does not hold taken (the directory
//! looked up when posted). The one part whose absence it cannot see is a full
//! inbox's refusal, whose absence tells no poster anything false.
//!
//! Nothing here knows an implementation or a language: each runtime's inbox
//! conformance test writes the JSON form ([`History::to_json`], read by
//! [`History::from_json`], described by
//! `tests/forge/conformance/inbox_history.schema.json`), and
//! `sce-codegen check-inbox-history` judges the files.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::history_common::{check_clock, may_come_next, Timed};
pub use crate::history_common::{HistoryError, Verdict};

/// What the inbox answered a post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delivery {
    /// The inbox took the input.
    Taken,
    /// The inbox handed the input back, for the reason given.
    Refused(Refusal),
}

/// Why the inbox handed a post back. A refused post never takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// The session's bounded inbox held its capacity.
    Full,
    /// The session's run had ended, and the runtime closes its inbox then.
    Ended,
    /// The tree does not hold the session the post named: it never did, or it
    /// let the session go when its run ended and the runtime closed its inbox.
    Unknown,
}

/// The door a post went through (docs/adr/0147, D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Door {
    /// The tree's directory, by the id of the session the post names. The
    /// directory holds the sessions of the tree, and a session leaves it when
    /// its run ends.
    Addressed,
    /// The handle of the session's own inbox, which names a session of the
    /// tree and looks nothing up.
    Handle,
}

/// Where an item that entered an external queue came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// An input a post took, applied from the session's inbox.
    Posted,
    /// An event the owner put on the session's own queue.
    Own,
}

/// One post a poster observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Post {
    /// The session the post named.
    pub session: u64,
    /// The input posted. The inputs of a run are distinct values.
    pub input: u64,
    /// The door the post went through.
    pub door: Door,
    /// What the inbox answered.
    pub outcome: Delivery,
    /// Clock reading taken immediately before the call.
    pub invoked: u64,
    /// Clock reading taken immediately after it returned.
    pub returned: u64,
}

/// What an operation of the owner did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerCall {
    /// `item` entered `session`'s external queue. A posted item is the input
    /// a post took; an own item is the owner's name for its event, and the
    /// own events of a run are distinct values.
    Enqueue {
        session: u64,
        item: u64,
        source: Source,
    },
    /// `session`'s run ended.
    End { session: u64 },
}

impl OwnerCall {
    /// The session the operation names.
    pub fn session(&self) -> u64 {
        match *self {
            OwnerCall::Enqueue { session, .. } | OwnerCall::End { session } => session,
        }
    }
}

/// One operation the owner observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerOperation {
    pub call: OwnerCall,
    /// Clock reading taken immediately before the call.
    pub invoked: u64,
    /// Clock reading taken immediately after it returned.
    pub returned: u64,
}

/// Everything one run observed, and what it was given.
#[derive(Debug, Clone)]
pub struct History {
    /// The sessions the tree holds for the run. A post naming any other is
    /// refused, and the owner runs no other.
    pub sessions: Vec<u64>,
    /// Whether the runtime refuses a post to a session whose run has ended.
    /// One that never closes takes it, and applies it at the end of the
    /// owner's loop.
    pub closes: bool,
    /// The most inputs a session's inbox holds, where the inbox is bounded;
    /// `None` for an inbox without a bound, which never refuses a post as
    /// full.
    pub capacity: Option<u64>,
    /// The owner's operations, in its program order.
    pub owner: Vec<OwnerOperation>,
    /// One list per poster, in that poster's program order.
    pub posters: Vec<Vec<Post>>,
}

/// An operation as the search counts it: the session named by its place among
/// the sessions of the history.
#[derive(Debug, Clone, Copy)]
enum Act {
    Post {
        session: usize,
        input: u64,
        door: Door,
        outcome: Delivery,
    },
    Enqueue {
        session: usize,
        item: u64,
        source: Source,
    },
    End {
        session: usize,
    },
}

#[derive(Debug, Clone, Copy)]
struct Step {
    act: Act,
    invoked: u64,
    returned: u64,
}

impl Timed for Step {
    fn invoked(&self) -> u64 {
        self.invoked
    }

    fn returned(&self) -> u64 {
        self.returned
    }
}

impl History {
    /// The operations as the search counts them, the owner's first, as
    /// participant 0, then each poster's; and every session the history
    /// names, held by the tree or not, in the order of the places the
    /// operations count them by.
    fn steps(&self) -> (Vec<Vec<Step>>, Vec<u64>) {
        let mut sessions: Vec<u64> = self
            .sessions
            .iter()
            .copied()
            .chain(self.owner.iter().map(|op| op.call.session()))
            .chain(self.posters.iter().flatten().map(|post| post.session))
            .collect();
        sessions.sort_unstable();
        sessions.dedup();
        let place = |session: u64| {
            sessions
                .binary_search(&session)
                .expect("every session the history names is collected")
        };
        let owner: Vec<Step> = self
            .owner
            .iter()
            .map(|op| Step {
                act: match op.call {
                    OwnerCall::Enqueue {
                        session,
                        item,
                        source,
                    } => Act::Enqueue {
                        session: place(session),
                        item,
                        source,
                    },
                    OwnerCall::End { session } => Act::End {
                        session: place(session),
                    },
                },
                invoked: op.invoked,
                returned: op.returned,
            })
            .collect();
        let mut participants = vec![owner];
        for posts in &self.posters {
            participants.push(
                posts
                    .iter()
                    .map(|post| Step {
                        act: Act::Post {
                            session: place(post.session),
                            input: post.input,
                            door: post.door,
                            outcome: post.outcome,
                        },
                        invoked: post.invoked,
                        returned: post.returned,
                    })
                    .collect(),
            );
        }
        (participants, sessions)
    }
}

/// Judge `history`.
pub fn check(history: &History) -> Verdict {
    let (participants, names) = history.steps();
    let known: Vec<bool> = names
        .iter()
        .map(|name| history.sessions.contains(name))
        .collect();
    if let Err(why) = well_formed(&participants, &names, &known) {
        return Verdict::Malformed(why);
    }
    let sequential = Sequential::of(history, &participants[0], known);
    let required = posts_each_post_must_follow(&participants);
    let start = State {
        frontier: vec![0; participants.len()],
        waiting: vec![VecDeque::new(); names.len()],
    };
    let mut seen: HashSet<State> = HashSet::new();
    seen.insert(start.clone());
    let mut deepest = start.frontier.clone();
    let mut stack = vec![Frame {
        state: start,
        next_choice: 0,
    }];

    while let Some(frame) = stack.last_mut() {
        // Every operation placed and nothing left waiting: a taken input that
        // never entered leaves this a dead end, which the loop below finds
        // with no operation left to place.
        if frame
            .state
            .frontier
            .iter()
            .zip(&participants)
            .all(|(placed, ops)| *placed == ops.len())
            && frame.state.waiting.iter().all(VecDeque::is_empty)
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
            if !may_come_next(&participants, &frame.state.frontier, who, op) {
                continue;
            }
            if !required[who][frame.state.frontier[who]]
                .iter()
                .zip(&frame.state.frontier)
                .all(|(needed, placed)| placed >= needed)
            {
                continue;
            }
            let Some(waiting) = sequential.admits(&frame.state, op) else {
                continue;
            };
            let mut frontier = frame.state.frontier.clone();
            frontier[who] += 1;
            let next = State { frontier, waiting };
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
/// placed, and what each session holds of the inputs taken and not yet
/// entered, in the order they were taken.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    frontier: Vec<usize>,
    waiting: Vec<VecDeque<u64>>,
}

struct Frame {
    state: State,
    /// The participant whose next operation the search tries next from here.
    next_choice: usize,
}

/// What the sequential object needs of a history, read off the owner's
/// operations once. Whether a session has ended, and whether an own event has
/// entered before, are functions of how many of the owner's operations are
/// placed, since only the owner enqueues and ends; so neither is part of the
/// search's state.
struct Sequential {
    closes: bool,
    /// The most inputs a session's inbox holds, if it is bounded.
    capacity: Option<usize>,
    /// `known[session]`: whether the tree holds the session.
    known: Vec<bool>,
    /// `end_at[session]`: the place among the owner's operations of the first
    /// end of the session, if it has one.
    end_at: Vec<Option<usize>>,
    /// `repeats[at]`: whether the owner's operation `at` is an own event
    /// that entered before, or the end of a session that ended before.
    repeats: Vec<bool>,
}

impl Sequential {
    fn of(history: &History, owner: &[Step], known: Vec<bool>) -> Sequential {
        let mut end_at = vec![None; known.len()];
        let mut own_entered: HashSet<u64> = HashSet::new();
        let repeats = owner
            .iter()
            .enumerate()
            .map(|(at, op)| match op.act {
                Act::Enqueue {
                    item,
                    source: Source::Own,
                    ..
                } => !own_entered.insert(item),
                Act::End { session } => {
                    let first = end_at[session].is_none();
                    if first {
                        end_at[session] = Some(at);
                    }
                    !first
                }
                Act::Enqueue {
                    source: Source::Posted,
                    ..
                }
                | Act::Post { .. } => false,
            })
            .collect();
        Sequential {
            closes: history.closes,
            // A capacity past what an index counts is one no session reaches.
            capacity: history
                .capacity
                .map(|capacity| usize::try_from(capacity).unwrap_or(usize::MAX)),
            known,
            end_at,
            repeats,
        }
    }

    /// Whether `session` has ended once the owner's first `owner_placed`
    /// operations are placed.
    fn ended(&self, session: usize, owner_placed: usize) -> bool {
        self.end_at[session].is_some_and(|at| at < owner_placed)
    }

    /// What the sessions hold after `op`, placed next from `state`, or `None`
    /// when the sequential object would not give the outcome `op` was
    /// observed to give. An operation of the owner is the owner's next, so its
    /// place among the owner's operations is `state.frontier[0]`.
    fn admits(&self, state: &State, op: &Step) -> Option<Vec<VecDeque<u64>>> {
        let owner_placed = state.frontier[0];
        match op.act {
            Act::Post {
                session,
                input,
                door,
                outcome,
            } => {
                let known = self.known[session];
                let ended = known && self.ended(session, owner_placed);
                // Where the runtime closes, every post to a session whose run
                // has ended is refused, through either door.
                let closed = ended && self.closes;
                // A session leaves the directory when its run ends, so a post
                // through the directory may find it gone, whether the runtime
                // closes or not; a handle looks nothing up.
                let gone = ended && door == Door::Addressed;
                let full = self
                    .capacity
                    .is_some_and(|capacity| state.waiting[session].len() >= capacity);
                let answered = match outcome {
                    Delivery::Taken => known && !closed,
                    Delivery::Refused(Refusal::Unknown) => !known || gone,
                    Delivery::Refused(Refusal::Ended) => closed,
                    Delivery::Refused(Refusal::Full) => known && !closed && full,
                };
                answered.then(|| {
                    let mut waiting = state.waiting.clone();
                    if outcome == Delivery::Taken {
                        waiting[session].push_back(input);
                    }
                    waiting
                })
            }
            Act::Enqueue {
                session,
                item,
                source: Source::Posted,
            } => {
                if state.waiting[session].front() != Some(&item) {
                    return None;
                }
                let mut waiting = state.waiting.clone();
                waiting[session].pop_front();
                Some(waiting)
            }
            Act::Enqueue {
                session,
                source: Source::Own,
                ..
            } => (!self.repeats[owner_placed]
                && !self.ended(session, owner_placed)
                && state.waiting[session].is_empty())
            .then(|| state.waiting.clone()),
            Act::End { .. } => (!self.repeats[owner_placed]).then(|| state.waiting.clone()),
        }
    }
}

/// For each taken post, how many of each participant's operations must already
/// be placed before it may be: the posts of every input that entered the
/// session's queue before this post's input did.
///
/// A session gives up the inputs it holds in the order it took them, and the
/// owner enqueues one after another, so when one taken input entered before
/// another, the post that took the first was placed before the post that took
/// the second, whatever the real-time order of the two posts. The search would
/// otherwise place overlapping posts in either order and learn that it chose
/// wrongly only when the enqueues are reached. Only an input that entered
/// once, as posted, the queue of the session it was posted to is used: any
/// other is refused whatever order its post is placed in, and the rule would
/// be a guess. Every entry is zero for an operation that is not such a post.
fn posts_each_post_must_follow(participants: &[Vec<Step>]) -> Vec<Vec<Vec<usize>>> {
    // Where each taken input was posted: its session and its place. The inputs
    // are distinct, which `well_formed` holds.
    let mut taken: HashMap<u64, (usize, usize, usize)> = HashMap::new();
    for (who, ops) in participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            if let Act::Post {
                session,
                input,
                outcome: Delivery::Taken,
                ..
            } = op.act
            {
                taken.insert(input, (session, who, at));
            }
        }
    }
    let mut entered: HashMap<u64, usize> = HashMap::new();
    for op in &participants[0] {
        if let Act::Enqueue {
            item,
            source: Source::Posted,
            ..
        } = op.act
        {
            *entered.entry(item).or_default() += 1;
        }
    }
    let mut needed: Vec<Vec<Vec<usize>>> = participants
        .iter()
        .map(|ops| vec![vec![0; participants.len()]; ops.len()])
        .collect();
    // Per session, the posts of the inputs that entered so far, as a frontier.
    let mut entered_before: HashMap<usize, Vec<usize>> = HashMap::new();
    for op in &participants[0] {
        let Act::Enqueue {
            session,
            item,
            source: Source::Posted,
        } = op.act
        else {
            continue;
        };
        let Some(&(posted_to, who, at)) = taken.get(&item) else {
            continue;
        };
        if posted_to != session || entered.get(&item) != Some(&1) {
            continue;
        }
        let so_far = entered_before
            .entry(session)
            .or_insert_with(|| vec![0; participants.len()]);
        needed[who][at].clone_from(so_far);
        so_far[who] = so_far[who].max(at + 1);
    }
    needed
}

/// Whether the history can be that of a run: the clock readings can be, no
/// input is posted twice, since the inputs of a run are distinct values
/// (`docs/formal/InboxService.tla` takes them so), the owner runs only
/// the sessions the tree holds, and a post through a handle names one of them,
/// since a handle is a session's own. Participant 0 is the owner and participant `n`
/// the poster `n - 1`, as the search counts them; `names[place]` is the
/// session counted at `place`, and `known[place]` whether the tree holds it.
fn well_formed(participants: &[Vec<Step>], names: &[u64], known: &[bool]) -> Result<(), String> {
    check_clock(participants)?;
    for (at, op) in participants[0].iter().enumerate() {
        let session = match op.act {
            Act::Enqueue { session, .. } | Act::End { session } => session,
            Act::Post { .. } => continue,
        };
        if !known[session] {
            return Err(format!(
                "participant 0 operation {at} is the owner's on session {}, which is not among \
                 the sessions the tree holds (`sessions`): the owner runs only those",
                names[session]
            ));
        }
    }
    for (who, ops) in participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            if let Act::Post {
                session,
                door: Door::Handle,
                ..
            } = op.act
            {
                if !known[session] {
                    return Err(format!(
                        "participant {who} operation {at} posts through the handle of session {}, \
                         which is not among the sessions the tree holds (`sessions`): a handle is \
                         a session's own",
                        names[session]
                    ));
                }
            }
        }
    }
    let mut posted: HashMap<u64, (usize, usize)> = HashMap::new();
    for (who, ops) in participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            let Act::Post { input, .. } = op.act else {
                continue;
            };
            if let Some((first_who, first_at)) = posted.insert(input, (who, at)) {
                return Err(format!(
                    "participant {who} operation {at} posts input {input}, which participant \
                     {first_who} operation {first_at} posted: the inputs of a run are distinct"
                ));
            }
        }
    }
    Ok(())
}

// ─────────────────────────────── The JSON form ───────────────────────────────

/// The version of the JSON form this module reads and writes. A reader that
/// meets another refuses the file instead of guessing at it.
pub const HISTORY_FORMAT_VERSION: u64 = 2;

/// The document as it sits in a file. Unknown fields are refused, so a field
/// a writer adds that this reader would drop is an error and not a silent
/// loss of what the writer meant.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryDocument {
    version: u64,
    sessions: Vec<u64>,
    closes: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capacity: Option<u64>,
    owner: Vec<OwnerOperationWire>,
    posters: Vec<Vec<PostWire>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum PosterCallWire {
    Post,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum DoorWire {
    Addressed,
    Handle,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum DeliveryWire {
    Taken,
    Refused,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RefusalWire {
    Full,
    Ended,
    Unknown,
}

/// One post, and the door it went through. `reason` is a refused post's, and
/// a taken post names none.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct PostWire {
    call: PosterCallWire,
    session: u64,
    input: u64,
    door: DoorWire,
    outcome: DeliveryWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<RefusalWire>,
    invoked: u64,
    returned: u64,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OwnerCallWire {
    Enqueue,
    End,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SourceWire {
    Posted,
    Own,
}

/// One operation of the owner. `item` and `source` are an enqueue's, and an
/// end names neither.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct OwnerOperationWire {
    call: OwnerCallWire,
    session: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    item: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<SourceWire>,
    invoked: u64,
    returned: u64,
}

impl History {
    /// Read a history from its JSON form.
    ///
    /// Refuses a document that is not JSON, names another version, has a field
    /// this format does not, lists no session or one twice, gives a capacity
    /// of 0, puts a post among the owner's operations or an enqueue or an end
    /// among a poster's, or states an operation that cannot be one: an enqueue
    /// without its item or its source, an end with either, a refused post
    /// without its reason or a taken one with a reason. What this checks is the
    /// shape; whether the times are those of a run, the inputs distinct and the
    /// owner's sessions the tree's, is [`check`]'s `Malformed`.
    pub fn from_json(text: &str) -> Result<History, HistoryError> {
        let document: HistoryDocument = serde_json::from_str(text)
            .map_err(|e| HistoryError(format!("not an inbox history: {e}")))?;
        if document.version != HISTORY_FORMAT_VERSION {
            return Err(HistoryError(format!(
                "version {} is not the version this reader reads ({HISTORY_FORMAT_VERSION})",
                document.version
            )));
        }
        if document.sessions.is_empty() {
            return Err(HistoryError(
                "`sessions` lists no session, and a tree holds its root at least".to_string(),
            ));
        }
        for (at, session) in document.sessions.iter().enumerate() {
            if document.sessions[..at].contains(session) {
                return Err(HistoryError(format!(
                    "`sessions` lists session {session} twice"
                )));
            }
        }
        if document.capacity == Some(0) {
            return Err(HistoryError(
                "`capacity` is 0, and a bounded inbox holds one input at least; an inbox \
                 without a bound leaves `capacity` out"
                    .to_string(),
            ));
        }
        let mut owner = Vec::with_capacity(document.owner.len());
        for (at, operation) in document.owner.iter().enumerate() {
            owner.push(operation.observed(&format!("owner[{at}]"))?);
        }
        let mut posters = Vec::with_capacity(document.posters.len());
        for (who, posts) in document.posters.iter().enumerate() {
            let mut observed = Vec::with_capacity(posts.len());
            for (at, post) in posts.iter().enumerate() {
                observed.push(post.observed(&format!("posters[{who}][{at}]"))?);
            }
            posters.push(observed);
        }
        Ok(History {
            sessions: document.sessions,
            closes: document.closes,
            capacity: document.capacity,
            owner,
            posters,
        })
    }

    /// Write the history in its JSON form, as [`History::from_json`] reads it.
    pub fn to_json(&self) -> String {
        let document = HistoryDocument {
            version: HISTORY_FORMAT_VERSION,
            sessions: self.sessions.clone(),
            closes: self.closes,
            capacity: self.capacity,
            owner: self.owner.iter().map(OwnerOperationWire::of).collect(),
            posters: self
                .posters
                .iter()
                .map(|posts| posts.iter().map(PostWire::of).collect())
                .collect(),
        };
        serde_json::to_string_pretty(&document).expect("a history serialises")
    }
}

impl PostWire {
    fn of(post: &Post) -> PostWire {
        let (outcome, reason) = match post.outcome {
            Delivery::Taken => (DeliveryWire::Taken, None),
            Delivery::Refused(refusal) => (
                DeliveryWire::Refused,
                Some(match refusal {
                    Refusal::Full => RefusalWire::Full,
                    Refusal::Ended => RefusalWire::Ended,
                    Refusal::Unknown => RefusalWire::Unknown,
                }),
            ),
        };
        PostWire {
            call: PosterCallWire::Post,
            session: post.session,
            input: post.input,
            door: match post.door {
                Door::Addressed => DoorWire::Addressed,
                Door::Handle => DoorWire::Handle,
            },
            outcome,
            reason,
            invoked: post.invoked,
            returned: post.returned,
        }
    }

    fn observed(&self, place: &str) -> Result<Post, HistoryError> {
        let outcome = match (self.outcome, self.reason) {
            (DeliveryWire::Taken, None) => Delivery::Taken,
            (DeliveryWire::Refused, Some(reason)) => Delivery::Refused(match reason {
                RefusalWire::Full => Refusal::Full,
                RefusalWire::Ended => Refusal::Ended,
                RefusalWire::Unknown => Refusal::Unknown,
            }),
            (DeliveryWire::Taken, Some(_)) => {
                return Err(HistoryError(format!(
                    "{place}: a taken post gives no `reason`"
                )));
            }
            (DeliveryWire::Refused, None) => {
                return Err(HistoryError(format!(
                    "{place}: a refused post says why it was refused (`reason`)"
                )));
            }
        };
        Ok(Post {
            session: self.session,
            input: self.input,
            door: match self.door {
                DoorWire::Addressed => Door::Addressed,
                DoorWire::Handle => Door::Handle,
            },
            outcome,
            invoked: self.invoked,
            returned: self.returned,
        })
    }
}

impl OwnerOperationWire {
    fn of(operation: &OwnerOperation) -> OwnerOperationWire {
        let (call, item, source) = match operation.call {
            OwnerCall::Enqueue { item, source, .. } => (
                OwnerCallWire::Enqueue,
                Some(item),
                Some(match source {
                    Source::Posted => SourceWire::Posted,
                    Source::Own => SourceWire::Own,
                }),
            ),
            OwnerCall::End { .. } => (OwnerCallWire::End, None, None),
        };
        OwnerOperationWire {
            call,
            session: operation.call.session(),
            item,
            source,
            invoked: operation.invoked,
            returned: operation.returned,
        }
    }

    fn observed(&self, place: &str) -> Result<OwnerOperation, HistoryError> {
        let refuse = |reason: &str| HistoryError(format!("{place}: {reason}"));
        let call = match (self.call, self.item, self.source) {
            (OwnerCallWire::Enqueue, Some(item), Some(source)) => OwnerCall::Enqueue {
                session: self.session,
                item,
                source: match source {
                    SourceWire::Posted => Source::Posted,
                    SourceWire::Own => Source::Own,
                },
            },
            (OwnerCallWire::Enqueue, None, _) => {
                return Err(refuse("an enqueue names the item that entered (`item`)"));
            }
            (OwnerCallWire::Enqueue, Some(_), None) => {
                return Err(refuse(
                    "an enqueue names where its item came from (`source`)",
                ));
            }
            (OwnerCallWire::End, None, None) => OwnerCall::End {
                session: self.session,
            },
            (OwnerCallWire::End, _, _) => {
                return Err(refuse(
                    "an end names its session only, no `item` or `source`",
                ));
            }
        };
        Ok(OwnerOperation {
            call,
            invoked: self.invoked,
            returned: self.returned,
        })
    }
}
