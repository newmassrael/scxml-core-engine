// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a host keeps its audit log (ADR 0109, decision D14; docs/SCE_HOST_FEATURES.md section 15;
//! the chain and its checkpoints are docs/SCE_AUDIT_LOG.md).
//!
//! An audit log is a set of **streams**, each an append-only sequence of rows numbered 1, 2, 3
//! and on. A row holds when it was appended, its entry, and the **chain**: the SHA-256 of the
//! chain before it and of the row, which its writer computes (`sce_rust_crypto::audit`). A
//! **checkpoint** is a signature over a stream, a sequence and the chain there, kept beside the
//! rows; a reader verifies a stream from any checkpoint forward.
//!
//! The store keeps rows and never computes or checks a digest or a signature: it keeps what it is
//! given, in order. What it decides is what makes the log a log:
//!
//! * a row is appended only after the stream's **head**, the last row appended, so two writers
//!   cannot both append one number ([`AuditAppend::Moved`]);
//! * a row is never changed, and is deleted only by retention, a whole **segment** at a time: every
//!   row up to a sequence a checkpoint is kept at, with the checkpoints before it, so the
//!   checkpoint at the cut anchors what remains ([`AuditStore::audit_drop_through`]);
//! * the head outlives retention, so the next row is numbered on and chained on;
//! * an append under the identity of an invocation (docs/SCE_HOST_FEATURES.md section 1) keeps
//!   the identity in the append's transaction, and an append of an identity kept answers the
//!   sequence it was appended at and appends nothing.
//!
//! The methods carry the prefix `audit_` because the stores that keep the log keep sessions and
//! values too, whose contracts name an `append` and a `get` of their own.
//!
//! * [`crate::MemoryStore`] keeps the log in the process.
//! * `SqliteStore` (feature `sqlite`) keeps it in the session store's file (layout 7).
//! * [`crate::audit_conformance`] is what every implementation is held to.

use std::error::Error;
use std::fmt;

use crate::kv::Identity;
use crate::Failure;

/// The bytes of a chain value: the whole output of SHA-256.
pub const CHAIN_BYTES: usize = 32;

/// A chain value.
pub type Chain = [u8; CHAIN_BYTES];

/// The chain before the first row of every stream: 32 zero bytes.
pub const GENESIS: Chain = [0; CHAIN_BYTES];

/// One row of a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRow {
    /// The stream it belongs to.
    pub stream: String,
    /// Its number in the stream, from 1.
    pub sequence: u64,
    /// When it was appended, in milliseconds since the Unix epoch.
    pub at_ms: u64,
    /// Its entry's bytes, which the store keeps and never reads: the redacted record, or, in a
    /// store that wraps another and seals what it keeps (ADR 0109, D8), its sealed record.
    pub entry: Vec<u8>,
    /// The chain at this row, as its writer computed it.
    pub chain: Chain,
}

/// The last row appended to a stream: its sequence and its chain, `0` and [`GENESIS`] for a
/// stream nothing was appended to. Retention deletes rows and leaves the head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditHead {
    pub sequence: u64,
    pub chain: Chain,
}

impl AuditHead {
    /// The head of a stream nothing was appended to.
    pub const EMPTY: AuditHead = AuditHead {
        sequence: 0,
        chain: GENESIS,
    };
}

/// A signature over a stream at a sequence: the `COSE_Sign1` its writer made over the stream, the
/// sequence and the chain there (docs/SCE_AUDIT_LOG.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCheckpoint {
    pub stream: String,
    pub sequence: u64,
    pub chain: Chain,
    /// The `COSE_Sign1`, which the store keeps and never reads.
    pub signature: Vec<u8>,
}

/// What an append did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAppend {
    /// The row was appended, at the sequence it named.
    Appended,
    /// The identity was appended before, at `sequence`; nothing was appended.
    Repeated { sequence: u64 },
    /// The stream's head is `head`, not the one the row follows: another append came first, and
    /// nothing was appended. The writer chains its row on `head` and appends again.
    Moved { head: AuditHead },
}

/// Why a store refused to delete a segment: no checkpoint is kept at the sequence it would end at,
/// and none would anchor what remained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoCheckpointAt {
    pub stream: String,
    pub sequence: u64,
}

impl fmt::Display for NoCheckpointAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no checkpoint of the stream '{}' is kept at {}, so a segment cannot end there",
            self.stream, self.sequence
        )
    }
}

impl Error for NoCheckpointAt {}

/// A store of audit streams. See the module documentation for what it keeps and why.
///
/// Every method is one transaction: it reads and writes what it names, or, when it returns an
/// error, writes nothing. A handle may be shared between threads; whether handles on one store may
/// live in several processes is the implementation's to say.
pub trait AuditStore: Send + Sync {
    /// The head of `stream`: [`AuditHead::EMPTY`] for one nothing was appended to.
    fn audit_head(&self, stream: &str) -> Result<AuditHead, Failure>;

    /// Append `row` to its stream, and `checkpoint`, a signature over the stream at the row, with
    /// it, in one transaction, when the row's sequence is one past the stream's head; answer
    /// [`AuditAppend::Moved`], appending nothing, when it is not. With `identity`, the identity is
    /// kept with the row, and an append of an identity kept answers [`AuditAppend::Repeated`] with
    /// its sequence, appending nothing, whatever the head is.
    fn audit_append(
        &self,
        row: &AuditRow,
        checkpoint: Option<&[u8]>,
        identity: Option<&Identity<'_>>,
    ) -> Result<AuditAppend, Failure>;

    /// Keep `checkpoint` unless one of its stream and sequence is kept, and answer whether it did.
    /// A checkpoint made at a stop, of the head, is kept this way.
    fn audit_checkpoint(&self, checkpoint: &AuditCheckpoint) -> Result<bool, Failure>;

    /// At most `limit` rows of `stream` numbered past `after`, in their order.
    fn audit_rows(&self, stream: &str, after: u64, limit: usize) -> Result<Vec<AuditRow>, Failure>;

    /// Every checkpoint of `stream` the store keeps, in the order of their sequences.
    fn audit_checkpoints(&self, stream: &str) -> Result<Vec<AuditCheckpoint>, Failure>;

    /// Delete the segment of `stream` that ends at `through`: every row numbered at most `through`
    /// and every checkpoint before it, keeping the checkpoint at `through`, which anchors what
    /// remains. Refused with [`NoCheckpointAt`], deleting nothing, when no checkpoint is kept at
    /// `through`. Answers how many rows it deleted.
    fn audit_drop_through(&self, stream: &str, through: u64) -> Result<u64, Failure>;

    /// The sequence the identity `identity` was appended at, if it was.
    fn audit_first(&self, identity: &Identity<'_>) -> Result<Option<u64>, Failure>;
}

/// The decisions every audit store makes, written once.
pub(crate) mod rules {
    use super::{AuditAppend, AuditHead, AuditRow};

    /// Whether `row` follows `head`: the refusal an append answers when it does not.
    pub(crate) fn follows(head: &AuditHead, row: &AuditRow) -> Result<(), AuditAppend> {
        if row.sequence == head.sequence.saturating_add(1) && head.sequence != u64::MAX {
            Ok(())
        } else {
            Err(AuditAppend::Moved { head: *head })
        }
    }
}
