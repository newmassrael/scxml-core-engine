// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The outboxes of ADR 0109 D10 sealed (D8): an outbox is kept under the entry of its client's
//! name, and every envelope in it is the record of the payload `[client, envelope]`, or `[client,
//! envelope, sender]` for one a session sent, sealed for the store [`STORE_OUTBOX`] and the
//! client's entry. An envelope moved to another client's outbox does not open, nor one changed by
//! a byte; its place in its own outbox (its number, its moment) is the wrapped store's, as a
//! session's version is. The sending session is kept as its entry beside the record, so the notes
//! an expiry writes are found by the entry of the session they are for, and its name inside the
//! record. The incarnation an outbox numbers in is kept as it is: a random text the client is
//! shown on every exchange.

use sce_forge_runtime::cbor::{
    read_array_len, read_bytes, read_text, write_array_head, write_bytes, write_text,
};
use sce_forge_runtime::codec::CodecError;
use sce_rust_session_store::{
    Append, Appended, Expired, Failure, Note, Outbox, OutboxEntry, SessionStore,
};
use zeroize::Zeroizing;

use super::{decoded, encoded, SealError, Sealed};
use crate::cose::put;

/// The name the outboxes of a `SessionStore` are sealed for.
pub const STORE_OUTBOX: &str = "outbox";

/// An append as the wrapped store keeps it: the entries of its client and sender, and its record.
pub(super) struct SealedAppend {
    client: String,
    sender: Option<String>,
    record: Vec<u8>,
}

impl SealedAppend {
    /// The append the wrapped store is given, in `incarnation`.
    pub(super) fn append<'a>(&'a self, incarnation: &'a str) -> Append<'a> {
        Append {
            client: &self.client,
            incarnation,
            sender: self.sender.as_deref(),
            envelope: &self.record,
        }
    }
}

/// What an entry's record holds: the client's name, the envelope, and the sending session's key.
struct Opened {
    client: String,
    envelope: Vec<u8>,
    sender: Option<String>,
}

/// The payload of `envelope` for `client`, sent by `sender`.
fn payload(client: &str, envelope: &[u8], sender: Option<&str>) -> Zeroizing<Vec<u8>> {
    let sender_bytes = sender.map_or(0, str::len);
    encoded(client.len() + envelope.len() + sender_bytes + 27, |w| {
        put(write_array_head(w, if sender.is_some() { 3 } else { 2 }));
        put(write_text(w, client));
        put(write_bytes(w, envelope));
        if let Some(sender) = sender {
            put(write_text(w, sender));
        }
    })
}

/// What the payload `plaintext` holds.
fn opened(plaintext: &[u8]) -> Result<Opened, SealError> {
    decoded(plaintext, |c| {
        let items = read_array_len(c)?;
        if items != 2 && items != 3 {
            return Err(CodecError::CborWrongLength);
        }
        let client = read_text(c, None)?.to_string();
        let envelope = read_bytes(c, None)?.to_vec();
        let sender = if items == 3 {
            Some(read_text(c, None)?.to_string())
        } else {
            None
        };
        Ok(Opened {
            client,
            envelope,
            sender,
        })
    })
}

