// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Signing and sealing (docs/SCE_HOST_FEATURES.md, section 10): COSE objects
//! (RFC 9052) made and read with keys the host holds and a statechart names by
//! id.
//!
//! The algorithms are fixed, not negotiated: EdDSA on Ed25519 (RFC 9053
//! section 2.2, COSE algorithm -8) for a `COSE_Sign1`, ChaCha20/Poly1305 (COSE
//! algorithm 24) under a 96-bit nonce drawn from the operating system for a
//! `COSE_Encrypt0`, and HKDF with SHA-256 (RFC 5869) for a derived key. Every
//! item is written in its deterministic form (RFC 8949 section 4.2.1) through
//! the CBOR heads the codecs write with (`sce_forge_runtime::cbor`), so an
//! object signed here is byte for byte the one another host signs from the
//! same key and payload. An object is read in any well-formed definite-length
//! encoding; one that is not the structure, or names another algorithm, does
//! not verify or open, which is an answer and not a failure.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hkdf::Hkdf;
use sce_forge_runtime::cbor::{
    read_array_len, read_bytes, read_head, read_map_len, read_text, skip, write_array_head,
    write_bytes, write_head, write_map_head, write_text, write_uint, MAJOR_TEXT, MAJOR_UNSIGNED,
};
use sce_forge_runtime::codec::{SceCursor, VecSink};
use serde_json::json;
use sha2::Sha256;

use crate::{failure, record, Outcome, Params, ReferenceHost, Transport, INVALID_ARGUMENT};

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

/// What the id of a derived key begins with: the host numbers them
/// `derived:1`, `derived:2`, ... in the order it derives them.
pub const DERIVED_PREFIX: &str = "derived:";

/// The failure of an operation whose key, trust set or input key the host does
/// not hold.
const UNKNOWN_KEY: &str = "unknown_key";

/// The failure of a seal the operating system gave no nonce for.
const UNAVAILABLE: &str = "unavailable";

// The bounds of the records' fields (`sce:std/host/cose_*.scxml`,
// `key_derive_*.scxml`), in bytes.
const KEY_ID_MAX: usize = 64;
const PAYLOAD_MAX: usize = 4096;
const PLAINTEXT_MAX: usize = 4352;
const SIGNED_MAX: usize = 4352;
const SEALED_MAX: usize = 4480;
const AAD_MAX: usize = 256;
const SECRET_MAX: usize = 256;
const SALT_MAX: usize = 64;
const INFO_MAX: usize = 256;

/// The bytes of a key: an Ed25519 seed, a ChaCha20/Poly1305 key, a public key.
const KEY_BYTES: usize = 32;
/// The bytes of an Ed25519 signature.
const SIGNATURE_BYTES: usize = 64;
/// The bytes of a ChaCha20/Poly1305 nonce: 96 bits.
const IV_BYTES: usize = 12;

/// Major type 1: the negative integer `-1 - n` for the argument `n` (RFC 8949
/// section 3.1).
const MAJOR_NEGATIVE: u8 = 1;
/// Major type 6: a tag (RFC 8949 section 3.4).
const MAJOR_TAG: u8 = 6;

/// The tags of a `COSE_Sign1` and a `COSE_Encrypt0` (RFC 9052 section 2).
const TAG_SIGN1: u64 = 18;
const TAG_ENCRYPT0: u64 = 16;

// The header labels these operations read and write (RFC 9052 section 3.1).
const LABEL_ALG: i64 = 1;
const LABEL_CRIT: i64 = 2;
const LABEL_KID: i64 = 4;
const LABEL_IV: i64 = 5;
const LABEL_PARTIAL_IV: i64 = 6;

/// EdDSA (RFC 9053 section 2.2).
const ALG_EDDSA: i64 = -8;
/// ChaCha20/Poly1305 with a 256-bit key and a 128-bit tag (RFC 9053 section 4.3).
const ALG_CHACHA20_POLY1305: i64 = 24;

