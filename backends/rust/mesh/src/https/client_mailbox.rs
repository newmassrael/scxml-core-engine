// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a client stands in its server's mailbox (SCE_MESH.md section 22.3): what a request
//! acknowledges, and what of a response is handed on. These are the rules of the module
//! `client_core` that an envelope is lost or handed on twice by, so they are here by themselves,
//! over numbers alone, and proved by Verus (docs/adr/0115, D13).
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes of
//! this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// Where an acknowledgement stands: the highest mailbox number handed to the core, and the
/// incarnation of the mailbox it counts in, `None` before the first `200`.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) ack: u64,
    pub(crate) epoch: Option<u128>,
}

/// Where the client stands: `ack` and `epoch` as a [`Mark`]'s, and under `receipt: durable` the
/// highest number in this incarnation that a durable checkpoint of the host holds.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mailbox {
    pub(crate) ack: u64,
    pub(crate) epoch: Option<u128>,
    pub(crate) held: Option<u64>,
}

/// Which numbering a response counts in: the client's own; the one its request was sent in, which
/// the client has left since; or one that is new, and whether it replaced one the client knew.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Numbering {
    Current,
    Left,
    Anew { replaced: bool },
}

#[cfg_attr(verus_only, verus_verify)]
impl Mailbox {
    /// A client that knows no mailbox; under `receipt: durable` (docs/adr/0109, D10) it holds nothing.
    #[cfg_attr(verus_only, verus_spec(made =>
        ensures
            made.ack == 0,
            made.epoch is None,
            made.held == if durable { Some(0u64) } else { None },
    ))]
    pub(crate) fn new(durable: bool) -> Mailbox {
        Mailbox {
            ack: 0,
            epoch: None,
            held: if durable { Some(0) } else { None },
        }
    }

    /// What the next request acknowledges: the highest number handed to the core, or, under
    /// `receipt: durable`, the highest a durable checkpoint of the host holds (docs/adr/0109,
    /// D10), so that the server never deletes what a crash of this client would lose.
    #[cfg_attr(verus_only, verus_spec(mark =>
        ensures
            mark.epoch == self.epoch,
            mark.ack <= self.ack,
            match self.held {
                Some(held) => mark.ack <= held && (mark.ack == held || mark.ack == self.ack),
                None => mark.ack == self.ack,
            },
    ))]
    pub(crate) fn acknowledging(&self) -> Mark {
        let ack = match self.held {
            Some(held) => held.min(self.ack),
            None => self.ack,
        };
        Mark {
            ack,
            epoch: self.epoch,
        }
    }

    /// A durable checkpoint of the host holds every envelope up to `number` (docs/adr/0109, D10).
    /// A mark of an incarnation the client has left counts in a numbering that is gone, and moves
    /// nothing; on a binding that is not durable this does nothing.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).ack == old(self).ack,
            final(self).epoch == old(self).epoch,
            final(self).held == match old(self).held {
                Some(held) if old(self).epoch == epoch && held < number => Some(number),
                unmoved => unmoved,
            },
    ))]
    pub(crate) fn hold(&mut self, epoch: Option<u128>, number: u64) {
        if self.epoch != epoch {
            return;
        }
        if let Some(held) = self.held {
            if held < number {
                self.held = Some(number);
            }
        }
    }

    /// A response named the incarnation `epoch`, to a request sent with `sent`: which numbering it
    /// counts in, and, for a mailbox that numbers anew, the acknowledgement started over at 0.
    #[cfg_attr(verus_only, verus_spec(numbering =>
        ensures
            match numbering {
                Numbering::Current => *final(self) == *old(self) && old(self).epoch == Some(epoch),
                Numbering::Left => *final(self) == *old(self) && old(self).epoch != Some(epoch)
                    && sent.epoch == Some(epoch),
                Numbering::Anew { replaced } => final(self).ack == 0
                    && final(self).held == (if old(self).held is Some { Some(0u64) } else { None }) && final(self).epoch == Some(epoch)
                    && replaced == (old(self).epoch is Some) && old(self).epoch != Some(epoch)
                    && sent.epoch != Some(epoch),
            },
    ))]
    pub(crate) fn enter(&mut self, sent: &Mark, epoch: u128) -> Numbering {
        if self.epoch == Some(epoch) {
            return Numbering::Current;
        }
        if sent.epoch == Some(epoch) {
            return Numbering::Left;
        }
        let replaced = self.epoch.is_some();
        self.ack = 0;
        self.epoch = Some(epoch);
        if self.held.is_some() {
            // What a checkpoint held counted in the numbering that is gone.
            self.held = Some(0);
        }
        Numbering::Anew { replaced }
    }

    /// Take `count` envelopes numbered from `first`, which a response in `numbering` carried to a
    /// request sent with `sent`: how many of the first are passed over, since they were handed on
    /// before, and the acknowledgement moved past them all. `None` takes nothing: a mailbox
    /// numbers from 1, and no number is past the last one there is.
    #[cfg_attr(verus_only, verus_spec(passed =>
        ensures
            final(self).epoch == old(self).epoch,
            final(self).held == old(self).held,
            match passed {
                None => *final(self) == *old(self)
                    && (first == 0 || count == 0 || first + count - 1 > u64::MAX),
                Some(passed) => first >= 1 && count >= 1 && passed <= count
                    && first + count - 1 <= u64::MAX
                    && passed == passed_over(taken(*old(self), numbering, *sent), first as int, count as int)
                    && final(self).ack == if numbering is Left || old(self).ack >= first + count - 1 { old(self).ack as int } else { first + count - 1 },
            },
    ))]
    pub(crate) fn take(
        &mut self,
        sent: &Mark,
        numbering: Numbering,
        first: u64,
        count: u64,
    ) -> Option<u64> {
        if first == 0 || count == 0 || count - 1 > u64::MAX - first {
            return None;
        }
        let last = first + (count - 1);
        let left = matches!(numbering, Numbering::Left);
        let taken = if left { sent.ack } else { self.ack };
        let passed = if taken < first {
            0
        } else if taken >= last {
            count
        } else {
            taken - first + 1
        };
        if !left && self.ack < last {
            self.ack = last;
        }
        Some(passed)
    }
}

