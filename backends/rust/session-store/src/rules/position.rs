// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a session store decides of its journal's positions (docs/adr/0114, D1 to D3; its protocol is
//! docs/formal/EventJournal.tla): the position the next entry is given, the horizon a reader reads
//! up to, what a read past a cursor answers, and whether an answer as of a position may be given.
//! Each store reads the last position it gave inside its transaction, asks these rules through the
//! module `rules`, and gives, reads and answers only what they say, so that no two stores number or
//! read one journal differently.
//!
//! A store's journal is `EventJournal`'s by its positions: `nextpos` is the one after the last
//! position the store gave, `log` the positions of the entries it keeps (an entry is named by its
//! position, which is given once), and `open` is empty in every state a reader sees, since a store
//! gives a position inside the transaction that keeps its entry, and one transaction at a time gives
//! positions and commits (SQLite's one write lock, `MemoryStore`'s one lock). A transaction's
//! `Begin` and its `Commit` are then one step at its commit, and one that is rolled back is no step,
//! its position given back with it. So a position given is `Begin`'s `nextpos`, above every position
//! before it, and its entry `Commit`'s with `Atomic`; the horizon is `Limit` (`Horizon` on), and
//! every position given after it is above it; a read past a cursor answers `Readable` in the order
//! of positions, which a follower takes one entry after another as steps of `Follow`, its cursor at
//! each entry's position; and an answer as of a position is given only at or below `Limit`, as
//! `Ask` asks (`ByPosition` on), so no entry kept later changes it (`Stable`).
//!
//! Taken as given, and judged elsewhere: a store reads the last position it gave and writes the next
//! one in the transaction that keeps the entry it numbers (held to that by the conformance kit); and
//! a read answers the entries it holds at the positions these rules give, in the order of their
//! positions (the stores' queries, held to the kit).

/// The positions a read answers: from `first` to `last`, both included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) first: u64,
    pub(crate) last: u64,
}

/// The position a store gives the next entry of its journal, after `issued`, the last one it gave
/// (`0` before any): the one after it, above every position given before; `None` when `issued` is
/// the last position there is, so that none is given twice.
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
pub(crate) fn horizon(issued: u64) -> u64 {
    issued
}

/// What a read past the position `after` answers, over a journal whose horizon is `horizon`: the
/// positions from the one after `after` to the horizon; `None` when the cursor is at the horizon or
/// past it, where nothing is there to read yet.
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
pub(crate) fn settled(position: u64, horizon: u64) -> bool {
    position <= horizon
}
