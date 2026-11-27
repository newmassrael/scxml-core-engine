// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What one target's outbound side decides (SCE_MESH.md section 10.10): whether an envelope is
//! sent at once, waits for the target to become ready, or is dropped and told; that a target which
//! stops being ready is told once, on that edge; and how the failed sends of one envelope are
//! counted and what its next wait grows from (section 16.7, rows 2 and 3). `Outbound` and
//! `Attempts::after_failure` hold the bytes, the clock and the policy, and decide none of this.
//! These rules are here by themselves, over counts and verdicts alone, and proved by Verus
//! (docs/adr/0115, D13): no envelope is sent ahead of one admitted before it that still waits, no
//! admit puts an envelope behind as many as the bound, every envelope admitted is sent, waits or is
//! told, a lost readiness is told once per edge, and no count wraps.
//!
//! Each rule of `sce:std/mesh` (`outbound_sends_now`, `outbound_overflows`, `outbound_stale` and
//! the retry rules) stays the one definition of its decision, generated into this crate: here its
//! verdict is an argument, and a proof below takes of a rule what its document says it computes. A
//! rule's own contract is judged where it is generated (docs/adr/0115, D10).
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes
//! of this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// Whether a target is ready: something said it is, and nothing since said it stopped.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gate {
    ready: bool,
}

/// What becomes of an envelope admitted to a target.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// Sent at once.
    Send,
    /// Put behind what waits, until the target is ready.
    Queue,
    /// Dropped, and told: what waits is at the bound.
    Drop,
}

#[cfg_attr(verus_only, verus_verify)]
impl Gate {
    /// A target nothing has said is ready.
    #[cfg_attr(verus_only, verus_spec(made =>
        ensures
            !made.ready,
    ))]
    pub(crate) const fn new() -> Gate {
        Gate { ready: false }
    }

    /// Whether the target is ready.
    #[cfg_attr(verus_only, verus_spec(ready =>
        ensures
            ready == self.ready,
    ))]
    pub(crate) fn is_ready(&self) -> bool {
        self.ready
    }

    /// The target became ready.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).ready,
    ))]
    pub(crate) fn ready(&mut self) {
        self.ready = true;
    }

    /// The target stopped being ready: whether it was, which is the edge that is told. A target
    /// that was not ready had no connection to lose.
    #[cfg_attr(verus_only, verus_spec(lost =>
        ensures
            lost == old(self).ready,
            !final(self).ready,
    ))]
    pub(crate) fn not_ready(&mut self) -> bool {
        let lost = self.ready;
        self.ready = false;
        lost
    }

    /// How many envelopes wait, as the rules read it: `waiting`, or the most a `u32` holds when
    /// more wait than that, so that a queue too long to count reads as full and never as empty.
    #[cfg_attr(verus_only, verus_spec(depth =>
        ensures
            depth as int == (if waiting as int <= u32::MAX as int { waiting as int } else { u32::MAX as int }),
    ))]
    pub(crate) fn depth(waiting: usize) -> u32 {
        if waiting > u32::MAX as usize {
            u32::MAX
        } else {
            waiting as u32
        }
    }

    /// What becomes of an envelope admitted to a target, from the rules' verdicts on what waits for
    /// it: `sends_now`, that the target is ready and nothing waits, and `overflows`, that what waits
    /// is at the bound. Without a buffer (`buffered` false) it is sent at once, ready or not, and
    /// the transport says what it could not send. With one, it is sent at once only when the rule
    /// says so, since an envelope sent past a queue would overtake the ones waiting; otherwise it
    /// waits, or is dropped when what waits is at the bound.
    #[cfg_attr(verus_only, verus_spec(admission =>
        ensures
            (admission is Send) == (sends_now || !buffered),
            (admission is Drop) == (buffered && !sends_now && overflows),
            (admission is Queue) == (buffered && !sends_now && !overflows),
    ))]
    pub(crate) fn admission(buffered: bool, sends_now: bool, overflows: bool) -> Admission {
        if !buffered || sends_now {
            return Admission::Send;
        }
        if overflows {
            return Admission::Drop;
        }
        Admission::Queue
    }
}

/// One envelope's retry history: the sends made and the wait last chosen before jitter. Starts
/// empty at the first send.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempts {
    made: u32,
    last_backoff_ms: Option<i64>,
}