#[cfg(verus_only)]
verus! {

/// The acknowledgement a response's numbers are read above.
spec fn taken(mailbox: Mailbox, numbering: Numbering, sent: Mark) -> int {
    if numbering is Left { sent.ack as int } else { mailbox.ack as int }
}

/// Of `count` numbers from `first`, how many are at or below `taken`.
spec fn passed_over(taken: int, first: int, count: int) -> int {
    if taken < first { 0 } else if taken >= first + count - 1 { count } else { taken - first + 1 }
}

/// A client and its server's mailbox, in the incarnation the client is in: the numbers the client
/// handed to its core, and those the server still holds for it.
pub(crate) ghost struct Run {
    mailbox: Mailbox,
    handed: spec_fn(int) -> bool,
    owed: spec_fn(int) -> bool,
}

impl Run {
    /// What an acknowledgement of the client would have the server drop was handed to the core,
    /// and nothing was handed that is above the acknowledgement.
    spec fn safe(self) -> bool {
        &&& forall|n: int| #[trigger] (self.owed)(n) && n <= self.mailbox.ack ==> (self.handed)(n)
        &&& forall|n: int| #[trigger] (self.handed)(n) ==> 1 <= n <= self.mailbox.ack
    }
}

/// The numbers a response of `count` envelopes from `first` hands on, past the first `passed`.
spec fn handed_on(first: int, count: int, passed: int) -> spec_fn(int) -> bool {
    |n: int| first + passed <= n < first + count
}

/// A response of the client's incarnation, taken as `Mailbox::take` says, keeps the run safe and
/// hands on no number twice.
///
/// Taken as given of the server (SCE_MESH.md section 22.3): its response begins at the oldest
/// number it holds above `base`, what the request acknowledged in this incarnation, and numbers on
/// without a gap.
proof fn a_response_is_taken_once_and_whole(
    run: Run, after: Mailbox, base: int, first: int, count: int, passed: int,
)
    requires
        run.safe(),
        1 <= first, 1 <= count,
        0 <= base <= run.mailbox.ack,
        forall|n: int| #[trigger] (run.owed)(n) && base < n ==> first <= n,
        passed == passed_over(run.mailbox.ack as int, first, count),
        after.ack == if run.mailbox.ack >= first + count - 1 { run.mailbox.ack as int } else { first + count - 1 },
    ensures
        forall|n: int| #[trigger] handed_on(first, count, passed)(n) ==> !(run.handed)(n),
        (Run {
            mailbox: after,
            handed: |n: int| (run.handed)(n) || handed_on(first, count, passed)(n),
            owed: run.owed,
        }).safe(),
{
    let handed = |n: int| (run.handed)(n) || handed_on(first, count, passed)(n);
    assert forall|n: int| #[trigger] (run.owed)(n) && n <= after.ack implies handed(n) by {
        if n > run.mailbox.ack {
            assert(handed_on(first, count, passed)(n));
        }
    }
}