/// The keys an embedding gives the host ([`ReferenceHost::set_cose_keys`]).
/// The statechart names each by its id and never holds one.
#[derive(Clone, Default)]
pub struct CoseKeys {
    /// The keys `sce-host-cose-sign1` signs with, by id: each the 32-byte seed
    /// of an Ed25519 key (RFC 8032 section 5.1.5).
    pub signing: BTreeMap<String, [u8; KEY_BYTES]>,
    /// The keys `sce-host-cose-encrypt0` and `sce-host-cose-decrypt0` seal and
    /// open with, and `sce-host-key-derive` derives from, by id: each a 32-byte
    /// ChaCha20/Poly1305 key. No id begins with [`DERIVED_PREFIX`].
    pub symmetric: BTreeMap<String, [u8; KEY_BYTES]>,
    /// The trust sets `sce-host-cose-verify1` verifies under, by name: the
    /// signers a statechart accepts, in the order they are tried.
    pub trust: BTreeMap<String, Vec<TrustedKey>>,
}

/// A signer of a trust set: its id and its Ed25519 public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedKey {
    pub id: String,
    pub public_key: [u8; KEY_BYTES],
}

/// A symmetric key the embedding gave under an id of the derived keys'
/// own namespace, [`DERIVED_PREFIX`], which would name two keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedKeyId(pub String);

impl fmt::Display for ReservedKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the symmetric key id `{}` begins with `{DERIVED_PREFIX}`, which names a derived key",
            self.0
        )
    }
}

impl std::error::Error for ReservedKeyId {}

/// The keys a host holds: those its embedding configured, and those its
/// derivations made, which a new configuration does not drop.
#[derive(Default)]
pub(crate) struct Keys {
    signing: BTreeMap<String, [u8; KEY_BYTES]>,
    symmetric: BTreeMap<String, [u8; KEY_BYTES]>,
    trust: BTreeMap<String, Vec<TrustedKey>>,
    derived: BTreeMap<String, [u8; KEY_BYTES]>,
    /// How many keys the host has derived, the number of the last one.
    derivations: u64,
}

impl Keys {
    /// Hold `keys` in place of the configured ones.
    pub(crate) fn configure(&mut self, keys: CoseKeys) -> Result<(), ReservedKeyId> {
        if let Some(id) = keys
            .symmetric
            .keys()
            .find(|id| id.starts_with(DERIVED_PREFIX))
        {
            return Err(ReservedKeyId(id.clone()));
        }
        self.signing = keys.signing;
        self.symmetric = keys.symmetric;
        self.trust = keys.trust;
        Ok(())
    }

    /// The symmetric key `id` names: a configured one, or one derived.
    fn symmetric_key(&self, id: &str) -> Option<[u8; KEY_BYTES]> {
        self.symmetric
            .get(id)
            .or_else(|| self.derived.get(id))
            .copied()
    }
}

/// The Ed25519 public key of the seed `seed`, which a trust set holds.
pub fn public_key_of(seed: &[u8; KEY_BYTES]) -> [u8; KEY_BYTES] {
    SigningKey::from_bytes(seed).verifying_key().to_bytes()
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
struct Arguments<'p>(&'p Params);

impl<'p> Arguments<'p> {
    fn text(&self, name: &str) -> &'p str {
        self.0.get(name).map(String::as_str).unwrap_or("")
    }

    /// A key id or a trust set's name: one to 64 bytes.
    fn id(&self, name: &str) -> Option<&'p str> {
        let id = self.text(name);
        (!id.is_empty() && id.len() <= KEY_ID_MAX).then_some(id)
    }

    /// A byte string of at most `max` bytes, from the text it travels as.
    fn bytes(&self, name: &str, max: usize) -> Option<Vec<u8>> {
        bytes_of(self.text(name)).filter(|bytes| bytes.len() <= max)
    }
}

