// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Signing and sealing (docs/SCE_HOST_FEATURES.md, section 10): COSE objects
//! (RFC 9052) made and read with keys the host holds and a statechart names by
//! id.
//!
//! The objects themselves are `sce_rust_crypto::cose`'s, the one implementation
//! every Rust host of SCE signs and seals with: EdDSA on Ed25519 for a
//! `COSE_Sign1`, ChaCha20/Poly1305 under a 96-bit nonce drawn from the host's
//! random source for a `COSE_Encrypt0`, and HKDF with SHA-256 for a derived
//! key. This module is the operations: what a request may name, which key it
//! names, and what a statechart is answered.
//!
//! A key is one the embedding configured ([`CoseKeys`]) or one of the host's
//! key ring (docs/SCE_HOST_FEATURES.md section 13): a key the host generated or
//! derived, whose id begins `generated:` or `derived:` and which the key ring
//! keeps under its use and its state.

use std::collections::BTreeMap;
use std::fmt;

use sce_rust_crypto::keyring::{is_key_ring_id, DerivationInput, KeyError, Purpose};
use sce_rust_crypto::random::{RandomSource, SystemRandom};
use serde_json::json;
use zeroize::{Zeroize, Zeroizing};

pub use sce_rust_crypto::cose::{
    cose_key_ed25519, decrypt0, derive_key, encrypt0, public_key_of, sign1, verify1, TrustedKey,
    IV_BYTES, KEY_BYTES,
};
pub use sce_rust_crypto::keyring::{DERIVED_PREFIX, GENERATED_PREFIX};

use crate::{
    failure, record, NamedBounds, Outcome, Params, ReferenceHost, Transport, INVALID_ARGUMENT,
};

/// The operation that signs a payload.
pub const SIGN1: &str = "sce-host-cose-sign1";
/// The operation that verifies a signed object.
pub const VERIFY1: &str = "sce-host-cose-verify1";
/// The operation that seals a plaintext.
pub const ENCRYPT0: &str = "sce-host-cose-encrypt0";
/// The operation that opens a sealed object.
pub const DECRYPT0: &str = "sce-host-cose-decrypt0";
/// The operation that derives a symmetric key.
pub const KEY_DERIVE: &str = "sce-host-key-derive";

/// The five operations, in the order the specification lists them.
pub const OPERATIONS: [&str; 5] = [SIGN1, VERIFY1, ENCRYPT0, DECRYPT0, KEY_DERIVE];

/// The failure of an operation whose key, trust set or input key the host does
/// not hold.
pub(crate) const UNKNOWN_KEY: &str = "unknown_key";

/// The failure of an operation whose random source gave no bytes.
pub(crate) const UNAVAILABLE: &str = "unavailable";

// The bounds of the records' fields (`sce:std/host/cose_*.scxml`,
// `key_derive_*.scxml`) that no build moves, in bytes. The payload, the
// plaintext and the objects made of them are named bounds, which the host
// holds as its build gave them (`NamedBounds`; docs/adr/0109, D17).
pub(crate) const KEY_ID_MAX: usize = 64;
const AAD_MAX: usize = 256;
const SECRET_MAX: usize = 256;
const SALT_MAX: usize = 64;
const INFO_MAX: usize = 256;

/// The keys an embedding gives the host ([`ReferenceHost::set_cose_keys`]).
/// The statechart names each by its id and never holds one. The host copies
/// them into memory it wipes, and wipes this value's keys when it takes them.
#[derive(Clone, Default)]
pub struct CoseKeys {
    /// The keys `sce-host-cose-sign1` signs with, by id: each the 32-byte seed
    /// of an Ed25519 key (RFC 8032 section 5.1.5).
    pub signing: BTreeMap<String, [u8; KEY_BYTES]>,
    /// The keys `sce-host-cose-encrypt0` and `sce-host-cose-decrypt0` seal and
    /// open with, and `sce-host-key-derive` derives from, by id: each a 32-byte
    /// ChaCha20/Poly1305 key.
    pub symmetric: BTreeMap<String, [u8; KEY_BYTES]>,
    /// The keys `sce-host-mac` tags and `sce-host-mac-verify` verifies with, by
    /// id: each an HMAC-SHA-256 key of any length.
    pub mac: BTreeMap<String, Vec<u8>>,
    /// The trust sets `sce-host-cose-verify1` verifies under, by name: the
    /// signers a statechart accepts, in the order they are tried.
    pub trust: BTreeMap<String, Vec<TrustedKey>>,
}

