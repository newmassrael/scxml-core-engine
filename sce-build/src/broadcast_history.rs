// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A linearizability checker for broadcast-log histories (SCE Protocol-Synthesis
//! RFC §synth-5-S; ADR 0062 §5), and the one JSON format a history is written in.
//!
//! A history is what the participants of one run observed: the one publisher
//! and each reader, their operations in program order, each with the instants
//! it was invoked and returned, both read from one clock every participant
//! shares. The history is linearizable when its operations can be put in one
//! sequence that
//!
//! - keeps every operation that returned before another was invoked ahead of
//!   it, and
//! - is a run of the sequential bounded log of the history's retention
//!   (ADR 0062 §2), in which each operation gives the result it was observed
//!   to give.
//!
//! The sequential log has the state `h`, the number of entries published, and
//! the entries by sequence number; only the last `retention` are retained.
//! A reader owns a cursor, which an `open` sets and each `read` moves:
//!
//! - `item(seq, value)` needs the cursor at `seq`, `seq` in the window
//!   (`seq < h` and `h - seq <= retention`) and `value` to be the entry
//!   published at `seq`; the cursor moves to `seq + 1`;
//! - `empty` needs the cursor at `h`;
//! - `lagged(skipped)` needs the cursor below the window and `skipped` to be
//!   the distance to its start, `(h - retention) - cursor`; the cursor moves
//!   to the start of the window.
//!
//! There are two checkers, and the first is the definition of the second's
//! answer.
//!
//! - [`check_general`] searches for that sequence with the search of
//!   [`crate::queue_history`]: Wing and Gong's, with the memo of states already
//!   explored that Lowe added. Here the state is only how many operations of
//!   each participant are placed, because `h` is the number of publishes among
//!   the publisher's, and a reader's cursor is a function of the readings it
//!   recorded. It is exponential in the number of operations in the worst case.
//! - [`check_specialised`], in [`specialised`], decides the same question
//!   without a search, in `O(n log n)`, by a theorem proved in that module's
//!   documentation: with one publisher, `h` is a step function of time and each
//!   other operation constrains it at one instant of its own interval. It
//!   declines a history outside the theorem's hypothesis, with a typed reason,
//!   and never guesses.
//!
//! [`check`] is what every caller uses: it asks the specialised checker, and
//! the general search only when that declines, so its verdict class
//! (linearizable, not linearizable, malformed) is the general search's on every
//! history. A [`Verdict::NotLinearizable`] that the specialised checker gave
//! has `explored` 0, since it explored no state, and `deepest` as
//! [`Unplaceable::deepest`] defines it. [`judge`] says in addition which method
//! decided and, for the specialised one, why an operation could not be placed.
//! `tests/broadcast_history_differential.rs` holds the two equal on generated,
//! mutated and shrunk histories.
//!
//! Nothing here knows an implementation or a language: the stress run of each
//! backend writes the JSON form ([`History::to_json`], read by
//! [`History::from_json`], described by
//! `tests/forge/conformance/broadcast_history.schema.json`), and
//! `sce-codegen check-broadcast-history` judges the files.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::history_common::{check_clock, may_come_next, HistoryError, Timed, Verdict};

pub mod specialised;

pub use specialised::{check as check_specialised, Decision, Declined, Obstruction, Unplaceable};

/// Where a reader's cursor starts (ADR 0062 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFrom {
    /// The oldest retained entry: `max(0, h - retention)`.
    Oldest,
    /// The next entry to be published: `h`.
    Head,
    /// This sequence number, no further than `h`: `min(n, h)`.
    At(u64),
}

/// What an operation asked of the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    /// Publish this value. Only the publisher publishes.
    Publish(u64),
    /// Open a reader at a position. Only a reader opens.
    Open(OpenFrom),
    /// Read at the cursor. Only a reader reads.
    Read,
    /// Read the number of entries published. Anyone may.
    Head,
}

/// What an operation was observed to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The publish took the sequence number `seq`.
    Published(u64),
    /// The reader's cursor was set to this.
    Opened(u64),
    /// The read returned the entry `seq`, which held `value`.
    Item { seq: u64, value: u64 },
    /// The read found the cursor at the head.
    Empty,
    /// The read found the cursor below the window, `skipped` entries short
    /// of its start.
    Lagged { skipped: u64 },
    /// The head was this.
    Head(u64),
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

impl Timed for Operation {
    fn invoked(&self) -> u64 {
        self.invoked
    }

