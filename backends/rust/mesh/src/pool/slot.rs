// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a pool decides of one session's slot under the slot's locks (docs/adr/0107, D3;
//! docs/adr/0109, D1; SCE_MESH.md section 14.4): whether an envelope for a key is admitted, to the
//! session the table holds for it or to a new one, or meets a bound; how an inbox counts what it
//! holds, and when a session may leave the table; and the lease the process holds on the key,
//! when it comes due for renewal, and what is written and given up under it. `Pool` holds the
//! table, the inputs, the clock and the store, and decides none of this. These rules are here by
//! themselves, over counts, fences and moments alone, and proved by Verus (docs/adr/0115, D13): a
//! stopping pool admits nothing; the table holds no more sessions than `max_live` and an inbox no
//! more envelopes than `max_inbox`; every input an inbox took is applied once by its session or
//! answered when the session left the table, which it leaves by itself only when its inbox holds
//! nothing;
//! a lease keeps the fence it was granted, a lost lease is neither written under nor renewed, and
//! a release gives up only the lease held; and a renewal comes due when the pool wakes for it.
//!
//! Who holds which lock and when is the pool's (`crate::pool`): a post and a removal both hold
//! the table, so no input reaches an inbox that left it; and a session leaves the table only in a
//! thread that holds its slot's `live` lock, so never while another serves it.
//!
//! Taken as given, and judged elsewhere: what the shell posts and takes is what it tells the
//! [`Door`] (the inbox's queue and its door move together, in `Pool`'s one place for each); an
//! inbox holds fewer inputs than a `usize` counts; and a fence the store grants names the lease
//! it granted (the store's own rules, `sce_rust_session_store`'s `rules/lease.rs`).
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes
//! of this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// Which bound of what the pool holds in memory an envelope met (docs/adr/0109, D1).
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Full {
    /// Its session's inbox holds `sessions.max_inbox` envelopes.
    Inbox,
    /// Its key is not in memory, and the pool holds `sessions.max_live`.
    Live,
    /// The pool is stopping (`Pool::stop`): it admits nothing.
    Stopping,
}

/// What one session's inbox holds, as the pool's bounds count it: how many inputs, how many of
/// them envelopes, and whether the inbox is closed, which it is once its session left the table.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Door {
    held: usize,
    envelopes: usize,
    closed: bool,
}

