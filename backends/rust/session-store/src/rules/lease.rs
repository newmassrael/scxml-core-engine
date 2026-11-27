// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a session store decides of a key's lease and of a write under it (docs/adr/0107, D6;
//! docs/adr/0114, D2): whether a lease another owner holds may be taken over, whether a write is
//! made under the lease held and from the version held, and whether a renewal or a release is of
//! the lease held. Each store reads what a call needs inside its transaction, asks these rules
//! through the module `rules`, and writes only what they admit, so that no two stores answer one
//! history differently. These rules are here by themselves, over numbers alone, and proved by
//! Verus (docs/adr/0115, D13) to refine the store's actions of `EventJournal`
//! (docs/formal/EventJournal.tla): a lease granted is a step of `Take`, its fence one no grant of
//! the key had; a write made is a step of `Commit` whose transaction holds the fence of the last
//! grant, `fence[s]`, and continues the state the store holds, so `st[s]` moves by one; a write
//! refused, a renewal and a release are no step of the store's variables. And of
//! `CheckpointWrite` (docs/formal/CheckpointWrite.tla), which judges a lease by time: a lease
//! another owner holds passes to a new one only once it ran out by the clock the taker gives, the
//! guard `clock[p] >= Due(j)` of its `Take`.
//!
//! Taken as given, and judged elsewhere: a fence a store grants is a number above every one it
//! gave before (one counter of the store that only rises, held to that by the conformance kit);
//! a call of a store is one transaction, so what the rules read is what the write they admit
//! changes; and whether two owners are one is the store's comparison of their names.
//!
//! What `CheckpointWrite` does not model, and a store does: a renewal, which moves the deadline
//! the store records; a release, after which the key may be taken at once; and an owner that asks
//! again while it holds the lease, as a process that started again under its name does, whose new
//! grant fences out its previous life before that life's deadline. Each is a step of
//! `EventJournal`, whose `Take` no clock judges, and none is a step of `CheckpointWrite`'s
//! `Take`: its `TakenWhenDue` holds of a store for a lease another owner holds, and of no other.
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes
//! of this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// A lease as a store holds it on a key, as these rules read it: its fence, and the moment it runs
/// out.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Grant {
    pub(crate) fence: u64,
    pub(crate) expires_ms: u64,
}

/// What a write of a key's session is answered.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Commit {
    /// Made: under the lease held on the key, from the version its session is at.
    Made,
    /// Refused: its fence is not the one of the lease held on the key, or none is held.
    Fenced,
    /// Refused: under the lease held, from a version another write moved since.
    Stale,
}

/// What a renewal of a lease is answered.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Renewal {
    /// Renewed: it is the lease held on the key, and it has not run out.
    Renewed,
    /// Refused: it is not the lease held on the key, or none is held.
    Superseded,
    /// Refused: it is the lease held, and it ran out at this moment, at or before the renewal.
    Expired(u64),
}

/// What a write under `fence` from the version `expected` is answered, over a key whose lease held
/// has the fence `held` (`None` when none is held) and whose session is at `current` (`0` for
/// none). The fence first: a writer that was fenced out is told so, whatever version it read.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(judged =>
    ensures
        (judged is Made) == (held == Some(fence) && current == expected),
        (judged is Fenced) == (held != Some(fence)),
        (judged is Stale) == (held == Some(fence) && current != expected),
))]
pub(crate) fn commit(held: Option<u64>, fence: u64, current: u64, expected: u64) -> Commit {
    if held != Some(fence) {
        return Commit::Fenced;
    }
    if current != expected {
        return Commit::Stale;
    }
    Commit::Made
}

/// Whether a lease that runs out at `expires_ms`, held by another owner unless `same_owner`, may be
/// taken over at `now_ms`: once it ran out; and at once by the owner that holds it, which is a
/// process that started again under its name, so that the new fence refuses the writes of its
/// previous life.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(takes =>
    ensures
        takes == (same_owner || now_ms >= expires_ms),
))]
pub(crate) fn takes_over(expires_ms: u64, same_owner: bool, now_ms: u64) -> bool {
    if same_owner {
        return true;
    }
    now_ms >= expires_ms
}

/// Whether `fence`, which a release presents, is the fence `held` of the lease held on the key
/// (`None` when none is held): only the holder of a lease gives it up.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(own =>
    ensures
        own == (held == Some(fence)),
))]
pub(crate) fn is_held(held: Option<u64>, fence: u64) -> bool {
    held == Some(fence)
}

