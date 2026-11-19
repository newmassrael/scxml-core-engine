// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! COSE objects (RFC 9052) and the keys they are made with: the primitives of
//! docs/SCE_HOST_FEATURES.md sections 10 and 13.
//!
//! The algorithms are fixed, not negotiated: EdDSA on Ed25519 (RFC 9053 section 2.2, COSE
//! algorithm -8) for a `COSE_Sign1`, ChaCha20/Poly1305 (COSE algorithm 24) under a 96-bit nonce the
//! caller draws for a `COSE_Encrypt0`, and HKDF with SHA-256 (RFC 5869) for a derived key. A public
//! key leaves as a `COSE_Key` (RFC 9052 section 7) of key type OKP and curve Ed25519 (RFC 9053
//! section 7.2). Every item is written in its deterministic form (RFC 8949 section 4.2.1) through
//! the CBOR heads the codecs write with (`sce_forge_runtime::cbor`), so an object signed here is
//! byte for byte the one another host signs from the same key and payload. An object is read in
//! any well-formed definite-length encoding; one that is not the structure, or names another
//! algorithm, does not verify or open, which is an answer and not a failure.

use std::convert::Infallible;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use sce_forge_runtime::cbor::{
    read_array_len, read_bytes, read_head, read_map_len, read_text, skip, write_array_head,
    write_bytes, write_head, write_map_head, write_text, write_uint, MAJOR_TEXT, MAJOR_UNSIGNED,
};
use sce_forge_runtime::codec::{SceCursor, VecSink};
use sha2::{Digest, Sha256};

use crate::ed25519;

/// The bytes of a key: an Ed25519 seed, a ChaCha20/Poly1305 key, a public key, a derived key.
pub const KEY_BYTES: usize = 32;
/// The bytes of a COSE key thumbprint (RFC 9679): the whole output of SHA-256.
pub const THUMBPRINT_BYTES: usize = 32;
/// The bytes of a ChaCha20/Poly1305 nonce: 96 bits.
pub const IV_BYTES: usize = 12;

/// Major type 1: the negative integer `-1 - n` for the argument `n` (RFC 8949 section 3.1).
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

// The parameters of a `COSE_Key` this crate writes (RFC 9052 section 7.1, RFC 9053 section 7.2).
const KEY_LABEL_KTY: i64 = 1;
const KEY_LABEL_KID: i64 = 2;
const KEY_LABEL_CRV: i64 = -1;
const KEY_LABEL_X: i64 = -2;
/// Key type OKP, an octet key pair (RFC 9053 section 7.2).
const KTY_OKP: i64 = 1;
/// The curve Ed25519 (RFC 9053 section 7.1).
const CRV_ED25519: i64 = 6;

/// EdDSA (RFC 9053 section 2.2).
const ALG_EDDSA: i64 = -8;
/// ChaCha20/Poly1305 with a 256-bit key and a 128-bit tag (RFC 9053 section 4.3).
const ALG_CHACHA20_POLY1305: i64 = 24;

/// A signer of a trust set: its id and its Ed25519 public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedKey {
    pub id: String,
    pub public_key: [u8; KEY_BYTES],
}

/// The Ed25519 public key of the seed `seed`, which a trust set holds.
pub fn public_key_of(seed: &[u8; KEY_BYTES]) -> [u8; KEY_BYTES] {
    ed25519::public_key_of(seed)
}

// ── Writing ────────────────────────────────────────────────────────────

/// A write to a growing vector, which cannot fail.
pub(crate) fn put(written: Result<(), Infallible>) {
    written.unwrap_or_else(|never| match never {});
}

/// Write an integer, major type 0 for zero or more and 1 below zero.
pub(crate) fn write_int(w: &mut VecSink<'_>, value: i64) {
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
    let signature = ed25519::sign(seed, &sig_structure(&protected, external_aad, payload));
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

/// The `COSE_Key` of an Ed25519 public key (RFC 9052 section 7; RFC 9053 section 7.2): the map
/// `{1: 1, 2: kid, -1: 6, -2: x}` of key type OKP, its key id, curve Ed25519 and the public key,
/// its labels in the bytewise order of their encodings (RFC 8949 section 4.2.1), which puts the
/// two positive labels before the two negative ones.
pub fn cose_key_ed25519(kid: &[u8], public_key: &[u8; KEY_BYTES]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_map_head(&mut w, 4));
    write_int(&mut w, KEY_LABEL_KTY);
    write_int(&mut w, KTY_OKP);
    write_int(&mut w, KEY_LABEL_KID);
    put(write_bytes(&mut w, kid));
    write_int(&mut w, KEY_LABEL_CRV);
    write_int(&mut w, CRV_ED25519);
    write_int(&mut w, KEY_LABEL_X);
    put(write_bytes(&mut w, public_key));
    out
}