/// Where the nonce of each seal comes from, when the embedding gives a source of
/// its own ([`ReferenceHost::set_nonce_source`]); a host given none draws each
/// nonce from its random source (docs/SCE_HOST_FEATURES.md section 11), as the
/// C++ reference host draws it from its primitives' `random`.
///
/// A nonce used twice under one key gives both plaintexts away, so a source
/// other than the system's is for a test that holds a seal to its bytes.
pub trait NonceSource {
    /// The nonce of the next seal, or `None` when the source cannot give one,
    /// which fails the seal `unavailable`.
    fn next_nonce(&mut self) -> Option<[u8; IV_BYTES]>;
}

/// The operating system's random source, as a source of nonces.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemNonces;

impl NonceSource for SystemNonces {
    fn next_nonce(&mut self) -> Option<[u8; IV_BYTES]> {
        let mut iv = [0u8; IV_BYTES];
        SystemRandom.fill(&mut iv).then_some(iv)
    }
}

/// A key the embedding gave under an id of the key ring's own namespace
/// (`generated:` or `derived:`), which would name two keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedKeyId(pub String);

impl fmt::Display for ReservedKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the key id `{}` begins with `{GENERATED_PREFIX}` or `{DERIVED_PREFIX}`, which name \
             the keys the host generates and derives",
            self.0
        )
    }
}

impl std::error::Error for ReservedKeyId {}

/// The keys an embedding configured, held in memory that is wiped when they are
/// replaced or dropped. The keys a host generates and derives are its key
/// ring's.
#[derive(Default)]
pub(crate) struct Keys {
    signing: BTreeMap<String, Zeroizing<[u8; KEY_BYTES]>>,
    symmetric: BTreeMap<String, Zeroizing<[u8; KEY_BYTES]>>,
    mac: BTreeMap<String, Zeroizing<Vec<u8>>>,
    trust: BTreeMap<String, Vec<TrustedKey>>,
}

impl Keys {
    /// Hold `keys` in place of the configured ones, and wipe them where they
    /// were given.
    pub(crate) fn configure(&mut self, mut keys: CoseKeys) -> Result<(), ReservedKeyId> {
        let reserved = keys
            .signing
            .keys()
            .chain(keys.symmetric.keys())
            .chain(keys.mac.keys())
            .find(|id| is_key_ring_id(id))
            .cloned();
        if let Some(id) = reserved {
            wipe(&mut keys);
            return Err(ReservedKeyId(id));
        }
        self.signing = keys
            .signing
            .iter()
            .map(|(id, key)| (id.clone(), Zeroizing::new(*key)))
            .collect();
        self.symmetric = keys
            .symmetric
            .iter()
            .map(|(id, key)| (id.clone(), Zeroizing::new(*key)))
            .collect();
        self.mac = keys
            .mac
            .iter()
            .map(|(id, key)| (id.clone(), Zeroizing::new(key.clone())))
            .collect();
        self.trust = std::mem::take(&mut keys.trust);
        wipe(&mut keys);
        Ok(())
    }
}

/// Wipe the keys of a configuration that was given to the host.
fn wipe(keys: &mut CoseKeys) {
    keys.signing.values_mut().for_each(Zeroize::zeroize);
    keys.symmetric.values_mut().for_each(Zeroize::zeroize);
    keys.mac.values_mut().for_each(Zeroize::zeroize);
}

/// The seed of the signing key `id` names, for `purpose` (a signature, or the
/// export of its public key).
pub(crate) fn signing_seed<T: Transport>(
    host: &ReferenceHost<T>,
    id: &str,
    purpose: Purpose,
) -> Result<Zeroizing<[u8; KEY_BYTES]>, KeyError> {
    if is_key_ring_id(id) {
        return host.keyring.key(id, purpose);
    }
    host.cose
        .signing
        .get(id)
        .map(|seed| Zeroizing::new(**seed))
        .ok_or(KeyError::Unknown)
}

/// The symmetric key `id` names, for `purpose` (a seal, an opening or the input
/// of a derivation).
pub(crate) fn symmetric_key<T: Transport>(
    host: &ReferenceHost<T>,
    id: &str,
    purpose: Purpose,
) -> Result<Zeroizing<[u8; KEY_BYTES]>, KeyError> {
    if is_key_ring_id(id) {
        return host.keyring.key(id, purpose);
    }
    host.cose
        .symmetric
        .get(id)
        .map(|key| Zeroizing::new(**key))
        .ok_or(KeyError::Unknown)
}