#[cfg_attr(verus_only, verus_verify)]
impl Door {
    /// The inbox of a session the table takes in: it holds nothing.
    #[cfg_attr(verus_only, verus_spec(made =>
        ensures
            made.held == 0,
            made.envelopes == 0,
            !made.closed,
    ))]
    pub(crate) const fn open() -> Door {
        Door {
            held: 0,
            envelopes: 0,
            closed: false,
        }
    }

    /// Whether the inbox holds no input.
    #[cfg_attr(verus_only, verus_spec(empty =>
        ensures
            empty == (self.held == 0),
    ))]
    pub(crate) fn is_empty(&self) -> bool {
        self.held == 0
    }

    /// An envelope is posted to the inbox: whether it is taken, which it is while fewer than
    /// `max_inbox` envelopes wait in it. One refused changes nothing.
    #[cfg_attr(verus_only, verus_spec(taken =>
        requires
            old(self).held < usize::MAX,
        ensures
            taken == (old(self).envelopes < max_inbox as usize),
            taken ==> final(self).held == old(self).held + 1
                && final(self).envelopes == old(self).envelopes + 1
                && final(self).closed == old(self).closed,
            !taken ==> *final(self) == *old(self),
    ))]
    pub(crate) fn post(&mut self, max_inbox: u32) -> bool {
        if self.envelopes >= max_inbox as usize {
            return false;
        }
        self.held += 1;
        self.envelopes += 1;
        true
    }

    /// An input that is no envelope (a link's readiness, a tick, a wake, a note) is put in the
    /// inbox: no bound counts it, since the pool makes it itself, at most one per session for each
    /// call of the host's.
    #[cfg_attr(verus_only, verus_spec(
        requires
            old(self).held < usize::MAX,
        ensures
            final(self).held == old(self).held + 1,
            final(self).envelopes == old(self).envelopes,
            final(self).closed == old(self).closed,
    ))]
    pub(crate) fn tell(&mut self) {
        self.held += 1;
    }

    /// The session's owner took the inbox's oldest input, an envelope when `envelope`.
    #[cfg_attr(verus_only, verus_spec(
        requires
            old(self).held > 0,
            envelope ==> old(self).envelopes > 0,
            !envelope ==> old(self).envelopes < old(self).held,
        ensures
            final(self).held == old(self).held - 1,
            final(self).envelopes
                == (if envelope { old(self).envelopes - 1 } else { old(self).envelopes as int }),
            final(self).closed == old(self).closed,
    ))]
    pub(crate) fn took(&mut self, envelope: bool) {
        self.held -= 1;
        if envelope {
            self.envelopes -= 1;
        }
    }

    /// The session is to leave the table, which it does by itself only when its inbox holds
    /// nothing, so that no input it took is dropped: whether it is out, closed now or before. One
    /// that holds an input stays, open, and its owner takes the input.
    #[cfg_attr(verus_only, verus_spec(out =>
        ensures
            out == (old(self).closed || old(self).held == 0),
            out ==> final(self).closed && final(self).held == old(self).held
                && final(self).envelopes == old(self).envelopes,
            !out ==> *final(self) == *old(self),
    ))]
    pub(crate) fn retire(&mut self) -> bool {
        if self.closed {
            return true;
        }
        if self.held > 0 {
            return false;
        }
        self.closed = true;
        true
    }

    /// The session leaves the table whatever its inbox holds (its key is no longer this
    /// process's, or the pool stopped): the inbox is closed and empty, and the count of what it
    /// held is given back, each of which the pool answers.
    #[cfg_attr(verus_only, verus_spec(given =>
        ensures
            given == old(self).held,
            final(self).closed,
            final(self).held == 0,
            final(self).envelopes == 0,
    ))]
    pub(crate) fn close(&mut self) -> usize {
        let given = self.held;
        self.closed = true;
        self.held = 0;
        self.envelopes = 0;
        given
    }
}

/// What becomes of an envelope posted for a session, with the table held: `door` is the door of
/// the session the table holds for its key, or `None` when it holds none, and `live` how many
/// sessions it holds. A stopping pool admits nothing; a key the table holds no session for is
/// given a new one while the table holds fewer than `max_live`; and the session's inbox takes the
/// envelope while fewer than `max_inbox` wait in it. Admitted, the door after the post.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(admitted =>
    requires
        door matches Some(before) ==> before.held < usize::MAX,
    ensures
        match admitted {
            Ok(after) => !stopping && match door {
                Some(before) => before.envelopes < max_inbox as usize
                    && after.held == before.held + 1
                    && after.envelopes == before.envelopes + 1
                    && after.closed == before.closed,
                None => live < max_live as usize && 0 < max_inbox
                    && after.held == 1 && after.envelopes == 1 && !after.closed,
            },
            Err(full) => match full {
                Full::Stopping => stopping,
                Full::Live => !stopping && door is None && live >= max_live as usize,
                Full::Inbox => !stopping && match door {
                    Some(before) => before.envelopes >= max_inbox as usize,
                    None => live < max_live as usize && max_inbox == 0,
                },
            },
        },
))]
pub(crate) fn admit(
    stopping: bool,
    door: Option<Door>,
    live: usize,
    max_live: u32,
    max_inbox: u32,
) -> Result<Door, Full> {
    if stopping {
        return Err(Full::Stopping);
    }
    let mut door = match door {
        Some(door) => door,
        None => {
            if live >= max_live as usize {
                return Err(Full::Live);
            }
            Door::open()
        }
    };
    if door.post(max_inbox) {
        Ok(door)
    } else {
        Err(Full::Inbox)
    }
}