/// What a renewal of the lease of `fence` at `now_ms` is answered, over a key whose lease held is
/// `held` (`None` when none is held): renewed only by the holder of the lease held, and only before
/// it runs out, since a holder whose lease ran out takes the key again and reads its session again.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(judged =>
    ensures
        (judged is Renewed) == (held matches Some(grant) && grant.fence == fence && now_ms < grant.expires_ms),
        (judged is Superseded) == !(held matches Some(grant) && grant.fence == fence),
        judged matches Renewal::Expired(at) ==> (held matches Some(grant) && grant.fence == fence
            && now_ms >= grant.expires_ms && at == grant.expires_ms),
))]
pub(crate) fn renewal(held: Option<Grant>, fence: u64, now_ms: u64) -> Renewal {
    match held {
        Some(grant) => {
            if grant.fence != fence {
                Renewal::Superseded
            } else if now_ms >= grant.expires_ms {
                Renewal::Expired(grant.expires_ms)
            } else {
                Renewal::Renewed
            }
        }
        None => Renewal::Superseded,
    }
}

#[cfg(verus_only)]
verus! {

/// One key of a store as `EventJournal` sees it: `st`, its `st[s]`, how many writes were made to
/// its session; and `fences`, the fence each lease granted on it was given, in the order granted.
/// `fence[s]`, how many leases were granted on the key, is how many fences there are, and the grant
/// a fence names is the one it was given at. `fences` is a history variable, which no store keeps
/// and the mapping from a store's fences to the model's needs (docs/adr/0115, D2).
pub(crate) ghost struct Keyed {
    fences: Seq<u64>,
    st: int,
}

impl Keyed {
    /// `fence[s]`.
    spec fn fence(self) -> int {
        self.fences.len() as int
    }

    /// The fences of the grants rise, so a fence names one grant; and the lease the store holds,
    /// `held`, is the last grant, so its fence names `fence[s]`.
    spec fn holds(self, held: Option<Grant>) -> bool {
        &&& forall|i: int, k: int| 0 <= i < k < self.fences.len()
            ==> #[trigger] self.fences[i] < #[trigger] self.fences[k]
        &&& held matches Some(grant) ==> self.fences.len() >= 1 && grant.fence == self.fences.last()
    }
}

/// `Take(p, s)`: a lease is granted on the key, and its fence is `fresh`, which no grant had.
spec fn step_take(keyed: Keyed, fresh: u64) -> Keyed {
    Keyed { fences: keyed.fences.push(fresh), st: keyed.st }
}

/// `Commit(t)` of a transaction that holds the key's fence, `t.f = fence[s]`, and continues its
/// state, `t.n = st[s] + 1`: the state is written.
spec fn step_commit(keyed: Keyed) -> Keyed {
    Keyed { fences: keyed.fences, st: keyed.st + 1 }
}

/// A lease asked for on a key at `now_ms`, by the owner of the lease held when `same_owner`, as
/// each store's `SessionStore::acquire` grants it: refused, nothing is written and the store takes
/// no step; granted, the store holds a lease of `fresh`, the number its counter gives next, until
/// `expires_ms`, a step of `Take`. A lease another owner holds is taken over only once it ran out
/// at `now_ms`, the clock the taker gives: the guard of `CheckpointWrite`'s `Take`, with the
/// deadline the store records as `Due(j)`.
fn an_acquire_is_a_step_of_take(
    held: Option<Grant>, same_owner: bool, now_ms: u64, fresh: u64, expires_ms: u64,
    keyed: Ghost<Keyed>,
) -> (after: (bool, Option<Grant>, Ghost<Keyed>))
    requires
        keyed@.holds(held),
        forall|i: int| 0 <= i < keyed@.fences.len() ==> #[trigger] keyed@.fences[i] < fresh,
    ensures
        after.2@.holds(after.1),
        after.0 ==> after.1 == Some(Grant { fence: fresh, expires_ms })
            && after.2@ == step_take(keyed@, fresh),
        !after.0 ==> after.1 == held && after.2@ == keyed@,
        held matches Some(grant) ==> (after.0 && !same_owner ==> now_ms >= grant.expires_ms),
{
    let granted = match held {
        Some(grant) => takes_over(grant.expires_ms, same_owner, now_ms),
        None => true,
    };
    if !granted {
        return (false, held, keyed);
    }
    proof {
        let next = keyed@.fences.push(fresh);
        assert forall|i: int, k: int| 0 <= i < k < next.len()
            implies #[trigger] next[i] < #[trigger] next[k] by {
            assert(next[i] == keyed@.fences[i]);
            if k < keyed@.fences.len() {
                assert(next[k] == keyed@.fences[k]);
            } else {
                assert(next[k] == fresh);
            }
        }
        assert(next.last() == fresh);
    }
    (true, Some(Grant { fence: fresh, expires_ms }), Ghost(step_take(keyed@, fresh)))
}