impl<S> Sealed<S> {
    /// `appends` as the wrapped store keeps them: each client and sender as its entry, and each
    /// envelope sealed, with its client's name and its sender's key, for the client's entry.
    pub(super) fn seal_appends(
        &self,
        appends: &[Append<'_>],
    ) -> Result<Vec<SealedAppend>, Failure> {
        appends
            .iter()
            .map(|append| {
                let client = self.digest_of(append.client);
                let record = self.seal(
                    STORE_OUTBOX,
                    client.as_bytes(),
                    &payload(append.client, append.envelope, append.sender),
                )?;
                Ok(SealedAppend {
                    client,
                    sender: append.sender.map(|sender| self.digest_of(sender)),
                    record,
                })
            })
            .collect()
    }

    /// `appended`, which the wrapped store answered for the appends of `appends`, named by the
    /// clients' names.
    pub(super) fn named(appended: Vec<Appended>, appends: &[Append<'_>]) -> Vec<Appended> {
        appended
            .into_iter()
            .zip(appends)
            .map(|(kept, append)| Appended {
                client: append.client.to_string(),
                ..kept
            })
            .collect()
    }

    /// What the record of an entry of `client`'s outbox, whose entry is `entry`, holds; refused
    /// when it was sealed for another client.
    fn open_entry(&self, client: &str, entry: &str, record: &[u8]) -> Result<Opened, Failure> {
        let plaintext = self.open(STORE_OUTBOX, entry.as_bytes(), record)?;
        let held = opened(&plaintext)?;
        if held.client != client {
            return Err(SealError::Refused.into());
        }
        Ok(held)
    }

    /// The entry `stored` of `client`'s outbox, opened.
    fn opened_entry(&self, client: &str, stored: OutboxEntry) -> Result<OutboxEntry, Failure> {
        let held = self.open_entry(client, &self.digest_of(client), &stored.envelope)?;
        Ok(OutboxEntry {
            number: stored.number,
            at_ms: stored.at_ms,
            sender: held.sender,
            envelope: held.envelope,
        })
    }

    /// `outbox`, the wrapped store's outbox of `client`, opened entry by entry.
    pub(super) fn opened_outbox(&self, client: &str, outbox: Outbox) -> Result<Outbox, Failure> {
        let entries = outbox
            .entries
            .into_iter()
            .map(|stored| self.opened_entry(client, stored))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Outbox {
            incarnation: outbox.incarnation,
            next: outbox.next,
            entries,
        })
    }

    /// `expired`, an entry an expiry of `client`'s outbox deleted, opened. The deletion is
    /// durable already, so an entry whose record does not open is told as the wrapped store kept
    /// it, its sender an entry and its envelope a record, rather than not told.
    pub(super) fn opened_expired(&self, client: &str, expired: Expired) -> Expired {
        let entry = match self.opened_entry(client, expired.entry.clone()) {
            Ok(entry) => entry,
            Err(_) => expired.entry,
        };
        Expired {
            client: client.to_string(),
            entry,
        }
    }

    /// `note`, a note the wrapped store holds, opened: its client and envelope read from the
    /// record of the entry it is for.
    pub(super) fn opened_note(&self, note: Note) -> Result<Note, Failure> {
        let plaintext = self.open(STORE_OUTBOX, note.client.as_bytes(), &note.envelope)?;
        let held = opened(&plaintext)?;
        if self.digest_of(&held.client) != note.client {
            return Err(SealError::Refused.into());
        }
        Ok(Note {
            id: note.id,
            client: held.client,
            number: note.number,
            at_ms: note.at_ms,
            envelope: held.envelope,
        })
    }
}

/// Every operation of an outbox over a sealing store, called from its `SessionStore`
/// implementation.
impl<S: SessionStore> Sealed<S> {
    pub(super) fn append_sealed(
        &self,
        appends: &[Append<'_>],
        now_ms: u64,
    ) -> Result<Vec<Appended>, Failure> {
        let sealed = self.seal_appends(appends)?;
        let given: Vec<Append<'_>> = sealed
            .iter()
            .zip(appends)
            .map(|(held, append)| held.append(append.incarnation))
            .collect();
        let appended = self.inner.append(&given, now_ms)?;
        Ok(Self::named(appended, appends))
    }

    pub(super) fn outbox_sealed(&self, client: &str) -> Result<Option<Outbox>, Failure> {
        self.inner
            .outbox(&self.digest_of(client))?
            .map(|outbox| self.opened_outbox(client, outbox))
            .transpose()
    }

    pub(super) fn open_outbox_sealed(
        &self,
        client: &str,
        incarnation: &str,
        now_ms: u64,
    ) -> Result<Outbox, Failure> {
        let outbox = self
            .inner
            .open_outbox(&self.digest_of(client), incarnation, now_ms)?;
        self.opened_outbox(client, outbox)
    }
}