/// How many more sessions a table that holds `live` may take in before it holds `max_live`.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(left =>
    ensures
        left as int == (if live < max_live as usize { max_live as int - live as int } else { 0 }),
))]
pub(crate) fn room(live: usize, max_live: u32) -> usize {
    let most = max_live as usize;
    if live >= most {
        return 0;
    }
    most - live
}

/// The lease the process holds on a session's key, as the pool keeps it beside the session.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyLease {
    state: Tenure,
}

/// Where a lease stands. Read only through a [`KeyLease`], whose field holding it no code
/// outside this module reaches.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tenure {
    /// The process holds no lease on the key.
    Unheld,
    /// The process holds the lease of fence `fence`, last granted or renewed at `renewed_ms`.
    Held { fence: u64, renewed_ms: u64 },
    /// Taken over, or not renewed: the session is dropped at its next run.
    Lost,
}

/// The moment a lease renewed at `renewed_ms` comes due again, every `every_ms`: their sum, or
/// the last moment there is, so that a renewal never comes due early by wrapping.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(moment =>
    ensures
        moment as int == when_due(renewed_ms, every_ms),
))]
pub(crate) fn renewal_moment(renewed_ms: u64, every_ms: u64) -> u64 {
    if renewed_ms > u64::MAX - every_ms {
        u64::MAX
    } else {
        renewed_ms + every_ms
    }
}

#[cfg_attr(verus_only, verus_verify)]
impl KeyLease {
    /// The process holds no lease on the key yet.
    #[cfg_attr(verus_only, verus_spec(made =>
        ensures
            made.state is Unheld,
    ))]
    pub(crate) const fn unheld() -> KeyLease {
        KeyLease {
            state: Tenure::Unheld,
        }
    }

    /// The store granted the lease of `fence` at `now_ms`.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).state == (Tenure::Held { fence, renewed_ms: now_ms }),
    ))]
    pub(crate) fn granted(&mut self, fence: u64, now_ms: u64) {
        self.state = Tenure::Held {
            fence,
            renewed_ms: now_ms,
        };
    }

    /// The fence a write of the session is made under: the one of the lease held, and none
    /// when the process holds none or lost it, so that a session whose key it lost writes nothing.
    #[cfg_attr(verus_only, verus_spec(held =>
        ensures
            self.state matches Tenure::Held { fence, .. } ==> held == Some(fence),
            !(self.state is Held) ==> held is None,
    ))]
    pub(crate) fn fence(&self) -> Option<u64> {
        match self.state {
            Tenure::Held { fence, .. } => Some(fence),
            Tenure::Unheld | Tenure::Lost => None,
        }
    }

    /// Whether the lease was lost, so that the session's run drops it before it takes an input.
    #[cfg_attr(verus_only, verus_spec(lost =>
        ensures
            lost == (self.state is Lost),
    ))]
    pub(crate) fn is_lost(&self) -> bool {
        matches!(self.state, Tenure::Lost)
    }

    /// The fence of the lease to renew at `now_ms`, renewed every `every_ms`: the lease held, once
    /// `every_ms` passed since it was last granted or renewed; none otherwise.
    #[cfg_attr(verus_only, verus_spec(due =>
        ensures
            self.state matches Tenure::Held { fence, renewed_ms } ==> due
                == (if now_ms as int >= when_due(renewed_ms, every_ms) {
                    Some(fence)
                } else {
                    None
                }),
            !(self.state is Held) ==> due is None,
    ))]
    pub(crate) fn renewal_due(&self, now_ms: u64, every_ms: u64) -> Option<u64> {
        match self.state {
            Tenure::Held { fence, renewed_ms } => {
                if now_ms >= renewal_moment(renewed_ms, every_ms) {
                    Some(fence)
                } else {
                    None
                }
            }
            Tenure::Unheld | Tenure::Lost => None,
        }
    }

    /// The moment the lease held comes due for renewal, every `every_ms`, which the pool wakes at;
    /// none when it holds none.
    #[cfg_attr(verus_only, verus_spec(at =>
        ensures
            self.state matches Tenure::Held { renewed_ms, .. }
                ==> (at matches Some(moment) && moment as int == when_due(renewed_ms, every_ms)),
            !(self.state is Held) ==> at is None,
    ))]
    pub(crate) fn next_renewal_ms(&self, every_ms: u64) -> Option<u64> {
        match self.state {
            Tenure::Held { renewed_ms, .. } => Some(renewal_moment(renewed_ms, every_ms)),
            Tenure::Unheld | Tenure::Lost => None,
        }
    }

    /// The store renewed the lease held at `now_ms`: it keeps its fence, and comes due again
    /// from then. A lease not held stays as it is: a renewal revives no lease that was lost.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            old(self).state matches Tenure::Held { fence, .. }
                ==> final(self).state == (Tenure::Held { fence, renewed_ms: now_ms }),
            !(old(self).state is Held) ==> *final(self) == *old(self),
    ))]
    pub(crate) fn renewed(&mut self, now_ms: u64) {
        if let Tenure::Held { fence, .. } = self.state {
            self.state = Tenure::Held {
                fence,
                renewed_ms: now_ms,
            };
        }
    }

    /// The store refused a renewal as lost, or a write as fenced out: another process holds the
    /// key.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).state is Lost,
    ))]
    pub(crate) fn lose(&mut self) {
        self.state = Tenure::Lost;
    }

    /// The session was dropped without giving the lease up: the key is another's, or its lease
    /// ran out.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).state is Unheld,
    ))]
    pub(crate) fn forget(&mut self) {
        self.state = Tenure::Unheld;
    }

    /// The session gives its lease up: the fence of the lease held, which the store releases, and
    /// none when the process holds none or lost it, since a lease taken over is not this
    /// process's to give up.
    #[cfg_attr(verus_only, verus_spec(given =>
        ensures
            final(self).state is Unheld,
            old(self).state matches Tenure::Held { fence, .. } ==> given == Some(fence),
            !(old(self).state is Held) ==> given is None,
    ))]
    pub(crate) fn give_up(&mut self) -> Option<u64> {
        let given = self.fence();
        self.state = Tenure::Unheld;
        given
    }
}