/// A write of the key's session under `fence` from the version `expected`, as each store's
/// `SessionStore::write` judges it over the lease `held` and the version `version` the session is
/// at, `next` being the version the write gives it (the number the store's counter gives next, or
/// `0` when the write removes the session). Made, it is a step of `Commit(t)` that writes: its
/// transaction holds the fence of the last grant, `t.f = fence[s]`, and continues the version the
/// store holds, `t.n = st[s] + 1`. Refused, nothing is written and the store takes no step, as a
/// `Commit` that is refused changes the writer's copy of the lease and nothing of the store.
/// `EventJournal` takes the second of the lease (the one holder of the fence has the state its
/// write continues); a store checks it, by the version.
fn a_write_is_a_step_of_commit(
    held: Option<Grant>, version: u64, fence: u64, expected: u64, next: u64, keyed: Ghost<Keyed>,
) -> (after: (Commit, u64, Ghost<Keyed>))
    requires
        keyed@.holds(held),
    ensures
        after.2@.holds(held),
        after.0 is Made ==> keyed@.fence() >= 1 && fence == keyed@.fences.last()
            && expected == version && after.1 == next && after.2@ == step_commit(keyed@),
        !(after.0 is Made) ==> after.1 == version && after.2@ == keyed@,
{
    let current = match held {
        Some(grant) => Some(grant.fence),
        None => None,
    };
    let judged = commit(current, fence, version, expected);
    match judged {
        Commit::Made => (judged, next, Ghost(step_commit(keyed@))),
        Commit::Fenced | Commit::Stale => (judged, version, keyed),
    }
}

/// A renewal of the lease of `fence` until `expires_ms`, as each store's `SessionStore::renew`
/// judges it at `now_ms`: renewed, the lease held keeps its fence and runs out at the new moment;
/// refused, nothing is written. Either is no step of the store's variables of `EventJournal`, which
/// has no renewal: the grants and the writes stay as they were, and the lease held is still the
/// last grant.
fn a_renewal_is_no_step_of_the_journal(
    held: Option<Grant>, fence: u64, now_ms: u64, expires_ms: u64, keyed: Ghost<Keyed>,
) -> (after: (Renewal, Option<Grant>))
    requires
        keyed@.holds(held),
    ensures
        keyed@.holds(after.1),
        after.0 is Renewed ==> (held matches Some(grant) && grant.fence == fence
            && now_ms < grant.expires_ms && after.1 == Some(Grant { fence, expires_ms })),
        !(after.0 is Renewed) ==> after.1 == held,
{
    let judged = renewal(held, fence, now_ms);
    match judged {
        Renewal::Renewed => (judged, Some(Grant { fence, expires_ms })),
        Renewal::Superseded | Renewal::Expired(_) => (judged, held),
    }
}

/// A release of the lease of `fence`, as each store's `SessionStore::release` judges it: of the
/// lease held, it is removed; of another, nothing is written. Either is no step of the store's
/// variables of `EventJournal`: the grants and the writes stay as they were, and a write under the
/// fence released is refused after it, as a write under a fence another grant replaced is.
fn a_release_is_no_step_of_the_journal(
    held: Option<Grant>, fence: u64, keyed: Ghost<Keyed>,
) -> (after: (bool, Option<Grant>))
    requires
        keyed@.holds(held),
    ensures
        keyed@.holds(after.1),
        after.0 ==> (held matches Some(grant) && grant.fence == fence && after.1 is None),
        !after.0 ==> after.1 == held,
{
    let current = match held {
        Some(grant) => Some(grant.fence),
        None => None,
    };
    if is_held(current, fence) {
        (true, None)
    } else {
        (false, held)
    }
}

}
