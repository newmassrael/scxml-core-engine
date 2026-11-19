// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a server keeps what proves a client's name (ADR 0109, decision D3; SCE_MESH.md section
//! 22.5).
//!
//! Two kinds of credential, each kept as the SHA-256 of the secret and never as the secret:
//!
//! * **pins** ([`PinStore`]): a name to the digest of the installation secret the `install`
//!   authenticator pinned it to at its first answered exchange (trust on first use);
//! * **tokens** ([`TokenStore`]): the digest of a bearer token, with the name it proves, the moment
//!   it was issued, the moment it expires and whether it is revoked. A presented token is digested
//!   and looked up by its digest, so the store is asked for nothing it could compare.
//!
//! The store keeps rows and compares no secret: whether a presented secret is the one a name is
//! pinned to, or whether a token's name is the name claimed, is decided by its caller with the one
//! comparison of a secret of SCE's Rust code (`sce_rust_crypto::equal::same_secret`), in time that
//! does not depend on where the two differ. A digest is not a secret an attacker can use, so a
//! lookup by it in an index is no comparison of one.
//!
//! [`CredentialStore`] is both, with the revocation of a name in one transaction: its tokens are
//! revoked and its pin removed together.
//!
//! * [`MemoryCredentials`] keeps them in the process: a server that keeps its credentials there
//!   forgets every pin and token when it stops.
//! * `SqliteStore` (feature `sqlite`) keeps them in the session store's file, beside the sessions
//!   and the keys (layout 3): a restart keeps them.
//! * [`crate::credential_conformance`] is what every implementation is held to.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::{moment, Failure};

/// The bytes of a credential's digest: the whole output of SHA-256.
pub const DIGEST_BYTES: usize = 32;

/// The SHA-256 of a secret, as a store keeps it.
pub type Digest = [u8; DIGEST_BYTES];

/// Where a server keeps the installation pins: a name to the bytes it is pinned to, which the
/// store keeps and never reads. The `install` authenticator pins the SHA-256 of the installation
/// secret; a store that wraps another and seals what it keeps (ADR 0109, D8) pins its sealing of
/// that digest in the one it wraps.
///
/// Every method is one transaction. A handle may be shared between threads; whether handles on one
/// store may live in several processes is the implementation's to say.
pub trait PinStore: Send + Sync {
    /// The bytes `name` is pinned to, or `None`.
    fn pinned(&self, name: &str) -> Result<Option<Vec<u8>>, Failure>;

    /// Pin `name` to `pin` unless it is pinned already, in one step, and answer the bytes it is
    /// pinned to afterwards: `pin` when this call pinned it, the ones held before when it did
    /// not. A pin is never replaced by this call; of two first exchanges that bring different
    /// secrets, the first kept is the pin. Whether the answer is `pin` is the caller's to compare,
    /// in constant time.
    fn pin(&self, name: &str, pin: &[u8]) -> Result<Vec<u8>, Failure>;

    /// Remove the pin of `name`, so that the next secret it brings is pinned: `true` when it was
    /// pinned. This is how a client that lost its secret claims its name again, by the operator's
    /// hand or a statechart's revocation.
    fn unpin(&self, name: &str) -> Result<bool, Failure>;

    /// Every pin held, in the order of the UTF-8 bytes of the names.
    fn pins(&self) -> Result<Vec<PinRow>, Failure>;

    /// Replace the bytes `name` is pinned to with `pin` when they are `held`, and answer whether
    /// it did: `false`, writing nothing, when the name is pinned to other bytes or to none. This
    /// is how a store that seals what it keeps seals a pin again under its active key (ADR 0109,
    /// D8), the digest it holds unchanged: a pin read and sealed again is never written over one
    /// that changed in between.
    fn repin(&self, name: &str, held: &[u8], pin: &[u8]) -> Result<bool, Failure>;
}

/// An installation pin as a store keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinRow {
    pub name: String,
    /// The bytes the name is pinned to.
    pub pin: Vec<u8>,
}

/// A bearer token as a store keeps it: the digest of its text, never the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRow {
    /// The SHA-256 of the token's text, by which a presented token is found.
    pub digest: Digest,
    /// The name the token proves.
    pub name: String,
    /// When it was issued, in milliseconds since the Unix epoch.
    pub issued_ms: u64,
    /// When it stops proving its name, in milliseconds since the Unix epoch: it proves it before
    /// this moment and not at it.
    pub expires_ms: u64,
    /// Whether it was revoked. A revoked token proves nothing, and stays revoked.
    pub revoked: bool,
    /// Bytes kept with the row that the store never reads: empty in a row its issuer writes. A
    /// store that wraps another and seals what it keeps (ADR 0109, D8) keeps the row's sealed
    /// record here in the one it wraps, and in that one's other columns only what that one
    /// compares or sets.
    pub sealed: Vec<u8>,
}

impl TokenRow {
    /// Whether the token proves its name at `now_ms`: not revoked, and not expired.
    pub fn live_at(&self, now_ms: u64) -> bool {
        !self.revoked && now_ms < self.expires_ms
    }
}

/// Where a server keeps the bearer tokens its host issued.
///
/// Every method is one transaction. A handle may be shared between threads; whether handles on one
/// store may live in several processes is the implementation's to say.
pub trait TokenStore: Send + Sync {
    /// The token whose text digests to `digest`, or `None`.
    fn token(&self, digest: &Digest) -> Result<Option<TokenRow>, Failure>;