    fn returned(&self) -> u64 {
        self.returned
    }
}

/// Everything one run observed.
#[derive(Debug, Clone)]
pub struct History {
    /// The retention of the sequential log the run is judged against: the
    /// number of entries a reader may rely on.
    pub retention: u64,
    /// The one publisher's operations, in its program order.
    pub publisher: Vec<Operation>,
    /// One list per reader, in that reader's program order.
    pub readers: Vec<Vec<Operation>>,
}

impl History {
    /// The participants as the search counts them: the publisher first.
    fn participants(&self) -> Vec<Vec<Operation>> {
        std::iter::once(self.publisher.clone())
            .chain(self.readers.iter().cloned())
            .collect()
    }
}

/// Which checker to ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// The specialised checker, and the general search when that declines.
    Auto,
    /// The general search alone: the definition.
    General,
    /// The specialised checker alone: a history it declines is an `Err`.
    Specialised,
}

/// Which checker decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Specialised,
    General,
}

/// A verdict, and how it was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judgement {
    pub verdict: Verdict,
    pub method: Method,
    /// When the specialised checker found the history not linearizable: the
    /// operation it could not place and why. The general search says only how
    /// far it got, in the verdict.
    pub unplaceable: Option<Unplaceable>,
}

/// Judge `history` with the checker `ask` names.
///
/// Only `Ask::Specialised` can fail, with the reason the specialised checker
/// declined the history; `Ask::Auto` goes on to the general search instead.
pub fn judge(history: &History, ask: Ask) -> Result<Judgement, Declined> {
    match ask {
        Ask::General => Ok(general_judgement(history)),
        Ask::Specialised => specialised_judgement(history),
        Ask::Auto => Ok(automatic_judgement(history)),
    }
}

/// The specialised checker's judgement, or the general search's when the
/// specialised checker declines.
fn automatic_judgement(history: &History) -> Judgement {
    specialised_judgement(history).unwrap_or_else(|_| general_judgement(history))
}

fn general_judgement(history: &History) -> Judgement {
    Judgement {
        verdict: check_general(history),
        method: Method::General,
        unplaceable: None,
    }
}

fn specialised_judgement(history: &History) -> Result<Judgement, Declined> {
    match check_specialised(history) {
        Decision::Linearizable => Ok(Judgement {
            verdict: Verdict::Linearizable,
            method: Method::Specialised,
            unplaceable: None,
        }),
        Decision::NotLinearizable(unplaceable) => Ok(Judgement {
            verdict: Verdict::NotLinearizable {
                explored: 0,
                deepest: unplaceable.deepest.clone(),
            },
            method: Method::Specialised,
            unplaceable: Some(unplaceable),
        }),
        Decision::Declined(declined) => Err(declined),
    }
}

/// Judge `history`: by the specialised checker, and by the general search when
/// that declines. The verdict class is the general search's on every history,
/// which `tests/broadcast_history_differential.rs` holds and the theorem in
/// [`specialised`] proves.
pub fn check(history: &History) -> Verdict {
    automatic_judgement(history).verdict
}

