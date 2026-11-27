// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a session store decides of its journal's positions (docs/adr/0114, D1 to D3; its protocol
//! is docs/formal/EventJournal.tla): the position the next entry is given, the horizon a reader
//! reads up to, what a read past a cursor answers, and whether an answer as of a position may be
//! given. Each store reads the last position it gave inside its transaction, asks these rules
//! through the module `rules`, and gives, reads and answers only what they say, so that no two
//! stores number or read one journal differently. These rules are here by themselves, over numbers
//! alone, and proved by Verus (docs/adr/0115, D13) to refine `EventJournal`'s numbering, horizon
//! and follower.
//!
//! A store's journal is `EventJournal`'s by its positions: `nextpos` is the one after the last
//! position the store gave, `log` the positions of the entries it keeps (an entry is named by its
//! position, which is given once), and `open` is empty in every state a reader sees, since a store
//! gives a position inside the transaction that keeps its entry, and one transaction at a time
//! gives positions and commits (SQLite's one write lock, `MemoryStore`'s one lock). A transaction's
//! `Begin` and its `Commit` are then one step at its commit, and one that is rolled back is no
//! step, its position given back with it. So a position given is `Begin`'s `nextpos`, above every
//! position before it, and its entry `Commit`'s with `Atomic`; the horizon is `Limit` (`Horizon`
//! on), and every position given after it is above it; a read past a cursor answers `Readable` in
//! the order of positions, which a follower takes one entry after another as steps of `Follow`, its
//! cursor at each entry's position; and an answer as of a position is given only at or below
//! `Limit`, as `Ask` asks (`ByPosition` on), so no entry kept later changes it (`Stable`).
//!
//! Taken as given, and judged elsewhere: a store reads the last position it gave and writes the
//! next one in the transaction that keeps the entry it numbers (held to that by the conformance
//! kit); and a read answers the entries it holds at the positions these rules give, in the order of
//! their positions (the stores' queries, held to the kit).
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes of
//! this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// The positions a read answers: from `first` to `last`, both included.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) first: u64,
    pub(crate) last: u64,
}

/// The position a store gives the next entry of its journal, after `issued`, the last one it gave
/// (`0` before any): the one after it, above every position given before; `None` when `issued` is
/// the last position there is, so that none is given twice.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(given =>
    ensures
        given matches Some(position) ==> issued < u64::MAX && position == issued + 1,
        given is None ==> issued == u64::MAX,
))]
pub(crate) fn next_position(issued: u64) -> Option<u64> {
    if issued == u64::MAX {
        return None;
    }
    Some(issued + 1)
}

/// The horizon of a journal whose last position given is `issued`, over a store that gives a
/// position inside the transaction that keeps its entry, one transaction at a time: that position,
/// since each position at or below it was committed or given back with its transaction, and each
/// one given later is above it.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(limit =>
    ensures
        limit == issued,
))]
pub(crate) fn horizon(issued: u64) -> u64 {
    issued
}

/// What a read past the position `after` answers, over a journal whose horizon is `horizon`: the
/// positions from the one after `after` to the horizon; `None` when the cursor is at the horizon or
/// past it, where nothing is there to read yet.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(read =>
    ensures
        read matches Some(span) ==> after < horizon && span.first == after + 1
            && span.last == horizon,
        read is None ==> after >= horizon,
))]
pub(crate) fn readable(after: u64, horizon: u64) -> Option<Span> {
    if after >= horizon {
        return None;
    }
    Some(Span {
        first: after + 1,
        last: horizon,
    })
}

/// Whether an answer as of `position` may be given over a journal whose horizon is `horizon`: at
/// or below it, where no entry can still appear, so the answer is the same whenever it is asked.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(admitted =>
    ensures
        admitted == (position <= horizon),
))]
pub(crate) fn settled(position: u64, horizon: u64) -> bool {
    position <= horizon
}