/// The MAC key `id` names, for `purpose` (a tag or its verification).
pub(crate) fn mac_key<T: Transport>(
    host: &ReferenceHost<T>,
    id: &str,
    purpose: Purpose,
) -> Result<Zeroizing<Vec<u8>>, KeyError> {
    if is_key_ring_id(id) {
        return host.keyring.material(id, purpose);
    }
    host.cose
        .mac
        .get(id)
        .map(|key| Zeroizing::new(key.to_vec()))
        .ok_or(KeyError::Unknown)
}

// ── The operations ─────────────────────────────────────────────────────

/// Where a derivation's input keying material comes from.
enum Input<'p> {
    /// Bytes the statechart holds.
    Secret(Vec<u8>),
    /// A symmetric key of the host, by its id.
    Key(&'p str),
}

/// A request the host accepts, its byte strings read from their text.
enum Request<'p> {
    Sign1 {
        key_id: &'p str,
        payload: Vec<u8>,
        aad: Vec<u8>,
    },
    Verify1 {
        trust_set: &'p str,
        cose: Vec<u8>,
        aad: Vec<u8>,
    },
    Encrypt0 {
        key_id: &'p str,
        plaintext: Vec<u8>,
        aad: Vec<u8>,
    },
    Decrypt0 {
        key_id: &'p str,
        cose: Vec<u8>,
        aad: Vec<u8>,
    },
    Derive {
        input: Input<'p>,
        salt: Vec<u8>,
        info: Vec<u8>,
    },
}

/// The parameters of a start, read by name.
pub(crate) struct Arguments<'p>(pub(crate) &'p Params);

impl<'p> Arguments<'p> {
    pub(crate) fn text(&self, name: &str) -> &'p str {
        self.0.get(name).map(String::as_str).unwrap_or("")
    }

    /// A key id or a trust set's name: one to 64 bytes.
    pub(crate) fn id(&self, name: &str) -> Option<&'p str> {
        let id = self.text(name);
        (!id.is_empty() && id.len() <= KEY_ID_MAX).then_some(id)
    }

    /// A byte string of at most `max` bytes, from the text it travels as.
    pub(crate) fn bytes(&self, name: &str, max: usize) -> Option<Vec<u8>> {
        bytes_of(self.text(name)).filter(|bytes| bytes.len() <= max)
    }
}