/// The request `params` makes of `operation`, or `None` when the host refuses
/// it: an empty key id or trust set, a derivation given both or neither of a
/// secret and an input key, or a value its record's field does not hold.
fn request<'p>(operation: &str, params: &'p Params) -> Option<Request<'p>> {
    let arguments = Arguments(params);
    Some(match operation {
        SIGN1 => Request::Sign1 {
            key_id: arguments.id("key_id")?,
            payload: arguments.bytes("payload", PAYLOAD_MAX)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        VERIFY1 => Request::Verify1 {
            trust_set: arguments.id("trust_set")?,
            cose: arguments.bytes("cose", SIGNED_MAX)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        ENCRYPT0 => Request::Encrypt0 {
            key_id: arguments.id("key_id")?,
            plaintext: arguments.bytes("plaintext", PLAINTEXT_MAX)?,
            aad: arguments.bytes("external_aad", AAD_MAX)?,
        },
        DECRYPT0 => Request::Decrypt0 {
            key_id: arguments.id("key_id")?,
            cose: arguments.bytes("cose", SEALED_MAX)?,
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

/// Whether the host refuses `operation` with `params`.
pub(crate) fn refuses(operation: &str, params: &Params) -> bool {
    request(operation, params).is_none()
}

/// Perform `operation`, a request [`refuses`] accepted.
pub(crate) fn perform<T: Transport>(
    host: &mut ReferenceHost<T>,
    operation: &str,
    params: &Params,
) -> Outcome {
    let Some(request) = request(operation, params) else {
        return Outcome::Refuse(INVALID_ARGUMENT);
    };
    let keys = &mut host.cose;
    match request {
        Request::Sign1 {
            key_id,
            payload,
            aad,
        } => match keys.signing.get(key_id) {
            Some(seed) => record(json!({
                "cose": text_of(&sign1(seed, key_id.as_bytes(), &payload, &aad)),
            })),
            None => failure(operation, UNKNOWN_KEY),
        },
        Request::Verify1 {
            trust_set,
            cose,
            aad,
        } => {
            let Some(trusted) = keys.trust.get(trust_set) else {
                return failure(operation, UNKNOWN_KEY);
            };
            // A payload the result cannot hold is no payload this operation
            // gives, so such an object is answered as one that does not verify.
            match verify1(trusted, &cose, &aad).filter(|(_, payload)| payload.len() <= PAYLOAD_MAX)
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
            let Some(key) = keys.symmetric_key(key_id) else {
                return failure(operation, UNKNOWN_KEY);
            };
            let mut iv = [0u8; IV_BYTES];
            if getrandom::getrandom(&mut iv).is_err() {
                return failure(operation, UNAVAILABLE);
            }
            record(json!({
                "cose": text_of(&encrypt0(&key, key_id.as_bytes(), &iv, &plaintext, &aad)),
            }))
        }
        Request::Decrypt0 { key_id, cose, aad } => {
            let Some(key) = keys.symmetric_key(key_id) else {
                return failure(operation, UNKNOWN_KEY);
            };
            // As for a verification: a plaintext past the record's bound does
            // not open here.
            match decrypt0(&key, &cose, &aad).filter(|plaintext| plaintext.len() <= PLAINTEXT_MAX) {
                Some(plaintext) => record(json!({
                    "opened": true,
                    "plaintext": text_of(&plaintext),
                })),
                None => record(json!({ "opened": false, "plaintext": "" })),
            }
        }
        Request::Derive { input, salt, info } => {
            let material = match input {
                Input::Secret(secret) => secret,
                Input::Key(id) => match keys.symmetric_key(id) {
                    Some(key) => key.to_vec(),
                    None => return failure(operation, UNKNOWN_KEY),
                },
            };
            let key = derive_key(&material, &salt, &info);
            keys.derivations += 1;
            let key_id = format!("{DERIVED_PREFIX}{}", keys.derivations);
            keys.derived.insert(key_id.clone(), key);
            record(json!({ "key_id": key_id }))
        }
    }
}

/// The bytes a byte string's text spells, one character to a byte (SCE
/// Accepted Subset section 2.12), or `None` for a text holding a character past
/// U+00FF, which spells no byte.
fn bytes_of(text: &str) -> Option<Vec<u8>> {
    text.chars().map(|c| u8::try_from(c).ok()).collect()
}

/// The text a byte string travels as: each byte the character of that code
/// point.
fn text_of(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

// ── The structures ─────────────────────────────────────────────────────

/// A write to a growing vector, which cannot fail.
fn put(written: Result<(), Infallible>) {
    written.unwrap_or_else(|never| match never {});
}

/// Write an integer, major type 0 for zero or more and 1 below zero.
fn write_int(w: &mut VecSink<'_>, value: i64) {
    if value >= 0 {
        put(write_uint(w, value.unsigned_abs()));
    } else {
        put(write_head(w, MAJOR_NEGATIVE, value.unsigned_abs() - 1));
    }
}

/// The protected header of a `COSE_Sign1`: the algorithm and the key's id,
/// labels in ascending order.
fn protected_sign1(kid: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_map_head(&mut w, 2));
    write_int(&mut w, LABEL_ALG);
    write_int(&mut w, ALG_EDDSA);
    write_int(&mut w, LABEL_KID);
    put(write_bytes(&mut w, kid));
    out
}

/// The protected header of a `COSE_Encrypt0`: the algorithm alone.
fn protected_encrypt0() -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_map_head(&mut w, 1));
    write_int(&mut w, LABEL_ALG);
    write_int(&mut w, ALG_CHACHA20_POLY1305);
    out
}

/// What a `COSE_Sign1` signs (RFC 9052 section 4.4): `Sig_structure` of the
/// context `Signature1`.
fn sig_structure(protected: &[u8], external_aad: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_array_head(&mut w, 4));
    put(write_text(&mut w, "Signature1"));
    put(write_bytes(&mut w, protected));
    put(write_bytes(&mut w, external_aad));
    put(write_bytes(&mut w, payload));
    out
}

/// What a `COSE_Encrypt0` authenticates beside its plaintext (RFC 9052 section
/// 5.3): `Enc_structure` of the context `Encrypt0`.
fn enc_structure(protected: &[u8], external_aad: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_array_head(&mut w, 3));
    put(write_text(&mut w, "Encrypt0"));
    put(write_bytes(&mut w, protected));
    put(write_bytes(&mut w, external_aad));
    out
}

/// The tagged `COSE_Sign1` of `payload`, signed with the Ed25519 key of `seed`
/// over `external_aad` too, `kid` in its protected header and an empty
/// unprotected one.
pub fn sign1(seed: &[u8; KEY_BYTES], kid: &[u8], payload: &[u8], external_aad: &[u8]) -> Vec<u8> {
    let protected = protected_sign1(kid);
    let signature = SigningKey::from_bytes(seed)
        .sign(&sig_structure(&protected, external_aad, payload))
        .to_bytes();
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_head(&mut w, MAJOR_TAG, TAG_SIGN1));
    put(write_array_head(&mut w, 4));
    put(write_bytes(&mut w, &protected));
    put(write_map_head(&mut w, 0));
    put(write_bytes(&mut w, payload));
    put(write_bytes(&mut w, &signature));
    out
}