#[cfg(verus_only)]
verus! {

/// When a lease renewed at `renewed_ms` comes due, every `every_ms`: their sum, or the last moment
/// a `u64` holds.
spec fn when_due(renewed_ms: u64, every_ms: u64) -> int {
    if renewed_ms as int + every_ms as int <= u64::MAX as int {
        renewed_ms as int + every_ms as int
    } else {
        u64::MAX as int
    }
}

/// An inbox within the pool's bounds: no more envelopes than inputs, no more than `max_inbox` of
/// them, and nothing in it once it is closed.
spec fn bounded(door: Door, max_inbox: u32) -> bool {
    &&& door.envelopes <= door.held
    &&& door.envelopes <= max_inbox as usize
    &&& door.closed ==> door.held == 0
}

/// What became of the inputs one inbox took, a history no pool keeps: `posted` of them taken,
/// `applied` by the session's owner, and `answered` when the session left the table.
pub(crate) ghost struct Ledger {
    posted: int,
    applied: int,
    answered: int,
}

/// Every input the inbox took and that was neither applied nor answered is one it holds.
spec fn accounts(door: Door, ledger: Ledger) -> bool {
    door.held as int == ledger.posted - ledger.applied - ledger.answered
}

/// An envelope posted with the table held, as `Pool::try_post` posts it: admitted, it is counted
/// once and the inbox stays within its bound, and a new session keeps the table within
/// `max_live`; refused, nothing moves. A stopping pool admits nothing.
fn an_admit_keeps_the_bounds(
    stopping: bool, door: Option<Door>, live: usize, max_live: u32, max_inbox: u32,
    ledger: Ghost<Ledger>,
) -> (answer: (Result<Door, Full>, Ghost<Ledger>))
    requires
        live <= max_live as usize,
        door matches Some(before) ==> bounded(before, max_inbox) && !before.closed
            && before.held < usize::MAX && accounts(before, ledger@),
        door is None ==> ledger@ == (Ledger { posted: 0, applied: 0, answered: 0 }),
    ensures
        stopping ==> answer.0 is Err,
        answer.0 matches Ok(after) ==> bounded(after, max_inbox) && !after.closed
            && accounts(after, answer.1@) && answer.1@.posted == ledger@.posted + 1
            && answer.1@.applied == ledger@.applied && answer.1@.answered == ledger@.answered
            && (door is None ==> live + 1 <= max_live as usize),
        answer.0 is Err ==> answer.1@ == ledger@,
{
    let admitted = admit(stopping, door, live, max_live, max_inbox);
    match admitted {
        Ok(_) => (admitted, Ghost(Ledger {
            posted: ledger@.posted + 1,
            applied: ledger@.applied,
            answered: ledger@.answered,
        })),
        Err(_) => (admitted, ledger),
    }
}

/// An input the pool makes itself put in an inbox the table holds, as `Inbox::tell` puts it:
/// counted once, and the bound on envelopes is as it was.
fn an_input_told_is_counted(
    door: &mut Door, max_inbox: Ghost<u32>, ledger: Ghost<Ledger>,
) -> (after: Ghost<Ledger>)
    requires
        bounded(*old(door), max_inbox@),
        !old(door).closed,
        old(door).held < usize::MAX,
        accounts(*old(door), ledger@),
    ensures
        bounded(*final(door), max_inbox@),
        !final(door).closed,
        accounts(*final(door), after@),
        after@.posted == ledger@.posted + 1,
        after@.applied == ledger@.applied,
        after@.answered == ledger@.answered,
{
    door.tell();
    Ghost(Ledger {
        posted: ledger@.posted + 1,
        applied: ledger@.applied,
        answered: ledger@.answered,
    })
}

/// The owner takes the inbox's oldest input, as `Inbox::take` takes it: it is applied once, and the
/// inbox holds one fewer. That the oldest is an envelope or not is the queue's.
fn an_input_taken_is_applied_once(
    door: &mut Door, envelope: bool, max_inbox: Ghost<u32>, ledger: Ghost<Ledger>,
) -> (after: Ghost<Ledger>)
    requires
        bounded(*old(door), max_inbox@),
        accounts(*old(door), ledger@),
        old(door).held > 0,
        envelope ==> old(door).envelopes > 0,
        !envelope ==> old(door).envelopes < old(door).held,
    ensures
        bounded(*final(door), max_inbox@),
        accounts(*final(door), after@),
        after@.applied == ledger@.applied + 1,
        after@.posted == ledger@.posted,
        after@.answered == ledger@.answered,
{
    door.took(envelope);
    Ghost(Ledger {
        posted: ledger@.posted,
        applied: ledger@.applied + 1,
        answered: ledger@.answered,
    })
}

/// A session that leaves the table by itself, as `Pool::retire_if_idle` and `Pool::evict` take it
/// out: only with nothing in its inbox, so every input it took was applied or answered, and
/// closed, so nothing reaches it after.
fn a_session_leaves_by_itself_only_empty(
    door: &mut Door, max_inbox: Ghost<u32>, ledger: Ghost<Ledger>,
) -> (out: bool)
    requires
        bounded(*old(door), max_inbox@),
        accounts(*old(door), ledger@),
    ensures
        bounded(*final(door), max_inbox@),
        accounts(*final(door), ledger@),
        out ==> final(door).closed && final(door).held == 0
            && ledger@.posted == ledger@.applied + ledger@.answered,
        !out ==> *final(door) == *old(door) && !old(door).closed,
{
    door.retire()
}

/// A session taken out of the table whatever its inbox holds, as `Pool::abandon` takes it: every
/// input it held is answered, once, and nothing reaches it after.
fn a_session_abandoned_answers_what_it_held(
    door: &mut Door, max_inbox: Ghost<u32>, ledger: Ghost<Ledger>,
) -> (answer: (usize, Ghost<Ledger>))
    requires
        bounded(*old(door), max_inbox@),
        accounts(*old(door), ledger@),
    ensures
        bounded(*final(door), max_inbox@),
        final(door).closed,
        accounts(*final(door), answer.1@),
        answer.0 == old(door).held,
        answer.1@.answered == ledger@.answered + answer.0,
        answer.1@.posted == answer.1@.applied + answer.1@.answered,
{
    let given = door.close();
    (given, Ghost(Ledger {
        posted: ledger@.posted,
        applied: ledger@.applied,
        answered: ledger@.answered + given,
    }))
}

/// The pool wakes for a renewal at the moment it comes due, as `Pool::next_wake_ms` and
/// `Pool::renew_leases` read the lease: a renewal is due at `now_ms` exactly when the moment the
/// pool wakes at has come, so it is neither missed nor made early; and what it renews is the
/// lease held, by its fence.
fn a_renewal_comes_due_when_the_pool_wakes(
    lease: &KeyLease, now_ms: u64, every_ms: u64,
) -> (answer: (Option<u64>, Option<u64>))
    ensures
        (answer.0 is Some) == (answer.1 matches Some(moment) && now_ms >= moment),
        answer.0 matches Some(fence)
            ==> (lease.state matches Tenure::Held { fence: held, .. } && held == fence),
{
    let due = lease.renewal_due(now_ms, every_ms);
    let at = lease.next_renewal_ms(every_ms);
    (due, at)
}

/// A lease the store granted and then renewed, as `Pool::load` and `Pool::renew_leases` take it: it
/// keeps the fence it was granted, so every write of the session is made under that fence, and it
/// comes due again from the moment it was granted or, once renewed, from the renewal.
fn a_lease_keeps_the_fence_it_was_granted(
    lease: &mut KeyLease, fence: u64, granted_ms: u64, now_ms: u64, every_ms: u64,
) -> (written: Option<u64>)
    ensures
        written == Some(fence),
        final(lease).state matches Tenure::Held { renewed_ms, .. }
            && (renewed_ms == granted_ms || renewed_ms == now_ms),
{
    lease.granted(fence, granted_ms);
    if lease.renewal_due(now_ms, every_ms).is_some() {
        lease.renewed(now_ms);
    }
    lease.fence()
}

/// A lease lost, as `Pool::renew_leases` and `Pool::checkpoint` mark it: no write is made under
/// it, no renewal comes due or revives it, and its run drops the session; a release gives
/// nothing up, since the lease is another's now.
fn a_lost_lease_is_neither_written_nor_renewed(
    lease: &mut KeyLease, now_ms: u64, every_ms: u64,
) -> (after: (Option<u64>, Option<u64>, bool, Option<u64>))
    ensures
        after.0 is None,
        after.1 is None,
        after.2,
        after.3 is None,
        final(lease).state is Unheld,
{
    lease.lose();
    lease.renewed(now_ms);
    let written = lease.fence();
    let due = lease.renewal_due(now_ms, every_ms);
    let lost = lease.is_lost();
    let given = lease.give_up();
    (written, due, lost, given)
}

}

