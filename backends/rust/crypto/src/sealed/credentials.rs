// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The credentials of ADR 0109, D3, sealed (D8).
//!
//! - A pin is kept under the entry of its name, and its value is the record of the payload
//!   `[name, pin]`.
//! - A token is kept under the keyed digest of its digest, and the name the wrapped store indexes
//!   it by, which a revocation matches, is the entry of its name. The record of the payload
//!   `[digest, name, issued_ms, expires_ms, sealed]` is the wrapped row's sealed bytes; its moments
//!   there are 0, and its revocation is the wrapped store's, which a revocation sets.

use std::collections::BTreeMap;

use sce_forge_runtime::cbor::{
    read_bytes, read_text, read_uint, write_array_head, write_bytes, write_text, write_uint,
};
use sce_forge_runtime::codec::CodecError;
use sce_rust_session_store::{
    moment, CredentialStore, Digest, Failure, PinRow, PinStore, Revoked, TokenRow, TokenStore,
};
use zeroize::Zeroizing;

use super::{
    decoded, encoded, expect_items, Reseal, Resealed, SealError, Sealed, STORE_PINS, STORE_TOKENS,
};
use crate::cose::put;
use crate::equal::same_secret;

/// The payload of the pin of `name` to `pin`.
fn pin_payload(name: &str, pin: &[u8]) -> Zeroizing<Vec<u8>> {
    encoded(name.len() + pin.len() + 18, |w| {
        put(write_array_head(w, 2));
        put(write_text(w, name));
        put(write_bytes(w, pin));
    })
}

/// The name and the pin a pin's payload holds.
fn pin_of(plaintext: &[u8]) -> Result<PinRow, SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 2)?;
        let name = read_text(c, None)?.to_string();
        let pin = read_bytes(c, None)?.to_vec();
        Ok(PinRow { name, pin })
    })
}

/// The payload of `row`, its moments as a store keeps them.
fn token_payload(row: &TokenRow) -> Zeroizing<Vec<u8>> {
    encoded(row.name.len() + row.sealed.len() + 80, |w| {
        put(write_array_head(w, 5));
        put(write_bytes(w, &row.digest));
        put(write_text(w, &row.name));
        put(write_uint(w, moment(row.issued_ms)));
        put(write_uint(w, moment(row.expires_ms)));
        put(write_bytes(w, &row.sealed));
    })
}

/// The token a token's payload holds, revoked as `revoked`.
fn token_of(plaintext: &[u8], revoked: bool) -> Result<TokenRow, SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 5)?;
        let digest = read_bytes(c, None)?;
        let digest = Digest::try_from(digest).map_err(|_| CodecError::CborWrongLength)?;
        let name = read_text(c, None)?.to_string();
        let issued_ms = read_uint(c)?;
        let expires_ms = read_uint(c)?;
        let sealed = read_bytes(c, None)?.to_vec();
        Ok(TokenRow {
            digest,
            name,
            issued_ms,
            expires_ms,
            revoked,
            sealed,
        })
    })
}

impl<S> Sealed<S> {
    /// The keyed digest a token of `digest` is kept under.
    fn token_entry(&self, digest: &Digest) -> Digest {
        self.keyed(digest)
    }

    /// The pin the wrapped store's row `row` holds, opened.
    fn open_pin(&self, row: &PinRow) -> Result<PinRow, Failure> {
        let plaintext = self.open(STORE_PINS, row.name.as_bytes(), &row.pin)?;
        pin_of(&plaintext).map_err(Failure::from)
    }

    /// The token the wrapped store's row `row` holds, opened.
    fn open_token(&self, row: &TokenRow) -> Result<TokenRow, Failure> {
        let plaintext = self.open(STORE_TOKENS, &row.digest, &row.sealed)?;
        token_of(&plaintext, row.revoked).map_err(Failure::from)
    }
}

impl<S: PinStore> Sealed<S> {
    /// The pin of `name` the wrapped store holds under `entry`, opened and held to its name.
    fn pin_held(&self, name: &str, entry: &str, record: Vec<u8>) -> Result<Vec<u8>, Failure> {
        let held = self.open_pin(&PinRow {
            name: entry.to_string(),
            pin: record,
        })?;
        if held.name != name {
            return Err(SealError::Refused.into());
        }
        Ok(held.pin)
    }
}