/// The tagged `COSE_Encrypt0` of `plaintext` under `key` and the nonce `iv`,
/// authenticating `external_aad` too, `kid` and `iv` in its unprotected header.
/// The nonce is the caller's to draw: one used twice under a key gives the two
/// plaintexts away.
pub fn encrypt0(
    key: &[u8; KEY_BYTES],
    kid: &[u8],
    iv: &[u8; IV_BYTES],
    plaintext: &[u8],
    external_aad: &[u8],
) -> Vec<u8> {
    let protected = protected_encrypt0();
    let ciphertext = ChaCha20Poly1305::new(Key::from_slice(key))
        .encrypt(
            Nonce::from_slice(iv),
            Payload {
                msg: plaintext,
                aad: &enc_structure(&protected, external_aad),
            },
        )
        .expect("ChaCha20/Poly1305 seals any message shorter than 2^38 bytes");
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_head(&mut w, MAJOR_TAG, TAG_ENCRYPT0));
    put(write_array_head(&mut w, 3));
    put(write_bytes(&mut w, &protected));
    put(write_map_head(&mut w, 2));
    write_int(&mut w, LABEL_KID);
    put(write_bytes(&mut w, kid));
    write_int(&mut w, LABEL_IV);
    put(write_bytes(&mut w, iv));
    put(write_bytes(&mut w, &ciphertext));
    out
}