    /// Keep `row` unless the store holds a token of its digest, and answer the row it holds
    /// afterwards. A token is never replaced: one issued again under the identity that issued it
    /// first (docs/SCE_HOST_FEATURES.md section 1) is the row kept the first time, its expiry and
    /// its revocation included, so a revoked token is not made live by a repeat.
    fn issue(&self, row: &TokenRow) -> Result<TokenRow, Failure>;

    /// Every token held, revoked and expired ones included, in the order of their digests.
    fn tokens(&self) -> Result<Vec<TokenRow>, Failure>;

    /// Replace the [`TokenRow::sealed`] bytes of the token whose digest is `digest` with `sealed`
    /// when they are `held`, every other column kept, and answer whether it did: `false`, writing
    /// nothing, when the store holds no such token or its bytes are others. This is how a store
    /// that seals what it keeps seals a token again under its active key (ADR 0109, D8).
    fn reseal(&self, digest: &Digest, held: &[u8], sealed: &[u8]) -> Result<bool, Failure>;
}

/// What the revocation of a name took away.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Revoked {
    /// How many of its tokens were live in the store's eyes (not revoked) and are revoked now.
    pub tokens: u32,
    /// Whether its pin was removed.
    pub pin: bool,
}

impl Revoked {
    /// The tokens and the pin removed, as one count.
    pub fn count(&self) -> u32 {
        self.tokens.saturating_add(u32::from(self.pin))
    }
}

/// A store of pins and tokens, with the revocation of a name that takes both.
pub trait CredentialStore: PinStore + TokenStore {
    /// Revoke every token of `name` not revoked yet and remove its pin, in one transaction. The
    /// tokens are kept as revoked rather than removed, so that a repeat of the issue that made one
    /// answers the revoked row and makes no live token.
    fn revoke(&self, name: &str) -> Result<Revoked, Failure>;
}

/// Pins and tokens in memory: what a server keeps when it is given no durable store. A clone is
/// another handle on the same credentials, as a second connection is on a database. One lock is a
/// transaction.
#[derive(Clone, Default)]
pub struct MemoryCredentials {
    shared: Arc<Mutex<CredentialRecords>>,
}

#[derive(Default)]
struct CredentialRecords {
    pins: BTreeMap<String, Vec<u8>>,
    tokens: BTreeMap<Digest, TokenRow>,
}

impl MemoryCredentials {
    pub fn new() -> Self {
        Self::default()
    }

    /// The records, whatever a thread that panicked while it held them left: nothing, since every
    /// call changes them only in steps that do not panic.
    fn records(&self) -> MutexGuard<'_, CredentialRecords> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PinStore for MemoryCredentials {
    fn pinned(&self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
        Ok(self.records().pins.get(name).cloned())
    }

    fn pin(&self, name: &str, pin: &[u8]) -> Result<Vec<u8>, Failure> {
        Ok(self
            .records()
            .pins
            .entry(name.to_string())
            .or_insert_with(|| pin.to_vec())
            .clone())
    }

    fn unpin(&self, name: &str) -> Result<bool, Failure> {
        Ok(self.records().pins.remove(name).is_some())
    }

    fn pins(&self) -> Result<Vec<PinRow>, Failure> {
        Ok(self
            .records()
            .pins
            .iter()
            .map(|(name, pin)| PinRow {
                name: name.clone(),
                pin: pin.clone(),
            })
            .collect())
    }

    fn repin(&self, name: &str, held: &[u8], pin: &[u8]) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(match records.pins.get_mut(name) {
            Some(kept) if kept.as_slice() == held => {
                *kept = pin.to_vec();
                true
            }
            _ => false,
        })
    }
}

impl TokenStore for MemoryCredentials {
    fn token(&self, digest: &Digest) -> Result<Option<TokenRow>, Failure> {
        Ok(self.records().tokens.get(digest).cloned())
    }

    fn issue(&self, row: &TokenRow) -> Result<TokenRow, Failure> {
        Ok(self
            .records()
            .tokens
            .entry(row.digest)
            .or_insert_with(|| TokenRow {
                issued_ms: moment(row.issued_ms),
                expires_ms: moment(row.expires_ms),
                ..row.clone()
            })
            .clone())
    }

    fn tokens(&self) -> Result<Vec<TokenRow>, Failure> {
        Ok(self.records().tokens.values().cloned().collect())
    }

    fn reseal(&self, digest: &Digest, held: &[u8], sealed: &[u8]) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(match records.tokens.get_mut(digest) {
            Some(row) if row.sealed == held => {
                row.sealed = sealed.to_vec();
                true
            }
            _ => false,
        })
    }
}

impl CredentialStore for MemoryCredentials {
    fn revoke(&self, name: &str) -> Result<Revoked, Failure> {
        let mut records = self.records();
        let mut tokens = 0u32;
        for row in records.tokens.values_mut() {
            if row.name == name && !row.revoked {
                row.revoked = true;
                tokens = tokens.saturating_add(1);
            }
        }
        let pin = records.pins.remove(name).is_some();
        Ok(Revoked { tokens, pin })
    }
}