impl<S: CredentialStore> Sealed<S> {
    /// Write every pin and token sealed under a key other than the active one again under the
    /// active one, each by the wrapped store's replacement from the bytes the pass read: one
    /// another process changed in between is left for a later pass. One that does not open is
    /// counted and left.
    pub fn reseal_credentials(&self) -> Result<Resealed, Failure> {
        let mut resealed = Resealed::default();
        for row in self.inner.pins()? {
            match self.reseal_record(STORE_PINS, row.name.as_bytes(), &row.pin)? {
                Reseal::Current => {}
                Reseal::Refused => resealed.refused += 1,
                Reseal::Record(record) => {
                    resealed.count(self.inner.repin(&row.name, &row.pin, &record)?);
                }
            }
        }
        for row in self.inner.tokens()? {
            match self.reseal_record(STORE_TOKENS, &row.digest, &row.sealed)? {
                Reseal::Current => {}
                Reseal::Refused => resealed.refused += 1,
                Reseal::Record(record) => {
                    resealed.count(self.inner.reseal(&row.digest, &row.sealed, &record)?);
                }
            }
        }
        Ok(resealed)
    }
}

impl<S: PinStore> PinStore for Sealed<S> {
    fn pinned(&self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
        let entry = self.digest_of(name);
        let Some(record) = self.inner.pinned(&entry)? else {
            return Ok(None);
        };
        self.pin_held(name, &entry, record).map(Some)
    }

    fn pin(&self, name: &str, pin: &[u8]) -> Result<Vec<u8>, Failure> {
        let entry = self.digest_of(name);
        let record = self.seal(STORE_PINS, entry.as_bytes(), &pin_payload(name, pin))?;
        let held = self.inner.pin(&entry, &record)?;
        self.pin_held(name, &entry, held)
    }

    fn unpin(&self, name: &str) -> Result<bool, Failure> {
        self.inner.unpin(&self.digest_of(name))
    }

    fn pins(&self) -> Result<Vec<PinRow>, Failure> {
        // The wrapped store lists them in the order of their entries; a map puts them in the order
        // of their names.
        let mut pins = BTreeMap::new();
        for row in self.inner.pins()? {
            let pin = self.open_pin(&row)?;
            pins.insert(pin.name, pin.pin);
        }
        Ok(pins
            .into_iter()
            .map(|(name, pin)| PinRow { name, pin })
            .collect())
    }

    fn repin(&self, name: &str, held: &[u8], pin: &[u8]) -> Result<bool, Failure> {
        let entry = self.digest_of(name);
        let Some(record) = self.inner.pinned(&entry)? else {
            return Ok(false);
        };
        if !same_secret(&self.pin_held(name, &entry, record.clone())?, held) {
            return Ok(false);
        }
        let replacement = self.seal(STORE_PINS, entry.as_bytes(), &pin_payload(name, pin))?;
        self.inner.repin(&entry, &record, &replacement)
    }
}

impl<S: TokenStore> TokenStore for Sealed<S> {
    fn token(&self, digest: &Digest) -> Result<Option<TokenRow>, Failure> {
        let Some(row) = self.inner.token(&self.token_entry(digest))? else {
            return Ok(None);
        };
        let token = self.open_token(&row)?;
        if token.digest != *digest {
            return Err(SealError::Refused.into());
        }
        Ok(Some(token))
    }

    fn issue(&self, row: &TokenRow) -> Result<TokenRow, Failure> {
        let entry = self.token_entry(&row.digest);
        let sealed = self.seal(STORE_TOKENS, &entry, &token_payload(row))?;
        let kept = self.inner.issue(&TokenRow {
            digest: entry,
            name: self.digest_of(&row.name),
            issued_ms: 0,
            expires_ms: 0,
            revoked: row.revoked,
            sealed,
        })?;
        self.open_token(&kept)
    }

    fn tokens(&self) -> Result<Vec<TokenRow>, Failure> {
        let mut tokens = self
            .inner
            .tokens()?
            .iter()
            .map(|row| self.open_token(row))
            .collect::<Result<Vec<TokenRow>, Failure>>()?;
        tokens.sort_by_key(|row| row.digest);
        Ok(tokens)
    }

    fn reseal(&self, digest: &Digest, held: &[u8], sealed: &[u8]) -> Result<bool, Failure> {
        let entry = self.token_entry(digest);
        let Some(row) = self.inner.token(&entry)? else {
            return Ok(false);
        };
        let token = self.open_token(&row)?;
        if token.digest != *digest || token.sealed != held {
            return Ok(false);
        }
        let replacement = self.seal(
            STORE_TOKENS,
            &entry,
            &token_payload(&TokenRow {
                sealed: sealed.to_vec(),
                ..token
            }),
        )?;
        self.inner.reseal(&entry, &row.sealed, &replacement)
    }
}

impl<S: CredentialStore> CredentialStore for Sealed<S> {
    fn revoke(&self, name: &str) -> Result<Revoked, Failure> {
        self.inner.revoke(&self.digest_of(name))
    }
}