/// The key HKDF-SHA-256 (RFC 5869) derives from `input` with `salt` and
/// `info`: 32 bytes. An empty salt is no salt, which RFC 5869 section 2.2
/// reads as a string of zeros.
pub fn derive_key(input: &[u8], salt: &[u8], info: &[u8]) -> [u8; KEY_BYTES] {
    let mut key = [0u8; KEY_BYTES];
    Hkdf::<Sha256>::new((!salt.is_empty()).then_some(salt), input)
        .expand(info, &mut key)
        .expect("32 bytes is a length HKDF-SHA-256 gives");
    key
}

/// A label of a header map (RFC 9052 section 3): an integer or a text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Label {
    Int(i128),
    Text(String),
}

/// The algorithm a header names: an integer, or anything else (a text names
/// an algorithm these operations do not use).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    Int(i64),
    Other,
}

/// What one header bucket says, of what these operations read.
#[derive(Debug, Default)]
struct Headers {
    /// Every label of the bucket, which none may repeat.
    labels: Vec<Label>,
    alg: Option<Algorithm>,
    /// Whether the bucket lists headers critical to its reader: these
    /// operations process none beyond what they read, so an object that has
    /// any is not one they accept.
    crit: bool,
    kid: Option<Vec<u8>>,
    iv: Option<Vec<u8>>,
    partial_iv: bool,
}

/// An integer item, of either sign.
fn read_int(c: &mut SceCursor<'_>) -> Option<i128> {
    match read_head(c).ok()? {
        (MAJOR_UNSIGNED, n) => Some(i128::from(n)),
        (MAJOR_NEGATIVE, n) => Some(-1 - i128::from(n)),
        _ => None,
    }
}

/// The major type of the next item, without reading it.
fn major_of(c: &SceCursor<'_>) -> Option<u8> {
    let mut probe = *c;
    read_head(&mut probe).ok().map(|(major, _)| major)
}

fn read_label(c: &mut SceCursor<'_>) -> Option<Label> {
    match major_of(c)? {
        MAJOR_UNSIGNED | MAJOR_NEGATIVE => read_int(c).map(Label::Int),
        MAJOR_TEXT => read_text(c, None)
            .ok()
            .map(|text| Label::Text(text.to_string())),
        _ => None,
    }
}

fn read_algorithm(c: &mut SceCursor<'_>) -> Option<Algorithm> {
    match major_of(c)? {
        MAJOR_UNSIGNED | MAJOR_NEGATIVE => Some(
            read_int(c)
                .and_then(|n| i64::try_from(n).ok())
                .map_or(Algorithm::Other, Algorithm::Int),
        ),
        _ => {
            skip(c).ok()?;
            Some(Algorithm::Other)
        }
    }
}

/// One header map, or `None` for one that is malformed, repeats a label, or
/// gives a header these operations read a value of the wrong type.
fn read_headers(c: &mut SceCursor<'_>) -> Option<Headers> {
    let entries = read_map_len(c).ok()?;
    let mut headers = Headers::default();
    for _ in 0..entries {
        let label = read_label(c)?;
        if headers.labels.contains(&label) {
            return None;
        }
        let known = match &label {
            Label::Int(n) => i64::try_from(*n).ok(),
            Label::Text(_) => None,
        };
        match known {
            Some(LABEL_ALG) => headers.alg = Some(read_algorithm(c)?),
            Some(LABEL_CRIT) => {
                skip(c).ok()?;
                headers.crit = true;
            }
            Some(LABEL_KID) => headers.kid = Some(read_bytes(c, None).ok()?.to_vec()),
            Some(LABEL_IV) => headers.iv = Some(read_bytes(c, None).ok()?.to_vec()),
            Some(LABEL_PARTIAL_IV) => {
                skip(c).ok()?;
                headers.partial_iv = true;
            }
            _ => skip(c).ok()?,
        }
        headers.labels.push(label);
    }
    Some(headers)
}

/// The protected bucket, from the byte string that carries it: an empty one is
/// an empty map (RFC 9052 section 3).
fn protected_headers(encoded: &[u8]) -> Option<Headers> {
    if encoded.is_empty() {
        return Some(Headers::default());
    }
    let mut c = SceCursor::new(encoded);
    let headers = read_headers(&mut c)?;
    (c.remaining() == 0).then_some(headers)
}