#[cfg_attr(verus_only, verus_verify)]
impl Attempts {
    /// No send made, and no wait chosen.
    #[cfg_attr(verus_only, verus_spec(fresh =>
        ensures
            fresh.made == 0,
            fresh.last_backoff_ms is None,
    ))]
    pub(crate) const fn new() -> Attempts {
        Attempts {
            made: 0,
            last_backoff_ms: None,
        }
    }

    /// The send just made failed: count it, and say how many were made. `None`, and nothing
    /// counted, when as many were made as a `u32` counts: no retry policy goes past that, so the
    /// envelope is given up.
    #[cfg_attr(verus_only, verus_spec(counted =>
        ensures
            final(self).last_backoff_ms == old(self).last_backoff_ms,
            counted matches Some(made) ==> old(self).made < u32::MAX && made == old(self).made + 1 && final(self).made == made,
            counted is None ==> old(self).made == u32::MAX && final(self).made == old(self).made,
    ))]
    pub(crate) fn failed(&mut self) -> Option<u32> {
        if self.made == u32::MAX {
            return None;
        }
        self.made += 1;
        Some(self.made)
    }

    /// A wait of `backoff` was chosen: the next one grows from it.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).made == old(self).made,
            final(self).last_backoff_ms == Some(backoff),
    ))]
    pub(crate) fn chose(&mut self, backoff: i64) {
        self.last_backoff_ms = Some(backoff);
    }

    /// Sends made so far.
    pub fn made(&self) -> u32 {
        self.made
    }

    /// The wait last chosen before jitter, `None` before the first.
    #[cfg_attr(verus_only, verus_spec(last =>
        ensures
            last == self.last_backoff_ms,
    ))]
    pub(crate) fn last_backoff_ms(&self) -> Option<i64> {
        self.last_backoff_ms
    }

    /// The history a checkpoint held: `made` sends, the last wait chosen `last_backoff_ms`.
    #[cfg_attr(verus_only, verus_spec(held =>
        ensures
            held.made == made,
            held.last_backoff_ms == last_backoff_ms,
    ))]
    pub(crate) fn resumed(made: u32, last_backoff_ms: Option<i64>) -> Attempts {
        Attempts {
            made,
            last_backoff_ms,
        }
    }
}