#[cfg(test)]
mod tests {
    use super::{admit, renewal_moment, room, Door, Full, KeyLease};

    /// An inbox that took `envelopes` envelopes and `told` other inputs, under a bound none met.
    fn holding(envelopes: usize, told: usize) -> Door {
        let mut door = Door::open();
        for _ in 0..envelopes {
            assert!(door.post(u32::MAX));
        }
        for _ in 0..told {
            door.tell();
        }
        door
    }

    #[test]
    fn an_inbox_takes_envelopes_below_its_bound_and_inputs_it_makes_past_it() {
        let mut door = holding(1, 0);
        assert!(door.post(2));
        assert!(!door.post(2), "an inbox took an envelope past max_inbox");
        assert_eq!(door, holding(2, 0), "a refused envelope moved the count");
        door.tell();
        assert_eq!(door, holding(2, 1), "an input the pool makes was refused");
    }

    #[test]
    fn an_inbox_empties_as_its_owner_takes_each_input_once() {
        let mut door = holding(1, 1);
        door.took(false);
        assert!(!door.is_empty());
        door.took(true);
        assert!(door.is_empty());
        assert!(
            door.post(1),
            "a taken envelope still counted against the bound"
        );
    }

    #[test]
    fn a_session_leaves_by_itself_only_with_its_inbox_empty() {
        let mut held = holding(1, 0);
        assert!(!held.retire(), "a session left the table holding an input");
        assert_eq!(held, holding(1, 0));
        held.took(true);
        assert!(held.retire());
        assert!(
            held.retire(),
            "a session out of the table was read as in it"
        );
    }