#[cfg(verus_only)]
verus! {

/// A store's journal as `EventJournal` sees it, by the positions alone: `log`, the positions of the
/// entries that are visible; `open`, those of the transactions under way; and `nextpos`, the
/// position the next transaction to begin is given. An entry is named by its position, which is
/// given once, so a set of positions is a set of entries.
pub(crate) ghost struct Positions {
    log: Set<int>,
    open: Set<int>,
    nextpos: int,
}

impl Positions {
    /// The journal of a store whose last position given is `issued`: the next position is the one
    /// after it, and every position the journal holds, visible or under way, was given, so it is at
    /// least 1 and below the next; no transaction under way has the position of an entry.
    spec fn holds(self, issued: u64) -> bool {
        &&& self.nextpos == issued as int + 1
        &&& forall|p: int| #[trigger] self.log.contains(p) ==> 1 <= p && p < self.nextpos
        &&& forall|p: int| #[trigger] self.open.contains(p)
            ==> 1 <= p && p < self.nextpos && !self.log.contains(p)
    }

    /// The least position of a transaction under way.
    spec fn lowest_open(self) -> int {
        choose|q: int| #[trigger] self.open.contains(q)
            && forall|r: int| #[trigger] self.open.contains(r) ==> q <= r
    }

    /// `Limit`, with `Horizon` on: below the least transaction under way, or the last position
    /// given when none is.
    spec fn limit(self) -> int {
        if self.open == Set::<int>::empty() {
            self.nextpos - 1
        } else {
            self.lowest_open() - 1
        }
    }

    /// `Readable`, for a follower whose cursor is `cursor`.
    spec fn readable(self, cursor: int) -> Set<int> {
        self.log.filter(|p: int| cursor < p && p <= self.limit())
    }

    /// `AsOf(r)`, with `ByPosition` on.
    spec fn as_of(self, r: int) -> Set<int> {
        self.log.filter(|p: int| p <= r)
    }
}

/// `Begin(p, s)`: a transaction is given `nextpos`, and the next position is the one after it.
spec fn step_begin(journal: Positions) -> Positions {
    Positions {
        log: journal.log,
        open: journal.open.insert(journal.nextpos),
        nextpos: journal.nextpos + 1,
    }
}

/// `Commit(t)` of the transaction at `position`, with `Atomic` on and its fence the lease's (the
/// module `lease`'s proof): its entry is visible, and it is under way no more.
spec fn step_commit(journal: Positions, position: int) -> Positions {
    Positions {
        log: journal.log.insert(position),
        open: journal.open.remove(position),
        nextpos: journal.nextpos,
    }
}

/// A position given to an entry a store keeps, as `fact_in` of the SQLite store and the write and
/// the observation of `MemoryStore` give it from the last position given, `issued`: `Begin` of the
/// transaction that takes it and then its `Commit`, one step at its commit, after which nothing is
/// under way again. The position is above the horizon every reader read up to before it, and the
/// horizon never falls: a follower whose cursor was at or below the horizon may read what it could
/// and the new entry, so it passes none (`NonePassed`), and an answer as of a position at or below
/// the horizon is the same after it (`Stable`). Refused, at the last position there is, nothing is
/// given, and the store keeps nothing.
fn a_position_given_is_a_step_of_begin_then_commit(
    issued: u64, journal: Ghost<Positions>,
) -> (after: (Option<u64>, u64, Ghost<Positions>))
    requires
        journal@.holds(issued),
        journal@.open == Set::<int>::empty(),
    ensures
        after.2@.holds(after.1),
        after.2@.open == Set::<int>::empty(),
        after.2@.limit() >= journal@.limit(),
        forall|r: int| r <= journal@.limit() ==> #[trigger] after.2@.as_of(r) == journal@.as_of(r),
        after.0 matches Some(position) ==> position as int == journal@.nextpos
            && position as int > journal@.limit()
            && !journal@.log.contains(position as int)
            && after.1 == position
            && after.2@ == step_commit(step_begin(journal@), position as int)
            && forall|c: int| c <= journal@.limit()
                ==> #[trigger] after.2@.readable(c) == journal@.readable(c).insert(position as int),
        after.0 is None ==> issued == u64::MAX && after.1 == issued && after.2@ == journal@,
{
    let given = next_position(issued);
    match given {
        Some(position) => {
            let ghost next = step_commit(step_begin(journal@), position as int);
            proof {
                assert(next.open =~= Set::<int>::empty());
                assert forall|r: int| r <= journal@.limit()
                    implies #[trigger] next.as_of(r) == journal@.as_of(r) by {
                    assert(next.as_of(r) =~= journal@.as_of(r));
                }
                assert forall|c: int| c <= journal@.limit()
                    implies #[trigger] next.readable(c)
                        == journal@.readable(c).insert(position as int) by {
                    assert(next.readable(c) =~= journal@.readable(c).insert(position as int));
                }
            }
            (given, position, Ghost(next))
        }
        None => (given, issued, journal),
    }
}

/// A read past the cursor `after`, as `after`, `stream` and `position_at` of each store make it
/// over the last position given, `issued`: the horizon it answers is `Limit`, at or above every
/// entry the store holds, and the positions it reads are those of `Readable` among the entries the
/// store holds, none when nothing is readable.
fn a_read_answers_what_is_readable(
    after: u64, issued: u64, journal: Ghost<Positions>,
) -> (read: (u64, Option<Span>))
    requires
        journal@.holds(issued),
        journal@.open == Set::<int>::empty(),
    ensures
        read.0 as int == journal@.limit(),
        forall|p: int| #[trigger] journal@.log.contains(p) ==> p <= read.0,
        read.1 matches Some(span) ==> forall|p: int|
            #[trigger] journal@.readable(after as int).contains(p)
                == (journal@.log.contains(p) && span.first <= p && p <= span.last),
        read.1 is None ==> journal@.readable(after as int) == Set::<int>::empty(),
{
    let limit = horizon(issued);
    let span = readable(after, limit);
    proof {
        assert(journal@.limit() == issued as int);
        if span is None {
            assert(journal@.readable(after as int) =~= Set::<int>::empty());
        }
    }
    (limit, span)
}

/// `Follow`: the entry a follower takes is `First`, the readable one of least position, and what it
/// may read after it, its cursor moved to that position, is what it could read before but that
/// entry. So a page a store answers in the order of positions, taken one entry after another, is
/// steps of `Follow`, each entry once (`FollowedOnce`), and a read on from the last entry of a page
/// misses none and repeats none.
proof fn a_follower_takes_the_first_and_reads_on_past_it(
    journal: Positions, cursor: int, first: int,
)
    requires
        journal.readable(cursor).contains(first),
        forall|q: int| #[trigger] journal.readable(cursor).contains(q) ==> first <= q,
    ensures
        journal.readable(first) == journal.readable(cursor).remove(first),
{
    assert(journal.readable(first) =~= journal.readable(cursor).remove(first));
}

/// An answer as of `position`, as `AsOf::admitted` judges it for each store's `as_of` over the last
/// position given, `issued`: given only at or below `Limit`, as `Ask` asks, which no position
/// given later reaches.
fn an_answer_as_of_a_position_is_asked_at_or_below_the_horizon(
    position: u64, issued: u64, journal: Ghost<Positions>,
) -> (admitted: bool)
    requires
        journal@.holds(issued),
        journal@.open == Set::<int>::empty(),
    ensures
        admitted == (position as int <= journal@.limit()),
{
    settled(position, horizon(issued))
}

}
