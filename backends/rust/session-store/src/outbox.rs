// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a server keeps for a client of a durable `https` binding until the client acknowledges
//! it: the client's outbox (ADR 0109, D10; SCE_MESH.md sections 10.11 and 22.3).
//!
//! A transactional outbox: what a session sends a client is appended to the client's outbox by
//! the same store transaction that writes the session's checkpoint ([`SessionStore::write`]), so
//! either both are durable or neither is, and the endpoint holds nothing for what was appended
//! once the write answered. An acknowledgement deletes what it covers
//! ([`SessionStore::acknowledge`]); an entry older than the binding's `retain_ms` is deleted by
//! an expiry ([`SessionStore::expire`]), which writes a [`Note`] for the session that sent it in
//! the same transaction, so that session is told at its next run (SCE_MESH.md section 16.7, row
//! 15) and takes the note by the write that holds what it made of it.
//!
//! An outbox numbers its entries 1, 2, 3 and on, in an incarnation: the text the first append or
//! the first exchange gave it when the store held nothing for the client. Both are kept with the
//! entries, so a restart keeps them (section 22.3: "one that restores its mailboxes keeps
//! theirs").
//!
//! [`SessionStore::write`]: crate::SessionStore::write
//! [`SessionStore::acknowledge`]: crate::SessionStore::acknowledge
//! [`SessionStore::expire`]: crate::SessionStore::expire

use std::error::Error;
use std::fmt;

/// One envelope a write appends to a client's outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Append<'a> {
    /// The client whose outbox keeps it: the name its exchanges post under.
    pub client: &'a str,
    /// The incarnation the client's outbox numbers in when the store holds none for the client
    /// yet; an outbox the store holds keeps its own.
    pub incarnation: &'a str,
    /// The session that sent it, which an expiry writes its note for; `None` for an envelope no
    /// session sent (an answer the host made in a session's place).
    pub sender: Option<&'a str>,
    /// The envelope's bytes, as the client receives them.
    pub envelope: &'a [u8],
}

/// Where an [`Append`] was kept: its client, the incarnation the outbox numbers in, and its
/// number there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Appended {
    pub client: String,
    pub incarnation: String,
    pub number: u64,
}

/// An envelope an outbox holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEntry {
    /// Its number in the outbox's incarnation; the entries of an outbox are consecutive.
    pub number: u64,
    /// When it was appended, in milliseconds since the Unix epoch: what its expiry counts from.
    pub at_ms: u64,
    /// The session that sent it, if one did.
    pub sender: Option<String>,
    pub envelope: Vec<u8>,
}

/// A client's outbox: the incarnation it numbers in, the number its next entry takes, and what it
/// holds, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outbox {
    pub incarnation: String,
    pub next: u64,
    pub entries: Vec<OutboxEntry>,
}

/// An entry an expiry deleted, with the client whose outbox held it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expired {
    pub client: String,
    pub entry: OutboxEntry,
}

/// What [`SessionStore::expire`](crate::SessionStore::expire) did to one client's outbox.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expiry {
    /// The entries it deleted, oldest first.
    pub expired: Vec<Expired>,
    /// Whether it removed the outbox itself: it held nothing, and nothing was appended to it or
    /// acknowledged of it since the moment the expiry counts from. The next append or exchange
    /// opens it again in a new incarnation, numbering from 1.
    pub closed: bool,
}

/// What an expiry wrote for the session that sent an entry it deleted: the session is told at its
/// next run, and takes the note by the write that holds what it made of it
/// ([`Write::notes_through`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// A number of the store's counter, above every note written before it.
    pub id: u64,
    /// The client the entry was for.
    pub client: String,
    /// The entry's number in the client's outbox.
    pub number: u64,
    /// When the entry was appended.
    pub at_ms: u64,
    /// The envelope that was not delivered.
    pub envelope: Vec<u8>,
}

/// What a write does to the session it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change<'a> {
    /// Keep `state` as the session, with `earliest_due` as the earliest moment it acts on its own
    /// once restored (`None` when it waits for nothing).
    Keep {
        state: &'a [u8],
        earliest_due: Option<u64>,
    },
    /// Remove the session, and every note written for it.
    Remove,
}

/// One write of a session store: what it does to the session, under which version and fence, and
/// what it appends to outboxes and takes of the session's notes, all in one transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write<'a> {
    pub change: Change<'a>,
    /// The version the write was made from, `0` for a session the store holds none of.
    pub expected_version: u64,
    /// The fence of the lease held on the session's key.
    pub fence: u64,
    /// The envelopes appended to their clients' outboxes, in the order given.
    pub outbox: &'a [Append<'a>],
    /// Every note of the session numbered at most this is removed: the notes whose rows the state
    /// written holds as raised. `0` takes none.
    pub notes_through: u64,
    /// The moment the appends are kept at, in milliseconds since the Unix epoch.
    pub now_ms: u64,
    /// The ids of the sessions (`_sessionid`) a removal ends: the root and every child its last
    /// state held. A store that keeps the first results of the key-value feature too (module
    /// [`crate::kv`]) deletes every one kept under their identities in the removal's transaction,
    /// and one that keeps an audit log (module [`crate::audit`]) the identities its appends were
    /// kept under, never a row. A keep takes none.
    pub sessions: &'a [&'a str],
}

impl<'a> Write<'a> {
    /// A write that keeps `state` and does nothing else.
    pub fn put(
        state: &'a [u8],
        earliest_due: Option<u64>,
        expected_version: u64,
        fence: u64,
    ) -> Self {
        Self {
            change: Change::Keep {
                state,
                earliest_due,
            },
            expected_version,
            fence,
            outbox: &[],
            notes_through: 0,
            now_ms: 0,
            sessions: &[],
        }
    }

    /// A write that removes the session and does nothing else.
    pub fn delete(expected_version: u64, fence: u64) -> Self {
        Self {
            change: Change::Remove,
            expected_version,
            fence,
            outbox: &[],
            notes_through: 0,
            now_ms: 0,
            sessions: &[],
        }
    }
}

/// What a write answered: the session's new version (`0` once removed) and where each of its
/// appends was kept, in their order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub version: u64,
    pub appended: Vec<Appended>,
}

/// The refusal of a store that keeps no outbox: it serves no durable `https` binding. The store
/// contract's outbox calls answer it by default; every store of this crate keeps one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoOutbox;

impl fmt::Display for NoOutbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "the store keeps no outbox, so it serves no binding with `receipt: durable` \
             (SCE_MESH.md section 10.11)",
        )
    }
}

impl Error for NoOutbox {}
