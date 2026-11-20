// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The audit log of ADR 0109, D14, sealed (D8).
//!
//! - A stream is kept under the entry of its name.
//! - A row's entry is the record of the payload `[stream, entry]`, sealed for the row of the
//!   stream's entry at its sequence (the entry, a colon and the sequence's decimal digits). Its
//!   sequence, its moment and its chain are the wrapped store's, which orders and keeps them.
//! - A checkpoint's signature, whose payload names the stream, is the record of the payload
//!   `[stream, signature]`, sealed for the checkpoint of the same row; its sequence and its chain
//!   are the wrapped store's.
//! - The identity of an append is kept as a first result of the key-value feature is (module
//!   `kv`): its session and its invoke each as its entry.
//!
//! What it does not give: the chain is the SHA-256 of what the rows hold in the clear, kept in the
//! clear so that a reader verifies it without the sealing keys; an entry of few possible values
//! can be found again from it by trying them. A log whose entries are that guessable keeps a
//! value that is not, a request's id, in each.

use sce_forge_runtime::cbor::{read_bytes, read_text, write_array_head, write_bytes, write_text};
use sce_rust_session_store::{
    AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore, Failure, Identity,
};
use zeroize::Zeroizing;

use super::{decoded, encoded, expect_items, SealError, Sealed};
use crate::cose::put;

/// The name the entries of an `AuditStore`'s rows are sealed for.
pub const STORE_AUDIT_ROWS: &str = "audit_rows";
/// The name the signatures of an `AuditStore`'s checkpoints are sealed for.
pub const STORE_AUDIT_CHECKPOINTS: &str = "audit_checkpoints";

/// The payload of `bytes` of the stream `stream`.
fn payload(stream: &str, bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    encoded(stream.len() + bytes.len() + 18, |w| {
        put(write_array_head(w, 2));
        put(write_text(w, stream));
        put(write_bytes(w, bytes));
    })
}

/// The stream and the bytes a payload holds.
fn opened(plaintext: &[u8]) -> Result<(String, Vec<u8>), SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 2)?;
        let stream = read_text(c, None)?.to_string();
        let bytes = read_bytes(c, None)?.to_vec();
        Ok((stream, bytes))
    })
}

/// What a row and a checkpoint of the stream entered as `entry` at `sequence` are sealed for.
fn indexed(entry: &str, sequence: u64) -> Vec<u8> {
    format!("{entry}:{sequence}").into_bytes()
}

impl<S> Sealed<S> {
    /// The bytes the record `record`, sealed for `store` at row `sequence` of the stream entered as
    /// `entry`, holds, held to the stream `stream`.
    fn open_audit(
        &self,
        store: &str,
        stream: &str,
        entry: &str,
        sequence: u64,
        record: &[u8],
    ) -> Result<Vec<u8>, Failure> {
        let plaintext = self.open(store, &indexed(entry, sequence), record)?;
        let (held, bytes) = opened(&plaintext)?;
        if held != stream {
            return Err(SealError::Refused.into());
        }
        Ok(bytes)
    }
}

impl<S: AuditStore> AuditStore for Sealed<S> {
    fn audit_head(&self, stream: &str) -> Result<AuditHead, Failure> {
        self.inner.audit_head(&self.digest_of(stream))
    }

    fn audit_append(
        &self,
        row: &AuditRow,
        checkpoint: Option<&[u8]>,
        identity: Option<&Identity<'_>>,
    ) -> Result<AuditAppend, Failure> {
        let entry = self.digest_of(&row.stream);
        let at = indexed(&entry, row.sequence);
        let sealed = AuditRow {
            stream: entry,
            entry: self.seal(STORE_AUDIT_ROWS, &at, &payload(&row.stream, &row.entry))?,
            ..row.clone()
        };
        let signature = match checkpoint {
            Some(signature) => Some(self.seal(
                STORE_AUDIT_CHECKPOINTS,
                &at,
                &payload(&row.stream, signature),
            )?),
            None => None,
        };
        let kept = identity.map(|identity| self.indexed_identity(identity));
        let identity = kept.as_ref().map(|kept| kept.identity());
        self.inner
            .audit_append(&sealed, signature.as_deref(), identity.as_ref())
    }

    fn audit_checkpoint(&self, checkpoint: &AuditCheckpoint) -> Result<bool, Failure> {
        let entry = self.digest_of(&checkpoint.stream);
        let signature = self.seal(
            STORE_AUDIT_CHECKPOINTS,
            &indexed(&entry, checkpoint.sequence),
            &payload(&checkpoint.stream, &checkpoint.signature),
        )?;
        self.inner.audit_checkpoint(&AuditCheckpoint {
            stream: entry,
            sequence: checkpoint.sequence,
            chain: checkpoint.chain,
            signature,
        })
    }

    fn audit_rows(&self, stream: &str, after: u64, limit: usize) -> Result<Vec<AuditRow>, Failure> {
        let entry = self.digest_of(stream);
        self.inner
            .audit_rows(&entry, after, limit)?
            .into_iter()
            .map(|row| {
                Ok(AuditRow {
                    stream: stream.to_string(),
                    entry: self.open_audit(
                        STORE_AUDIT_ROWS,
                        stream,
                        &entry,
                        row.sequence,
                        &row.entry,
                    )?,
                    ..row
                })
            })
            .collect()
    }

    fn audit_checkpoints(&self, stream: &str) -> Result<Vec<AuditCheckpoint>, Failure> {
        let entry = self.digest_of(stream);
        self.inner
            .audit_checkpoints(&entry)?
            .into_iter()
            .map(|checkpoint| {
                Ok(AuditCheckpoint {
                    stream: stream.to_string(),
                    signature: self.open_audit(
                        STORE_AUDIT_CHECKPOINTS,
                        stream,
                        &entry,
                        checkpoint.sequence,
                        &checkpoint.signature,
                    )?,
                    ..checkpoint
                })
            })
            .collect()
    }

    fn audit_drop_through(&self, stream: &str, through: u64) -> Result<u64, Failure> {
        self.inner
            .audit_drop_through(&self.digest_of(stream), through)
    }

    fn audit_first(&self, identity: &Identity<'_>) -> Result<Option<u64>, Failure> {
        self.inner
            .audit_first(&self.indexed_identity(identity).identity())
    }
}