/// What the request acknowledged in the incarnation `epoch`: nothing, when it counted in another.
spec fn base(sent: Mark, epoch: u128) -> int {
    if sent.epoch == Some(epoch) { sent.ack as int } else { 0 }
}

/// A request acknowledges nothing the client did not hand to its core, and under `receipt: durable`
/// nothing its host did not report as held: the server drops no more than that.
fn a_request_drops_only_what_was_handed(mailbox: &Mailbox, run: Ghost<Run>) -> (mark: Mark)
    requires
        run@.safe(),
        run@.mailbox == *mailbox,
    ensures
        mark.epoch == mailbox.epoch,
        mark.ack <= mailbox.ack,
        forall|n: int| #[trigger] (run@.owed)(n) && n <= mark.ack ==> (run@.handed)(n),
        mailbox.held matches Some(held) ==> mark.ack <= held,
{
    mailbox.acknowledging()
}

/// Whatever a response is, the client is safe after it, in the incarnation it is in then: in its
/// own, it handed on no number twice; in one it left, its acknowledgement did not move; in a new
/// one, the acknowledgement began at 0, and what was handed on is the response's.
///
/// Taken as given of the server (SCE_MESH.md section 22.3): its response begins at the oldest
/// number it holds above what the request acknowledged in its incarnation, and it holds no number
/// below 1. Of the client: a request acknowledged no more than it acknowledges now in the same
/// incarnation, since no step takes `ack` back but a new incarnation's.
fn a_response_keeps_the_client_safe(
    mailbox: &mut Mailbox, sent: &Mark, epoch: u128, first: u64, count: u64, run: Ghost<Run>,
    owed: Ghost<spec_fn(int) -> bool>,
) -> (answer: (Numbering, Option<u64>))
    requires
        run@.safe(),
        run@.mailbox == *old(mailbox),
        old(mailbox).epoch == Some(epoch) ==> owed@ == run@.owed,
        sent.epoch == Some(epoch) && old(mailbox).epoch == Some(epoch) ==> sent.ack <= old(mailbox).ack,
        forall|n: int| #[trigger] (owed@)(n) ==> n >= 1,
        forall|n: int| #[trigger] (owed@)(n) && base(*sent, epoch) < n ==> first <= n,
    ensures
        answer.0 is Left ==> final(mailbox).ack == old(mailbox).ack && final(mailbox).epoch == old(mailbox).epoch,
        answer.0 is Left || answer.1 is None ==> !(answer.0 is Anew) ==> *final(mailbox) == *old(mailbox),
        !(answer.0 is Left) ==> ({
            let before = |n: int| !(answer.0 is Anew) && (run@.handed)(n);
            let new = |n: int| answer.1 matches Some(passed) && handed_on(first as int, count as int, passed as int)(n);
            &&& forall|n: int| #[trigger] new(n) ==> !before(n)
            &&& (Run { mailbox: *final(mailbox), handed: |n: int| before(n) || new(n), owed: owed@ }).safe()
        }),
{
    let numbering = mailbox.enter(sent, epoch);
    let ghost entered = *mailbox;
    let passed = mailbox.take(sent, numbering, first, count);
    proof {
        let before = |n: int| !(numbering is Anew) && (run@.handed)(n);
        let run = Run { mailbox: entered, handed: before, owed: owed@ };
        if !(numbering is Left) {
            assert(run.safe());
            if let Some(passed) = passed {
                a_response_is_taken_once_and_whole(
                    run, *mailbox, base(*sent, epoch), first as int, count as int, passed as int);
            }
        }
    }
    (numbering, passed)
}

}