// ── Reading ────────────────────────────────────────────────────────────

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
    if signature.len() != ed25519::SIGNATURE_BYTES {
        return None;
    }
    let to_be_signed = sig_structure(protected, external_aad, payload);
    let kid = headers.kid.or(unprotected.kid);
    trusted
        .iter()
        .filter(|signer| match &kid {
            Some(kid) => kid.as_slice() == signer.id.as_bytes(),
            None => true,
        })
        .find(|signer| ed25519::verify(&signer.public_key, &to_be_signed, signature))
        .map(|signer| (signer.id.clone(), payload.to_vec()))
}

/// The `COSE_Key` RFC 9679 takes the thumbprint of for an Ed25519 public key: the map of the
/// members its key type requires and nothing else, `{1: 1, -1: 6, -2: x}` (key type OKP, curve
/// Ed25519, the key), in its deterministic encoding (RFC 8949 section 4.2.1). It names the
/// algorithm and the curve with the key, so it is also how a key is carried where only its bytes
/// would be ambiguous: in a header that offers it, and in a pin a server keeps of it.
pub fn cose_key_ed25519_required(public_key: &[u8; KEY_BYTES]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = VecSink::new(&mut out);
    put(write_map_head(&mut w, 3));
    write_int(&mut w, KEY_LABEL_KTY);
    write_int(&mut w, KTY_OKP);
    write_int(&mut w, KEY_LABEL_CRV);
    write_int(&mut w, CRV_ED25519);
    write_int(&mut w, KEY_LABEL_X);
    put(write_bytes(&mut w, public_key));
    out
}

/// The Ed25519 public key `cose_key` carries when it is exactly the encoding
/// [`cose_key_ed25519_required`] writes, and `None` for anything else: another key type or curve,
/// another member, another encoding of the same map, or bytes after it.
pub fn ed25519_of_cose_key(cose_key: &[u8]) -> Option<[u8; KEY_BYTES]> {
    let prefix_length = cose_key.len().checked_sub(KEY_BYTES)?;
    let public_key: [u8; KEY_BYTES] = cose_key[prefix_length..].try_into().ok()?;
    (cose_key_ed25519_required(&public_key) == cose_key).then_some(public_key)
}

/// The COSE key thumbprint of an Ed25519 public key (RFC 9679 section 3): the SHA-256 of
/// [`cose_key_ed25519_required`].
pub fn thumbprint_ed25519(public_key: &[u8; KEY_BYTES]) -> [u8; THUMBPRINT_BYTES] {
    Sha256::digest(cose_key_ed25519_required(public_key)).into()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes_of_hex;

    /// The seed of RFC 8032 section 7.1, test 1, and its public key.
    const SEED_1: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const PUBLIC_1: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    /// The `COSE_Key` RFC 9052's grammar and RFC 8949's deterministic order make of that public
    /// key under the key id `11`, written out by hand: `a4` a map of four, `01 01` kty OKP, `02 42
    /// 3131` kid, `20 06` crv Ed25519, `21 5820 ...` x.
    const COSE_KEY_1: &str = "a40101024231312006215820\
                              d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    #[test]
    fn a_public_key_leaves_as_the_deterministic_cose_key() {
        let seed: [u8; KEY_BYTES] = bytes_of_hex(SEED_1)
            .expect("hex")
            .try_into()
            .expect("32 bytes");
        let public = public_key_of(&seed);
        assert_eq!(public.to_vec(), bytes_of_hex(PUBLIC_1).expect("hex"));
        assert_eq!(
            cose_key_ed25519(b"11", &public),
            bytes_of_hex(COSE_KEY_1).expect("hex")
        );
    }

    /// RFC 9679's input for the public key of RFC 8032's test 1, written out by hand: `a3` a map of
    /// three, `01 01` kty OKP, `20 06` crv Ed25519, `21 5820 ...` x; and its SHA-256, computed apart
    /// from this crate. Red if the thumbprint takes in another member (the key id) or another order.
    #[test]
    fn a_thumbprint_digests_the_required_members_alone() {
        let public: [u8; KEY_BYTES] = bytes_of_hex(PUBLIC_1)
            .expect("hex")
            .try_into()
            .expect("32 bytes");
        let required = cose_key_ed25519_required(&public);
        assert_eq!(
            required,
            bytes_of_hex(
                "a301012006215820d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
            )
            .expect("hex")
        );
        assert_eq!(
            thumbprint_ed25519(&public).to_vec(),
            bytes_of_hex("866eefbd6718c8846cd7ddfe43fc74ab1daac4538ff8514ea2ec2d410a415743")
                .expect("hex")
        );
        assert_eq!(ed25519_of_cose_key(&required), Some(public));
        assert_eq!(
            ed25519_of_cose_key(&cose_key_ed25519(b"11", &public)),
            None,
            "a key that carries a key id is not the required form"
        );
        assert_eq!(ed25519_of_cose_key(&required[1..]), None);
        assert_eq!(ed25519_of_cose_key(&[0u8; 8]), None, "shorter than a key");
    }
}