/// The two buckets of an object, held to RFC 9052 section 3: no label in both,
/// and no header critical to a reader, since these operations process none
/// beyond what they read.
fn buckets(protected: Headers, unprotected: Headers) -> Option<(Headers, Headers)> {
    let shared = protected
        .labels
        .iter()
        .any(|label| unprotected.labels.contains(label));
    (!shared && !protected.crit && !unprotected.crit).then_some((protected, unprotected))
}

/// The tag and the array head of an object, read: `true` when they are `tag`
/// and `items`.
fn read_frame(c: &mut SceCursor<'_>, tag: u64, items: u64) -> bool {
    read_head(c).ok() == Some((MAJOR_TAG, tag)) && read_array_len(c).ok() == Some(items)
}

/// The signer of `cose` among `trusted` and the payload it signed over
/// `external_aad` too, or `None` when no key of the set signed it, or it is
/// not a tagged `COSE_Sign1` of EdDSA. A key id the object carries, in either
/// bucket, selects the signer of that id alone; an object with none is tried
/// against every signer, in the set's order.
pub fn verify1(
    trusted: &[TrustedKey],
    cose: &[u8],
    external_aad: &[u8],
) -> Option<(String, Vec<u8>)> {
    let mut c = SceCursor::new(cose);
    if !read_frame(&mut c, TAG_SIGN1, 4) {
        return None;
    }
    let protected = read_bytes(&mut c, None).ok()?;
    let unprotected = read_headers(&mut c)?;
    // A detached payload (`nil`) is no byte string, and is not verified here.
    let payload = read_bytes(&mut c, None).ok()?;
    let signature = read_bytes(&mut c, None).ok()?;
    if c.remaining() != 0 {
        return None;
    }
    let (headers, unprotected) = buckets(protected_headers(protected)?, unprotected)?;
    if headers.alg != Some(Algorithm::Int(ALG_EDDSA)) {
        return None;
    }
    let signature = Signature::from_bytes(&<[u8; SIGNATURE_BYTES]>::try_from(signature).ok()?);
    let to_be_signed = sig_structure(protected, external_aad, payload);
    let kid = headers.kid.or(unprotected.kid);
    trusted
        .iter()
        .filter(|signer| match &kid {
            Some(kid) => kid.as_slice() == signer.id.as_bytes(),
            None => true,
        })
        .find(|signer| {
            VerifyingKey::from_bytes(&signer.public_key)
                .is_ok_and(|key| key.verify_strict(&to_be_signed, &signature).is_ok())
        })
        .map(|signer| (signer.id.clone(), payload.to_vec()))
}

/// The plaintext of `cose` opened with `key` over `external_aad` too, or
/// `None` when it does not open, or is not a tagged `COSE_Encrypt0` of
/// ChaCha20/Poly1305 with a whole 96-bit nonce. The key id the object carries
/// is not read: the caller names the key.
pub fn decrypt0(key: &[u8; KEY_BYTES], cose: &[u8], external_aad: &[u8]) -> Option<Vec<u8>> {
    let mut c = SceCursor::new(cose);
    if !read_frame(&mut c, TAG_ENCRYPT0, 3) {
        return None;
    }
    let protected = read_bytes(&mut c, None).ok()?;
    let unprotected = read_headers(&mut c)?;
    // A detached ciphertext (`nil`) is no byte string, and does not open here.
    let ciphertext = read_bytes(&mut c, None).ok()?;
    if c.remaining() != 0 {
        return None;
    }
    let (headers, unprotected) = buckets(protected_headers(protected)?, unprotected)?;
    if headers.alg != Some(Algorithm::Int(ALG_CHACHA20_POLY1305))
        || headers.partial_iv
        || unprotected.partial_iv
    {
        return None;
    }
    let iv = headers.iv.or(unprotected.iv)?;
    if iv.len() != IV_BYTES {
        return None;
    }
    ChaCha20Poly1305::new(Key::from_slice(key))
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: ciphertext,
                aad: &enc_structure(protected, external_aad),
            },
        )
        .ok()
}
