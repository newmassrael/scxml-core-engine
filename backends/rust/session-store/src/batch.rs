// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Many writes committed together (ADR 0109, D15): what [`SessionStore::write_many`] takes and
//! answers.
//!
//! Group commit, as a database writes the commits of many transactions in one flush of its log
//! and answers each after it (PostgreSQL's `commit_delay`, InnoDB's group commit): a batch is one
//! transaction of the store, made durable once, and each of its items is decided on its own, as
//! the write or the acknowledgement it is would be decided alone. An item whose version or fence
//! the store refuses is answered refused and leaves nothing of itself; the others are kept. The
//! durability of each item is the one it would have had alone; what a batch shares is the cost of
//! making it durable.
//!
//! [`SessionStore::write_many`]: crate::SessionStore::write_many

use crate::outbox::{Write, Written};
use crate::WriteError;

/// One item of a batch: a session's write, or a client's acknowledgement of its outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Batched<'a> {
    /// What [`SessionStore::write`](crate::SessionStore::write) does with `write` to the session
    /// `key` names: refused, leaving nothing of itself, unless its fence and its version are the
    /// current ones.
    Write { key: &'a str, write: Write<'a> },
    /// What [`SessionStore::acknowledge`](crate::SessionStore::acknowledge) does: delete from the
    /// outbox of `client` every entry numbered at most `through`, when `incarnation` is the one it
    /// numbers in, at `now_ms`. Never refused: an acknowledgement of another incarnation deletes
    /// nothing.
    Acknowledge {
        client: &'a str,
        incarnation: &'a str,
        through: u64,
        now_ms: u64,
    },
}

/// What an item of a batch did, once the batch is durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Committed {
    /// A write was kept: what [`SessionStore::write`](crate::SessionStore::write) would have
    /// answered.
    Written(Written),
    /// An acknowledgement deleted this many entries.
    Acknowledged(u64),
}

/// How one item of a batch was decided: kept, or refused for its version or its fence
/// ([`WriteError::Conflict`]), or failed alone ([`WriteError::Failed`]), leaving nothing of
/// itself either way.
pub type Decided = Result<Committed, WriteError>;