    #[test]
    fn a_session_abandoned_gives_back_what_it_held() {
        let mut door = holding(2, 1);
        assert_eq!(door.close(), 3);
        assert!(door.is_empty());
        assert!(door.retire(), "a closed inbox was read as open");
    }

    #[test]
    fn an_admit_meets_the_stop_the_live_bound_and_the_inbox_bound_in_that_order() {
        assert_eq!(
            admit(true, Some(Door::open()), 0, 1, 1),
            Err(Full::Stopping)
        );
        assert_eq!(admit(true, None, 1, 1, 1), Err(Full::Stopping));
        assert_eq!(admit(false, None, 1, 1, 1), Err(Full::Live));
        assert_eq!(admit(false, None, 0, 1, 1), Ok(holding(1, 0)));
        assert_eq!(
            admit(false, Some(holding(1, 0)), 1, 1, 1),
            Err(Full::Inbox),
            "a session at max_inbox took one more"
        );
        assert_eq!(
            admit(false, Some(holding(0, 2)), 1, 1, 1),
            Ok(holding(1, 2)),
            "the inputs the pool makes counted against max_inbox"
        );
    }

    #[test]
    fn a_new_session_whose_inbox_takes_nothing_is_not_made() {
        assert_eq!(admit(false, None, 0, 1, 0), Err(Full::Inbox));
    }

