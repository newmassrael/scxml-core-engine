// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the history checkers of the concurrent kinds share (SCE
//! Protocol-Synthesis RFC §synth-5-P, verification layer 2; ADR 0062 §7).
//!
//! A history is what the participants of one run observed: for each, its
//! operations in program order, each with the instants it was invoked and
//! returned, both read from one clock every participant shares. Judging one
//! is a search for a sequence of the operations that keeps the real-time
//! order and is a run of the kind's sequential specification (Herlihy and
//! Wing, "Linearizability: A Correctness Condition for Concurrent Objects",
//! TOPLAS 12(3), 1990; Wing and Gong, JPDC 17(1-2), 1993). The
//! specification differs by kind. The clock model, the real-time order, the
//! answer of the search and the error of a file that is not a history do not,
//! and live here once.

/// An observed operation of any kind: the two clock readings around it.
pub trait Timed {
    /// Clock reading taken immediately before the call.
    fn invoked(&self) -> u64;
    /// Clock reading taken immediately after it returned.
    fn returned(&self) -> u64;
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

/// Why a file is not a history of a kind's format: the place in the document
/// and what is wrong there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryError(pub String);

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HistoryError {}

/// Whether the clock readings can be those of a run: each operation returned
/// after it was invoked, and a participant's next operation was invoked after
/// its previous one returned.
pub fn check_clock<T: Timed>(participants: &[Vec<T>]) -> Result<(), String> {
    for (who, ops) in participants.iter().enumerate() {
        for (at, op) in ops.iter().enumerate() {
            if op.returned() <= op.invoked() {
                return Err(format!(
                    "participant {who} operation {at} returned at {} but was invoked at {}",
                    op.returned(),
                    op.invoked()
                ));
            }
            if let Some(previous) = at.checked_sub(1).map(|p| &ops[p]) {
                if op.invoked() <= previous.returned() {
                    return Err(format!(
                        "participant {who} operation {at} was invoked at {} before its previous \
                         operation returned at {}",
                        op.invoked(),
                        previous.returned()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Whether `op`, the next unplaced operation of participant `who`, may be
/// placed next: no unplaced operation may have returned before it was
/// invoked. A participant's operations return in program order, so the
/// earliest-returning unplaced one of each is the one at its frontier.
pub fn may_come_next<T: Timed>(
    participants: &[Vec<T>],
    frontier: &[usize],
    who: usize,
    op: &T,
) -> bool {
    participants.iter().enumerate().all(|(other, ops)| {
        other == who
            || match ops.get(frontier[other]) {
                None => true,
                Some(first) => first.returned() > op.invoked(),
            }
    })
}