/// Judge `history` by the general search alone, the definition of what is
/// linearizable here.
pub fn check_general(history: &History) -> Verdict {
    let participants = history.participants();
    if let Err(why) = well_formed(history, &participants) {
        return Verdict::Malformed(why);
    }
    let sequential = Sequential::of(history.retention, &participants);

    let start = vec![0; participants.len()];
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    seen.insert(start.clone());
    let mut deepest = start.clone();
    let mut stack = vec![Frame {
        frontier: start,
        next_choice: 0,
    }];

    while let Some(frame) = stack.last_mut() {
        if frame
            .frontier
            .iter()
            .zip(&participants)
            .all(|(placed, ops)| *placed == ops.len())
        {
            return Verdict::Linearizable;
        }

        let mut successor = None;
        while frame.next_choice < participants.len() {
            let who = frame.next_choice;
            frame.next_choice += 1;
            let Some(op) = participants[who].get(frame.frontier[who]) else {
                continue;
            };
            if !may_come_next(&participants, &frame.frontier, who, op) {
                continue;
            }
            let h = sequential.published[frame.frontier[0]];
            if !sequential.admits(who, frame.frontier[who], op, h) {
                continue;
            }
            let mut next = frame.frontier.clone();
            next[who] += 1;
            if seen.insert(next.clone()) {
                successor = Some(next);
                break;
            }
        }

        match successor {
            Some(frontier) => {
                if frontier.iter().sum::<usize>() > deepest.iter().sum::<usize>() {
                    deepest = frontier.clone();
                }
                stack.push(Frame {
                    frontier,
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

struct Frame {
    /// How many of each participant's operations are placed.
    frontier: Vec<usize>,
    /// The participant whose next operation the search tries next from here.
    next_choice: usize,
}

/// What the sequential log needs of a history, read off it once: the values
/// published by sequence number, how many entries are published after each
/// prefix of the publisher's operations, and the cursor each reader holds
/// before each of its operations.
struct Sequential {
    retention: u64,
    values: Vec<u64>,
    /// `published[k]`: the publishes among the publisher's first `k` operations.
    published: Vec<u64>,
    /// `cursor[reader][at]`: the cursor before the reader's operation `at`,
    /// `None` before the reader has been opened. Participant 0, the
    /// publisher, has none.
    cursor: Vec<Vec<Option<u64>>>,
}

impl Sequential {
    fn of(retention: u64, participants: &[Vec<Operation>]) -> Sequential {
        let mut values = Vec::new();
        let mut published = vec![0u64];
        for op in &participants[0] {
            if let Call::Publish(value) = op.call {
                values.push(value);
            }
            published.push(values.len() as u64);
        }
        let mut cursor = vec![Vec::new()];
        for ops in &participants[1..] {
            let mut held = None;
            let mut before = Vec::with_capacity(ops.len());
            for op in ops {
                before.push(held);
                held = match (op.call, op.outcome, held) {
                    (_, Outcome::Opened(at), _) => Some(at),
                    (_, Outcome::Item { seq, .. }, _) => Some(seq.saturating_add(1)),
                    (_, Outcome::Lagged { skipped }, Some(at)) => Some(at.saturating_add(skipped)),
                    (_, _, held) => held,
                };
            }
            cursor.push(before);
        }
        Sequential {
            retention,
            values,
            published,
            cursor,
        }
    }

    /// Whether the sequential log with `h` entries published gives `op`, the
    /// operation `at` of participant `who`, the result it was observed to give.
    fn admits(&self, who: usize, at: usize, op: &Operation, h: u64) -> bool {
        match (op.call, op.outcome) {
            // The publisher's own count already fixed the sequence number.
            (Call::Publish(_), Outcome::Published(_)) => true,
            (Call::Head, Outcome::Head(seen)) => seen == h,
            (Call::Open(from), Outcome::Opened(cursor)) => {
                cursor
                    == match from {
                        OpenFrom::Oldest => h.saturating_sub(self.retention),
                        OpenFrom::Head => h,
                        OpenFrom::At(n) => n.min(h),
                    }
            }
            (Call::Read, outcome) => {
                let Some(cursor) = self.cursor[who][at] else {
                    return false;
                };
                match outcome {
                    Outcome::Item { seq, value } => {
                        cursor == seq
                            && seq < h
                            && h - seq <= self.retention
                            && usize::try_from(seq)
                                .ok()
                                .and_then(|seq| self.values.get(seq))
                                == Some(&value)
                    }
                    Outcome::Empty => cursor == h,
                    Outcome::Lagged { skipped } => {
                        h > self.retention
                            && cursor < h - self.retention
                            && skipped == (h - self.retention) - cursor
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

fn well_formed(history: &History, participants: &[Vec<Operation>]) -> Result<(), String> {
    if history.retention == 0 {
        return Err("the retention is 0, and a log retains at least one entry".to_string());
    }
    check_clock(participants)?;
    let mut published = 0u64;
    for (at, op) in history.publisher.iter().enumerate() {
        match (op.call, op.outcome) {
            (Call::Publish(_), Outcome::Published(seq)) => {
                if seq != published {
                    return Err(format!(
                        "the publisher's operation {at} took sequence number {seq} but is the \
                         publish number {published}"
                    ));
                }
                published += 1;
            }
            (Call::Head, Outcome::Head(_)) => {}
            _ => {
                return Err(format!(
                    "the publisher's operation {at} is a {:?} that returned {:?}; the publisher \
                     publishes and reads the head",
                    op.call, op.outcome
                ));
            }
        }
    }
    for (who, ops) in history.readers.iter().enumerate() {
        let mut opened = false;
        for (at, op) in ops.iter().enumerate() {
            match (op.call, op.outcome) {
                (Call::Open(_), Outcome::Opened(_)) => opened = true,
                (Call::Head, Outcome::Head(_)) => {}
                (Call::Read, Outcome::Item { .. } | Outcome::Empty | Outcome::Lagged { .. }) => {
                    if !opened {
                        return Err(format!(
                            "reader {who} operation {at} reads before the reader was opened"
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "reader {who} operation {at} is a {:?} that returned {:?}; a reader \
                         opens, reads and reads the head",
                        op.call, op.outcome
                    ));
                }
            }
        }
    }
    Ok(())
}

// ─────────────────────────────── The JSON form ───────────────────────────────

/// The version of the JSON form this module reads and writes. A reader that
/// meets another refuses the file instead of guessing at it.
pub const HISTORY_FORMAT_VERSION: u64 = 1;

/// The document as it sits in a file. Unknown fields are refused, so a field
/// a writer adds that this reader would drop is an error and not a silent
/// loss of what the writer meant.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryDocument {
    version: u64,
    retention: u64,
    publisher: Vec<OperationWire>,
    readers: Vec<Vec<OperationWire>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CallWire {
    Publish,
    Open,
    Read,
    Head,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OpenFromWire {
    Oldest,
    Head,
    At,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReadOutcomeWire {
    Item,
    Empty,
    Lagged,
}

/// One operation. Which of the optional fields it names depends on the call,
/// and [`OperationWire::observed`] refuses a field the call does not take.
#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct OperationWire {
    call: CallWire,
    /// A read: what it returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outcome: Option<ReadOutcomeWire>,
    /// A publish: the value published. An item: the value read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<u64>,
    /// A publish: the sequence number it took. An item: the entry read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
    /// A lagged read: the entries it was short of the window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    skipped: Option<u64>,
    /// An open: where the cursor was asked to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from: Option<OpenFromWire>,
    /// An open from `at`: the sequence number asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    at: Option<u64>,
    /// An open: the cursor it set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cursor: Option<u64>,
    /// A head: the number it returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    head: Option<u64>,
    invoked: u64,
    returned: u64,
}

impl History {
    /// Read a history from its JSON form.
    ///
    /// Refuses a document that is not JSON, names another version, has a field
    /// this format does not, or states an operation that cannot be one: a call
    /// without a field it takes, or with one it does not. What this checks is
    /// the shape; whether the times are those of a run is [`check`]'s
    /// `Malformed`.
    pub fn from_json(text: &str) -> Result<History, HistoryError> {
        let document: HistoryDocument = serde_json::from_str(text)
            .map_err(|e| HistoryError(format!("not a broadcast-log history: {e}")))?;
        if document.version != HISTORY_FORMAT_VERSION {
            return Err(HistoryError(format!(
                "version {} is not the version this reader reads ({HISTORY_FORMAT_VERSION})",
                document.version
            )));
        }
        let observe = |list: &str, operations: &[OperationWire]| {
            operations
                .iter()
                .enumerate()
                .map(|(at, operation)| operation.observed(&format!("{list}[{at}]")))
                .collect::<Result<Vec<_>, _>>()
        };
        let publisher = observe("publisher", &document.publisher)?;
        let mut readers = Vec::with_capacity(document.readers.len());
        for (who, operations) in document.readers.iter().enumerate() {
            readers.push(observe(&format!("readers[{who}]"), operations)?);
        }
        Ok(History {
            retention: document.retention,
            publisher,
            readers,
        })
    }

    /// Write the history in its JSON form, as [`History::from_json`] reads it.
    pub fn to_json(&self) -> String {
        let document = HistoryDocument {
            version: HISTORY_FORMAT_VERSION,
            retention: self.retention,
            publisher: self.publisher.iter().map(OperationWire::of).collect(),
            readers: self
                .readers
                .iter()
                .map(|operations| operations.iter().map(OperationWire::of).collect())
                .collect(),
        };
        serde_json::to_string_pretty(&document).expect("a history serialises")
    }
}

impl OperationWire {
    fn bare(call: CallWire, operation: &Operation) -> OperationWire {
        OperationWire {
            call,
            outcome: None,
            value: None,
            seq: None,
            skipped: None,
            from: None,
            at: None,
            cursor: None,
            head: None,
            invoked: operation.invoked,
            returned: operation.returned,
        }
    }

    fn of(operation: &Operation) -> OperationWire {
        match (operation.call, operation.outcome) {
            (Call::Publish(value), Outcome::Published(seq)) => OperationWire {
                value: Some(value),
                seq: Some(seq),
                ..OperationWire::bare(CallWire::Publish, operation)
            },
            (Call::Open(from), Outcome::Opened(cursor)) => OperationWire {
                from: Some(match from {
                    OpenFrom::Oldest => OpenFromWire::Oldest,
                    OpenFrom::Head => OpenFromWire::Head,
                    OpenFrom::At(_) => OpenFromWire::At,
                }),
                at: match from {
                    OpenFrom::At(n) => Some(n),
                    _ => None,
                },
                cursor: Some(cursor),
                ..OperationWire::bare(CallWire::Open, operation)
            },
            (Call::Head, Outcome::Head(head)) => OperationWire {
                head: Some(head),
                ..OperationWire::bare(CallWire::Head, operation)
            },
            (_, Outcome::Item { seq, value }) => OperationWire {
                outcome: Some(ReadOutcomeWire::Item),
                seq: Some(seq),
                value: Some(value),
                ..OperationWire::bare(CallWire::Read, operation)
            },
            (_, Outcome::Lagged { skipped }) => OperationWire {
                outcome: Some(ReadOutcomeWire::Lagged),
                skipped: Some(skipped),
                ..OperationWire::bare(CallWire::Read, operation)
            },
            // A pairing of call and outcome no run produces is written as the
            // read that found nothing, which the checker then refuses as
            // malformed when it is read back and judged.
            _ => OperationWire {
                outcome: Some(ReadOutcomeWire::Empty),
                ..OperationWire::bare(CallWire::Read, operation)
            },
        }
    }

    fn observed(&self, place: &str) -> Result<Operation, HistoryError> {
        let refuse = |reason: &str| HistoryError(format!("{place}: {reason}"));
        // Every field the call takes is named and every other is absent.
        let takes = |names: &[&str]| -> Result<(), HistoryError> {
            let present = [
                ("outcome", self.outcome.is_some()),
                ("value", self.value.is_some()),
                ("seq", self.seq.is_some()),
                ("skipped", self.skipped.is_some()),
                ("from", self.from.is_some()),
                ("at", self.at.is_some()),
                ("cursor", self.cursor.is_some()),
                ("head", self.head.is_some()),
            ];
            for (name, is_present) in present {
                if names.contains(&name) && !is_present {
                    return Err(refuse(&format!("names no `{name}`, which this call takes")));
                }
                if !names.contains(&name) && is_present {
                    return Err(refuse(&format!(
                        "names `{name}`, which this call does not take"
                    )));
                }
            }
            Ok(())
        };
        let (call, outcome) = match self.call {
            CallWire::Publish => {
                takes(&["value", "seq"])?;
                (
                    Call::Publish(self.value.expect("checked")),
                    Outcome::Published(self.seq.expect("checked")),
                )
            }
            CallWire::Head => {
                takes(&["head"])?;
                (Call::Head, Outcome::Head(self.head.expect("checked")))
            }
            CallWire::Open => {
                let from = match self.from {
                    Some(OpenFromWire::Oldest) => {
                        takes(&["from", "cursor"])?;
                        OpenFrom::Oldest
                    }
                    Some(OpenFromWire::Head) => {
                        takes(&["from", "cursor"])?;
                        OpenFrom::Head
                    }
                    Some(OpenFromWire::At) => {
                        takes(&["from", "at", "cursor"])?;
                        OpenFrom::At(self.at.expect("checked"))
                    }
                    None => return Err(refuse("an open names where the cursor starts (`from`)")),
                };
                (
                    Call::Open(from),
                    Outcome::Opened(self.cursor.expect("checked")),
                )
            }
            CallWire::Read => match self.outcome {
                Some(ReadOutcomeWire::Item) => {
                    takes(&["outcome", "seq", "value"])?;
                    (
                        Call::Read,
                        Outcome::Item {
                            seq: self.seq.expect("checked"),
                            value: self.value.expect("checked"),
                        },
                    )
                }
                Some(ReadOutcomeWire::Empty) => {
                    takes(&["outcome"])?;
                    (Call::Read, Outcome::Empty)
                }
                Some(ReadOutcomeWire::Lagged) => {
                    takes(&["outcome", "skipped"])?;
                    (
                        Call::Read,
                        Outcome::Lagged {
                            skipped: self.skipped.expect("checked"),
                        },
                    )
                }
                None => {
                    return Err(refuse("a read is an item, empty or lagged (`outcome`)"));
                }
            },
        };
        Ok(Operation {
            call,
            outcome,
            invoked: self.invoked,
            returned: self.returned,
        })
    }
}