/// The request `params` makes of `operation`, or `None` when the host refuses
/// it: an empty key id or trust set, a derivation given both or neither of a
/// secret and an input key, or a value its record's field does not hold at
/// the bounds `bounds` gives the named ones.
fn request<'p>(operation: &str, params: &'p Params, bounds: &NamedBounds) -> Option<Request<'p>> {
    let arguments = Arguments(params);
    Some(match operation {
        SIGN1 => Request::Sign1 {
            key_id: arguments.id("key_id")?,
            payload: arguments.bytes("payload", bounds.cose_payload)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        VERIFY1 => Request::Verify1 {
            trust_set: arguments.id("trust_set")?,
            cose: arguments.bytes("cose", bounds.cose_signed)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        ENCRYPT0 => Request::Encrypt0 {
            key_id: arguments.id("key_id")?,
            plaintext: arguments.bytes("plaintext", bounds.cose_plaintext)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        DECRYPT0 => Request::Decrypt0 {
            key_id: arguments.id("key_id")?,
            cose: arguments.bytes("cose", bounds.cose_sealed)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        KEY_DERIVE => {
            let secret = arguments.bytes("secret", SECRET_MAX)?;
            let input_key_id = arguments.text("input_key_id");
            let input = match (secret.is_empty(), input_key_id.is_empty()) {
                (false, true) => Input::Secret(secret),
                (true, false) => Input::Key(arguments.id("input_key_id")?),
                _ => return None,
            };
            Request::Derive {
                input,
                salt: arguments.bytes("salt", SALT_MAX)?,
                info: arguments.bytes("info", INFO_MAX)?,
            }
        }
        _ => return None,
    })
}

/// Whether the host refuses `operation` with `params` at `bounds`.
pub(crate) fn refuses(operation: &str, params: &Params, bounds: &NamedBounds) -> bool {
    request(operation, params, bounds).is_none()
}

/// The nonce of the next seal: from the source the embedding gave, or else from
/// the host's random source.
fn next_nonce<T: Transport>(host: &mut ReferenceHost<T>) -> Option<[u8; IV_BYTES]> {
    if let Some(nonces) = host.nonces.as_mut() {
        return nonces.next_nonce();
    }
    let mut iv = [0u8; IV_BYTES];
    host.random.fill(&mut iv).then_some(iv)
}

/// Perform `operation`, a request [`refuses`] accepted.
pub(crate) fn perform<T: Transport>(
    host: &mut ReferenceHost<T>,
    operation: &str,
    params: &Params,
) -> Outcome {
    let bounds = host.bounds;
    let Some(request) = request(operation, params, &bounds) else {
        return Outcome::Refuse(INVALID_ARGUMENT);
    };
    match request {
        Request::Sign1 {
            key_id,
            payload,
            aad,
        } => match signing_seed(host, key_id, Purpose::Sign) {
            Ok(seed) => record(json!({
                "cose": text_of(&sign1(&seed, key_id.as_bytes(), &payload, &aad)),
            })),
            Err(error) => failure(operation, error.code()),
        },
        Request::Verify1 {
            trust_set,
            cose,
            aad,
        } => {
            let Some(trusted) = host.cose.trust.get(trust_set) else {
                return failure(operation, UNKNOWN_KEY);
            };
            // A payload the result cannot hold is no payload this operation
            // gives, so such an object is answered as one that does not verify.
            match verify1(trusted, &cose, &aad)
                .filter(|(_, payload)| payload.len() <= bounds.cose_payload)
            {
                Some((key_id, payload)) => record(json!({
                    "valid": true,
                    "key_id": key_id,
                    "payload": text_of(&payload),
                })),
                None => record(json!({ "valid": false, "key_id": "", "payload": "" })),
            }
        }
        Request::Encrypt0 {
            key_id,
            plaintext,
            aad,
        } => {
            let key = match symmetric_key(host, key_id, Purpose::Seal) {
                Ok(key) => key,
                Err(error) => return failure(operation, error.code()),
            };
            let Some(iv) = next_nonce(host) else {
                return failure(operation, UNAVAILABLE);
            };
            record(json!({
                "cose": text_of(&encrypt0(&key, key_id.as_bytes(), &iv, &plaintext, &aad)),
            }))
        }
        Request::Decrypt0 { key_id, cose, aad } => {
            let key = match symmetric_key(host, key_id, Purpose::Open) {
                Ok(key) => key,
                Err(error) => return failure(operation, error.code()),
            };
            // As for a verification: a plaintext past the record's bound does
            // not open here.
            match decrypt0(&key, &cose, &aad)
                .filter(|plaintext| plaintext.len() <= bounds.cose_plaintext)
            {
                Some(plaintext) => record(json!({
                    "opened": true,
                    "plaintext": text_of(&plaintext),
                })),
                None => record(json!({ "opened": false, "plaintext": "" })),
            }
        }
        Request::Derive { input, salt, info } => {
            let (named, material) = match &input {
                Input::Secret(secret) => (
                    DerivationInput::Secret(secret),
                    Zeroizing::new(secret.clone()),
                ),
                Input::Key(id) => match symmetric_key(host, id, Purpose::DeriveFrom) {
                    Ok(key) => (DerivationInput::Key(id), Zeroizing::new(key.to_vec())),
                    Err(error) => return failure(operation, error.code()),
                },
            };
            let now_ms = crate::clock::utc_now(host)
                .and_then(|ms| u64::try_from(ms).ok())
                .unwrap_or(0);
            let ReferenceHost {
                keyring, random, ..
            } = host;
            match keyring.derive(named, &material, &salt, &info, now_ms, random) {
                Ok(key_id) => record(json!({ "key_id": key_id })),
                Err(error) => failure(operation, error.code()),
            }
        }
    }
}

/// The bytes a byte string's text spells, one character to a byte (SCE
/// Accepted Subset section 2.12), or `None` for a text holding a character past
/// U+00FF, which spells no byte.
pub(crate) fn bytes_of(text: &str) -> Option<Vec<u8>> {
    text.chars().map(|c| u8::try_from(c).ok()).collect()
}

/// The text a byte string travels as: each byte the character of that code
/// point.
pub(crate) fn text_of(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}
