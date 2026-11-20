// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A durable audit log's chain and signed checkpoints (ADR 0109, D14; the bytes are fixed by
//! docs/SCE_AUDIT_LOG.md): append-only rows, each chained on the one before, a signature over the
//! chain every so many rows and at a stop, a reader that verifies a stream from a signed checkpoint
//! forward, and retention that deletes whole segments.
//!
//! - **The chain.** Row `n` of a stream holds `chain_n = SHA-256(chain_(n-1) || E_n)`, `chain_0`
//!   32 zero bytes and `E_n` the deterministic CBOR array `[stream, n, at_ms, entry]` (text, uint,
//!   uint, bytes) ([`chained`]). A row changed, removed or moved changes every chain after it.
//! - **A checkpoint** is a tagged `COSE_Sign1` (RFC 9052) under the host's audit key
//!   ([`crate::keyring::AUDIT_SIGNING_KEY_ID`], EdDSA on Ed25519, the id as `kid`) of the
//!   deterministic CBOR array `["sce-audit-1", stream, n, chain_n]` ([`checkpoint_payload`]),
//!   written with row `n` every `checkpoint_every` rows and of the head at a stop. A truncation of
//!   the stream before a checkpoint is a checkpoint past the last row.
//! - **A reader** ([`verify`]) checks every checkpoint's signature, anchors the walk at the first
//!   row's predecessor (the genesis for a stream that starts at 1, the signed checkpoint a
//!   retention left at its cut otherwise), and recomputes every chain forward, holding each
//!   stored chain and each checkpoint to it.
//! - **Retention** ([`retain`]) deletes the rows older than a moment by whole segments: through
//!   the latest checkpoint before the first row it keeps, so that checkpoint anchors the rest.
//!
//! The store keeps what it is given (`sce_rust_session_store::AuditStore`); this module computes
//! and checks. What an entry holds, its redaction included, is its writer's.

use std::fmt;

use sce_forge_runtime::cbor::{write_array_head, write_bytes, write_text, write_uint};
use sce_forge_runtime::codec::VecSink;
use sce_rust_session_store::audit::GENESIS;
use sce_rust_session_store::{
    AuditAppend, AuditCheckpoint, AuditRow, AuditStore, Chain, Failure, Identity,
};
use sha2::{Digest, Sha256};

use crate::cose::{put, sign1, verify1, TrustedKey, KEY_BYTES};
use crate::keyring::{KeyError, Keyring, AUDIT_SIGNING_KEY_ID};
use crate::random::RandomSource;

/// The context a checkpoint's payload begins with: the version of its form.
pub const CHECKPOINT_CONTEXT: &str = "sce-audit-1";

/// How many rows one page of a pass over a stream reads.
const PAGE: usize = 512;

/// How many times an append chains its row on a head another append moved before it gives up.
const APPEND_ATTEMPTS: usize = 16;

/// `E_n`, the bytes row `sequence` of `stream` is chained by.
pub fn entry_item(stream: &str, sequence: u64, at_ms: u64, entry: &[u8]) -> Vec<u8> {
    let mut item = Vec::with_capacity(stream.len() + entry.len() + 32);
    let mut w = VecSink::new(&mut item);
    put(write_array_head(&mut w, 4));
    put(write_text(&mut w, stream));
    put(write_uint(&mut w, sequence));
    put(write_uint(&mut w, at_ms));
    put(write_bytes(&mut w, entry));
    item
}

/// The chain of row `sequence` of `stream`, appended at `at_ms` with `entry`, after `previous`.
pub fn chained(previous: &Chain, stream: &str, sequence: u64, at_ms: u64, entry: &[u8]) -> Chain {
    let mut hasher = Sha256::new();
    hasher.update(previous);
    hasher.update(entry_item(stream, sequence, at_ms, entry));
    hasher.finalize().into()
}

/// What a checkpoint of `stream` at `sequence`, whose chain is `chain`, signs.
pub fn checkpoint_payload(stream: &str, sequence: u64, chain: &Chain) -> Vec<u8> {
    let mut payload = Vec::with_capacity(stream.len() + 64);
    let mut w = VecSink::new(&mut payload);
    put(write_array_head(&mut w, 4));
    put(write_text(&mut w, CHECKPOINT_CONTEXT));
    put(write_text(&mut w, stream));
    put(write_uint(&mut w, sequence));
    put(write_bytes(&mut w, chain));
    payload
}

/// The `COSE_Sign1` of a checkpoint of `stream` at `sequence`, whose chain is `chain`, signed with
/// the Ed25519 key of `seed`.
pub fn sign_checkpoint(
    seed: &[u8; KEY_BYTES],
    stream: &str,
    sequence: u64,
    chain: &Chain,
) -> Vec<u8> {
    sign1(
        seed,
        AUDIT_SIGNING_KEY_ID.as_bytes(),
        &checkpoint_payload(stream, sequence, chain),
        &[],
    )
}