    #[test]
    fn the_room_left_is_what_max_live_leaves() {
        assert_eq!(room(1, 3), 2);
        assert_eq!(room(3, 3), 0);
        assert_eq!(room(4, 3), 0, "a table past its bound read as having room");
    }

    #[test]
    fn a_renewal_comes_due_every_period_and_never_early_by_wrapping() {
        assert_eq!(renewal_moment(10, 5), 15);
        assert_eq!(renewal_moment(u64::MAX - 1, 5), u64::MAX);
        let mut lease = KeyLease::unheld();
        assert_eq!(lease.renewal_due(u64::MAX, 5), None);
        assert_eq!(lease.next_renewal_ms(5), None);
        lease.granted(7, 100);
        assert_eq!(lease.next_renewal_ms(50), Some(150));
        assert_eq!(lease.renewal_due(149, 50), None);
        assert_eq!(lease.renewal_due(150, 50), Some(7));
        lease.renewed(150);
        assert_eq!(lease.fence(), Some(7), "a renewal moved the fence");
        assert_eq!(lease.renewal_due(150, 50), None);
        assert_eq!(lease.next_renewal_ms(50), Some(200));
    }

    #[test]
    fn a_lost_lease_is_neither_written_nor_renewed_nor_given_up() {
        let mut lease = KeyLease::unheld();
        lease.granted(7, 100);
        lease.lose();
        assert!(lease.is_lost());
        lease.renewed(500);
        assert!(lease.is_lost(), "a renewal revived a lost lease");
        assert_eq!(lease.fence(), None);
        assert_eq!(lease.renewal_due(u64::MAX, 1), None);
        assert_eq!(
            lease.give_up(),
            None,
            "a release gave up a lease another holds"
        );
        assert!(!lease.is_lost());
    }

    #[test]
    fn a_lease_held_is_given_up_once_and_a_dropped_one_is_forgotten() {
        let mut lease = KeyLease::unheld();
        lease.granted(9, 0);
        assert_eq!(lease.give_up(), Some(9));
        assert_eq!(lease.give_up(), None);
        lease.granted(10, 0);
        lease.forget();
        assert_eq!(lease.fence(), None);
        assert_eq!(lease.give_up(), None);
    }
}