#[cfg(verus_only)]
verus! {

/// One target's envelopes as section 10.10 promises them, each numbered in the order it was
/// admitted: `admitted` is the number the next one takes; those that wait are numbered from
/// `first`, `waiting` of them, oldest first; `sent` is the number of the last one sent, -1 before
/// the first. Every number below `first` was sent or told, and so was every one from
/// `first + waiting` on, dropped at its admit because what waited was at the bound.
pub(crate) ghost struct Line {
    admitted: int,
    first: int,
    waiting: int,
    sent: int,
}

impl Line {
    /// Nothing that waits was admitted after what was sent, and an envelope was dropped behind
    /// what waits only while that was at `bound`.
    spec fn holds(self, bound: int) -> bool {
        &&& 0 <= self.waiting
        &&& self.sent < self.first
        &&& self.first + self.waiting <= self.admitted
        &&& (self.first + self.waiting == self.admitted || self.waiting >= bound)
    }
}

/// The bound on what waits for a target: `max_pending` under a buffer; without one nothing waits.
spec fn bound(buffered: bool, max_pending: u32) -> int {
    if buffered { max_pending as int } else { 0 }
}

/// An envelope admitted to a target, as `Outbound::admit` admits it, taking the number
/// `line.admitted`: sent at once only when nothing waits, so after every envelope admitted before
/// it; put behind what waits only below the bound, so in order behind it; dropped, and told, only
/// at the bound; and without a buffer, sent. A ready target that nothing waits for sends at once.
///
/// Taken as given of the rules, as their documents compute them: `outbound_sends_now` is true when
/// the target is ready and nothing waits, and `outbound_overflows` when what waits is at
/// `max_pending`; each reads what waits as `Gate::depth` gives it.
fn an_admit_keeps_the_order_and_the_bound(
    gate: &Gate, buffered: bool, max_pending: u32, waiting: usize, line: Ghost<Line>,
) -> (answer: (Admission, Ghost<Line>))
    requires
        line@.holds(bound(buffered, max_pending)),
        line@.waiting == waiting as int,
        !buffered ==> waiting == 0,
    ensures
        answer.1@.holds(bound(buffered, max_pending)),
        answer.1@.admitted == line@.admitted + 1,
        !buffered ==> answer.1@.waiting == 0,
        !buffered ==> answer.0 is Send,
        buffered && gate.ready && waiting == 0 ==> answer.0 is Send,
        answer.0 is Send ==> line@.waiting == 0 && answer.1@.sent == line@.admitted
            && answer.1@.waiting == 0 && (buffered ==> gate.ready),
        answer.0 is Queue ==> buffered && line@.waiting < max_pending
            && answer.1@.first == line@.first && answer.1@.waiting == line@.waiting + 1
            && answer.1@.sent == line@.sent,
        answer.0 is Drop ==> buffered && line@.waiting >= max_pending
            && answer.1@.first == line@.first && answer.1@.waiting == line@.waiting
            && answer.1@.sent == line@.sent,
{
    let depth = Gate::depth(waiting);
    let sends_now = gate.is_ready() && depth == 0;
    let overflows = depth >= max_pending;
    let admission = Gate::admission(buffered, sends_now, overflows);
    let ghost next = match admission {
        Admission::Send => Line {
            admitted: line@.admitted + 1,
            first: line@.admitted + 1,
            waiting: 0,
            sent: line@.admitted,
        },
        Admission::Queue => Line {
            admitted: line@.admitted + 1,
            first: line@.first,
            waiting: line@.waiting + 1,
            sent: line@.sent,
        },
        Admission::Drop => Line {
            admitted: line@.admitted + 1,
            first: line@.first,
            waiting: line@.waiting,
            sent: line@.sent,
        },
    };
    (admission, Ghost(next))
}

/// A target that becomes ready, as `Outbound::mark_ready` drains it: every envelope that waited
/// leaves, sent unless `outbound_stale` says it waited too long, when it is told; so the last one
/// sent, `last`, is one that waited, or none was and it is the last sent before. Then nothing waits
/// and the target is ready, so the next envelope admitted is sent after all of them.
///
/// Taken as given of the queue: it gives what it holds oldest first (a `VecDeque` taken from its
/// front), so the drain sends in the order the envelopes were admitted.
fn a_ready_target_releases_what_waited(
    gate: &mut Gate, line: Ghost<Line>, bound: Ghost<int>, last: Ghost<int>,
) -> (after: Ghost<Line>)
    requires
        line@.holds(bound@),
        last@ == line@.sent || line@.first <= last@ < line@.first + line@.waiting,
    ensures
        final(gate).ready,
        after@.holds(bound@),
        after@.waiting == 0,
        after@.admitted == line@.admitted,
        after@.sent == last@,
{
    gate.ready();
    Ghost(Line { admitted: line@.admitted, first: line@.admitted, waiting: 0, sent: last@ })
}

/// A target that stops being ready, as `Outbound::mark_not_ready` tells it: told once when it was
/// ready, and not again before it is ready again; never when it was not ready.
fn losing_readiness_is_told_once(gate: &mut Gate) -> (told: (bool, bool))
    ensures
        told.0 == old(gate).ready,
        !told.1,
        !final(gate).ready,
{
    let first = gate.not_ready();
    let again = gate.not_ready();
    (first, again)
}

/// A send that failed, as `Attempts::after_failure` takes it before it asks the rules: one more
/// send is counted, and no count wraps, since a count that cannot go on is `None`, which gives the
/// envelope up; and the wait last chosen moves only when a wait is chosen, `backoff`.
fn a_failed_send_is_counted_once(attempts: &mut Attempts, backoff: Option<i64>) -> (counted: Option<u32>)
    ensures
        counted matches Some(made) ==> made as int == old(attempts).made + 1 && final(attempts).made == made,
        counted is None ==> old(attempts).made == u32::MAX && final(attempts).made == u32::MAX,
        backoff matches Some(chosen) ==> final(attempts).last_backoff_ms == Some(chosen),
        backoff is None ==> final(attempts).last_backoff_ms == old(attempts).last_backoff_ms,
{
    let counted = attempts.failed();
    if let Some(chosen) = backoff {
        attempts.chose(chosen);
    }
    counted
}

}