/// Whether `checkpoint` is signed by the audit key whose public key is `public_key`, over its own
/// stream, sequence and chain.
pub fn checkpoint_verifies(public_key: &[u8; KEY_BYTES], checkpoint: &AuditCheckpoint) -> bool {
    let signer = [TrustedKey {
        id: AUDIT_SIGNING_KEY_ID.to_string(),
        public_key: *public_key,
    }];
    verify1(&signer, &checkpoint.signature, &[]).is_some_and(|(_, payload)| {
        payload == checkpoint_payload(&checkpoint.stream, checkpoint.sequence, &checkpoint.chain)
    })
}

/// Why an append or a checkpoint was not made.
#[derive(Debug)]
pub enum AuditError {
    /// The audit key could not be used: retired (`key_retired`), or the key ring failed.
    Key(KeyError),
    /// The store failed.
    Store(Failure),
    /// Other appends moved the stream's head every time this one chained on it.
    Busy,
}

impl AuditError {
    /// The failure code of the operation that met this (docs/SCE_HOST_FEATURES.md section 15).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Key(KeyError::Retired) => "key_retired",
            Self::Key(_) | Self::Store(_) | Self::Busy => "unavailable",
        }
    }
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(error) => write!(f, "the audit key: {error}"),
            Self::Store(failure) => write!(f, "{failure}"),
            Self::Busy => f.write_str("the stream's head moved under every attempt to append"),
        }
    }
}

impl std::error::Error for AuditError {}

impl From<KeyError> for AuditError {
    fn from(error: KeyError) -> Self {
        Self::Key(error)
    }
}

impl From<Failure> for AuditError {
    fn from(failure: Failure) -> Self {
        Self::Store(failure)
    }
}

/// What an append is made with: the store, the key ring the audit key is in, the random source a
/// first audit key is drawn from, and the moment.
pub struct Writer<'a> {
    pub store: &'a dyn AuditStore,
    pub keyring: &'a Keyring,
    pub random: &'a mut dyn RandomSource,
    pub now_ms: u64,
}

impl Writer<'_> {
    /// Append `entry` to `stream`, chained on its head, with a checkpoint when its sequence is a
    /// multiple of `checkpoint_every` (none for `0`), all in one transaction of the store, and
    /// answer its sequence. Under `identity`, an append of an identity the store keeps answers
    /// the sequence it was appended at and appends nothing.
    pub fn append(
        &mut self,
        stream: &str,
        entry: &[u8],
        checkpoint_every: u64,
        identity: Option<&Identity<'_>>,
    ) -> Result<u64, AuditError> {
        if let Some(identity) = identity {
            if let Some(sequence) = self.store.audit_first(identity)? {
                return Ok(sequence);
            }
        }
        for _ in 0..APPEND_ATTEMPTS {
            let head = self.store.audit_head(stream)?;
            let sequence = head.sequence.checked_add(1).ok_or(AuditError::Busy)?;
            let chain = chained(&head.chain, stream, sequence, self.now_ms, entry);
            let signature = if checkpoint_every != 0 && sequence % checkpoint_every == 0 {
                let seed = self.keyring.audit_seed(self.now_ms, &mut *self.random)?;
                Some(sign_checkpoint(&seed, stream, sequence, &chain))
            } else {
                None
            };
            let row = AuditRow {
                stream: stream.to_string(),
                sequence,
                at_ms: self.now_ms,
                entry: entry.to_vec(),
                chain,
            };
            match self
                .store
                .audit_append(&row, signature.as_deref(), identity)?
            {
                AuditAppend::Appended => return Ok(sequence),
                AuditAppend::Repeated { sequence } => return Ok(sequence),
                AuditAppend::Moved { .. } => continue,
            }
        }
        Err(AuditError::Busy)
    }

    /// Sign the head of `stream` unless a checkpoint is kept at it, or nothing was appended: what a
    /// host does for each stream at a stop, so a truncation of what came after the last periodic
    /// checkpoint is told. Answers the sequence it signed, if it signed one.
    pub fn checkpoint_head(&mut self, stream: &str) -> Result<Option<u64>, AuditError> {
        let head = self.store.audit_head(stream)?;
        if head.sequence == 0 {
            return Ok(None);
        }
        let kept = self
            .store
            .audit_checkpoints(stream)?
            .iter()
            .any(|checkpoint| checkpoint.sequence == head.sequence);
        if kept {
            return Ok(None);
        }
        let seed = self.keyring.audit_seed(self.now_ms, &mut *self.random)?;
        let checkpoint = AuditCheckpoint {
            stream: stream.to_string(),
            sequence: head.sequence,
            chain: head.chain,
            signature: sign_checkpoint(&seed, stream, head.sequence, &head.chain),
        };
        Ok(self
            .store
            .audit_checkpoint(&checkpoint)?
            .then_some(head.sequence))
    }
}

/// What a reader found a stream to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every row from `from` through `through` chains on its anchor, and every checkpoint, of which
    /// `checkpoints` were read, is signed and holds the chain at its sequence. `from` is past
    /// `through` for a stream that holds no row.
    Verified {
        from: u64,
        through: u64,
        checkpoints: u64,
    },
    /// The checkpoint at `sequence` is not signed by the audit key over what it holds.
    Forged { sequence: u64 },
    /// The first row the store holds, `sequence`, follows no checkpoint and is not the first of the
    /// stream: nothing anchors it.
    Unanchored { sequence: u64 },
    /// The row after `after` is not the next number: `found` is, or none is.
    Missing { after: u64, found: Option<u64> },
    /// Row `sequence` does not chain on the rows before it: it, or one before it the checkpoints
    /// do not hold, was changed.
    Changed { sequence: u64 },
    /// A checkpoint, or the head, is at `sequence`, past the last row the store holds, `last`: rows
    /// were cut from the end.
    Truncated { sequence: u64, last: u64 },
}

/// Read `stream` from the store and hold it to its chain and its checkpoints under the audit key
/// whose public key is `public_key`.
pub fn verify(
    store: &dyn AuditStore,
    stream: &str,
    public_key: &[u8; KEY_BYTES],
) -> Result<Verdict, Failure> {
    let checkpoints = store.audit_checkpoints(stream)?;
    if let Some(forged) = checkpoints
        .iter()
        .find(|checkpoint| !checkpoint_verifies(public_key, checkpoint))
    {
        return Ok(Verdict::Forged {
            sequence: forged.sequence,
        });
    }
    let head = store.audit_head(stream)?;
    let first = store.audit_rows(stream, 0, 1)?;
    // The anchor: the genesis before row 1, or the checkpoint a retention left at its cut.
    let (mut sequence, mut chain) = match first.first() {
        None => match checkpoints.last() {
            None => (0, GENESIS),
            Some(last) => (last.sequence, last.chain),
        },
        Some(row) if row.sequence == 1 => (0, GENESIS),
        Some(row) => {
            let before = row.sequence - 1;
            match checkpoints
                .iter()
                .find(|checkpoint| checkpoint.sequence == before)
            {
                Some(anchor) => (anchor.sequence, anchor.chain),
                None => {
                    return Ok(Verdict::Unanchored {
                        sequence: row.sequence,
                    })
                }
            }
        }
    };
    let from = sequence + 1;
    loop {
        let page = store.audit_rows(stream, sequence, PAGE)?;
        if page.is_empty() {
            break;
        }
        for row in page {
            if row.sequence != sequence + 1 {
                return Ok(Verdict::Missing {
                    after: sequence,
                    found: Some(row.sequence),
                });
            }
            let expected = chained(&chain, stream, row.sequence, row.at_ms, &row.entry);
            if expected != row.chain {
                return Ok(Verdict::Changed {
                    sequence: row.sequence,
                });
            }
            if let Some(checkpoint) = checkpoints
                .iter()
                .find(|checkpoint| checkpoint.sequence == row.sequence)
            {
                if checkpoint.chain != expected {
                    return Ok(Verdict::Changed {
                        sequence: row.sequence,
                    });
                }
            }
            sequence = row.sequence;
            chain = expected;
        }
    }
    // A checkpoint past the last row, or a head past it, is a cut from the end.
    let latest = checkpoints
        .iter()
        .map(|checkpoint| checkpoint.sequence)
        .chain(std::iter::once(head.sequence))
        .max()
        .unwrap_or(0);
    if latest > sequence {
        return Ok(Verdict::Truncated {
            sequence: latest,
            last: sequence,
        });
    }
    Ok(Verdict::Verified {
        from,
        through: sequence,
        checkpoints: u64::try_from(checkpoints.len()).unwrap_or(u64::MAX),
    })
}

/// The sequence retention cuts `stream` at for a moment `before_ms`: the latest checkpoint
/// before the first row appended at or after `before_ms`, so that every row it deletes is older
/// and the checkpoint it keeps anchors the rest. `None` when no checkpoint is that early.
pub fn retention_cut(
    store: &dyn AuditStore,
    stream: &str,
    before_ms: u64,
) -> Result<Option<u64>, Failure> {
    // The first row that is kept whatever the checkpoints: the first appended at or after the
    // moment, or, when every row is older, one past the last.
    let mut after = 0;
    let mut kept_from: Option<u64> = None;
    'rows: loop {
        let page = store.audit_rows(stream, after, PAGE)?;
        let Some(last) = page.last().map(|row| row.sequence) else {
            break;
        };
        for row in &page {
            if row.at_ms >= before_ms {
                kept_from = Some(row.sequence);
                break 'rows;
            }
        }
        after = last;
    }
    let bound = kept_from.unwrap_or_else(|| after.saturating_add(1));
    Ok(store
        .audit_checkpoints(stream)?
        .iter()
        .map(|checkpoint| checkpoint.sequence)
        .filter(|sequence| *sequence < bound)
        .max())
}

/// Delete the segments of `stream` older than `before_ms` ([`retention_cut`]) and answer how many
/// rows were deleted.
pub fn retain(store: &dyn AuditStore, stream: &str, before_ms: u64) -> Result<u64, Failure> {
    match retention_cut(store, stream, before_ms)? {
        Some(cut) => store.audit_drop_through(stream, cut),
        None => Ok(0),
    }
}
