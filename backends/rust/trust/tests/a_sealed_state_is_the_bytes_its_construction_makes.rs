// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A known-answer vector of the sealed state (`backup::seal_state`), the form a device keeps of itself
// between runs under a data key its host's platform key store wraps (docs/SCE_SHARED_LIST_PEER_ABI.md
// section 6). A device's state sealed by one build is opened by the next, and by any other
// implementation of the form; a change to how the key, the nonce or the bytes sealed are made would
// open nothing that exists, and would pass every test that seals and opens in one process.
//
// Two things are asked of the vector. That the bytes `seal_state` gives for fixed inputs are the ones
// recorded (`tests/fixtures/sealed_state.json`), so that the form does not drift unseen: a change is a
// decision, made by recording again. And that they are the bytes the documented construction gives,
// made here again from the primitives alone and not from the crate's own code: the contents written
// as `SCEB1` and each part after its length; the associated data as the domain, the list, the node and
// the form, each after its length; the cipher's key and the nonce's key drawn from the data key by
// HKDF-Expand with SHA-256 under their labels (RFC 5869 section 2.3, written here as the one HMAC block
// it is for 32 bytes); the nonce HMAC-SHA-256 under the nonce's key of the associated data's length,
// the associated data and the contents, cut to 24 bytes; and the whole `SCES1`, the nonce, and the
// contents sealed with XChaCha20-Poly1305 under the cipher's key with that associated data. So the
// vector checks the construction, not only that the bytes stay the same. The recorded file also holds
// the keys and the nonce on the way, for an implementation in another language to check itself step
// by step.
//
// The vector is recorded again with `SCE_SEALED_KAT=write cargo test -p sce-rust-trust --test
// a_sealed_state_is_the_bytes_its_construction_makes`.

use std::path::Path;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use sce_rust_trust::backup::{open_state, seal_state, Contents, DataKey};
use serde_json::{json, Value};
use sha2::Sha256;

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/sealed_state.json"
);
const WRITE_AGAIN: &str = "SCE_SEALED_KAT=write cargo test -p sce-rust-trust --test \
     a_sealed_state_is_the_bytes_its_construction_makes";

/// The data key the vector is sealed under.
const KEY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];
const LIST: &str = "home";
const NODE: u64 = 4;
/// The seed of the device's key, which the contents hold.
const SEED: [u8; 32] = [7; 32];
/// What the caller names the form it seals, with its version: the peer core's state.
const CONTEXT: &str = "sce-peer/state/4";

/// The documented domain, magic texts and labels of the form, written here as the construction
/// names them, not read from the crate.
const STATE_DOMAIN: &[u8] = b"sce-trust/state/1";
const CONTENTS_MAGIC: &[u8] = b"SCEB1";
const STATE_MAGIC: &[u8] = b"SCES1";
const SEAL_LABEL: &[u8] = b"sce-trust/state/1 seal";
const NONCE_LABEL: &[u8] = b"sce-trust/state/1 nonce";
/// The counter HKDF-Expand ends its first block with.
const FIRST_BLOCK: &[u8] = &[1];

/// What the vector seals beside the seed: named extras, in the order they are added, as a device
/// keeps its MLS state, the key of an epoch and the state of its core.
fn extras() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("mls", vec![1, 2, 3]),
        ("epoch-key/2", vec![9; 32]),
        ("peer/state", br#"{"version":4}"#.to_vec()),
    ]
}

fn contents() -> Contents {
    extras().into_iter().fold(
        Contents::new(LIST, NODE, SEED),
        |contents, (name, bytes)| contents.with(name, bytes),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap())
        .collect()
}

/// `part` after its length, a big-endian 64-bit number.
fn put(out: &mut Vec<u8>, part: &[u8]) {
    out.extend_from_slice(&(part.len() as u64).to_be_bytes());
    out.extend_from_slice(part);
}

/// HMAC-SHA-256 under `key` of `parts`, one after the other.
fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).unwrap();
    for part in parts {
        mac.update(part);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&mac.finalize().into_bytes());
    out
}

/// What the construction gives on the way, and at the end.
struct Made {
    plain: Vec<u8>,
    header: Vec<u8>,
    cipher_key: [u8; 32],
    nonce_key: [u8; 32],
    nonce: [u8; 24],
    sealed: Vec<u8>,
}

/// The sealed state of the vector's inputs, made from the documented construction with the
/// primitives alone.
fn by_construction() -> Made {
    let mut plain = CONTENTS_MAGIC.to_vec();
    put(&mut plain, LIST.as_bytes());
    put(&mut plain, &NODE.to_be_bytes());
    put(&mut plain, &SEED);
    put(&mut plain, &(extras().len() as u64).to_be_bytes());
    for (name, bytes) in extras() {
        put(&mut plain, name.as_bytes());
        put(&mut plain, &bytes);
    }

    let node = NODE.to_be_bytes();
    let parts: [&[u8]; 4] = [STATE_DOMAIN, LIST.as_bytes(), &node, CONTEXT.as_bytes()];
    let mut header = Vec::new();
    for part in parts {
        put(&mut header, part);
    }

    // HKDF-Expand for 32 bytes is one block: T(1) = HMAC(PRK, info || 0x01), and the data key is
    // the pseudorandom key.
    let cipher_key = hmac(&KEY, &[SEAL_LABEL, FIRST_BLOCK]);
    let nonce_key = hmac(&KEY, &[NONCE_LABEL, FIRST_BLOCK]);
    let length = (header.len() as u64).to_be_bytes();
    let hashed: [&[u8]; 3] = [&length, &header, &plain];
    let tag = hmac(&nonce_key, &hashed);
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&tag[..24]);
    let ciphertext = XChaCha20Poly1305::new(Key::from_slice(&cipher_key))
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: &header,
            },
        )
        .unwrap();
    let mut sealed = STATE_MAGIC.to_vec();
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&ciphertext);
    Made {
        plain,
        header,
        cipher_key,
        nonce_key,
        nonce,
        sealed,
    }
}

/// The vector as the file records it: the inputs, what is made on the way, and the sealed bytes.
fn vector(made: &Made) -> Value {
    json!({
        "form": "SCES1 || nonce (24) || XChaCha20-Poly1305(cipher key, nonce, contents, associated data)",
        "key": hex(&KEY),
        "list": LIST,
        "node": NODE.to_string(),
        "seed": hex(&SEED),
        "extras": extras()
            .into_iter()
            .map(|(name, bytes)| json!([name, hex(&bytes)]))
            .collect::<Vec<_>>(),
        "context": CONTEXT,
        "contents": hex(&made.plain),
        "associated_data": hex(&made.header),
        "cipher_key": hex(&made.cipher_key),
        "nonce_key": hex(&made.nonce_key),
        "nonce": hex(&made.nonce),
        "sealed": hex(&made.sealed),
    })
}

/// The crate seals the vector's inputs into the bytes the construction gives; the recording is those
/// bytes, and what it records on the way is what the construction makes; and the recorded bytes open
/// under the key, for the device they were sealed for, into the contents sealed.
#[test]
fn a_sealed_state_is_the_recorded_vector_and_what_its_construction_gives() {
    let made = by_construction();
    let sealed = seal_state(&DataKey::from_bytes(KEY), CONTEXT, &contents()).unwrap();
    assert_eq!(
        hex(&sealed),
        hex(&made.sealed),
        "the crate seals what the documented construction gives"
    );

    let fresh = vector(&made);
    if std::env::var("SCE_SEALED_KAT").as_deref() == Ok("write") {
        std::fs::create_dir_all(Path::new(PATH).parent().unwrap()).unwrap();
        std::fs::write(PATH, serde_json::to_string_pretty(&fresh).unwrap() + "\n").unwrap();
    }
    let recorded: Value = serde_json::from_str(
        &std::fs::read_to_string(PATH)
            .unwrap_or_else(|_| panic!("{PATH} is not recorded: {WRITE_AGAIN}")),
    )
    .expect("the recording is JSON");
    for name in [
        "key",
        "list",
        "node",
        "seed",
        "extras",
        "context",
        "contents",
        "associated_data",
        "cipher_key",
        "nonce_key",
        "nonce",
        "sealed",
    ] {
        assert_eq!(
            recorded[name], fresh[name],
            "`{name}` is not what was recorded; a change of the sealed form is recorded again \
             with {WRITE_AGAIN}"
        );
    }

    let bytes = unhex(recorded["sealed"].as_str().unwrap());
    assert_eq!(
        open_state(&DataKey::from_bytes(KEY), LIST, NODE, CONTEXT, &bytes),
        Ok(contents()),
        "the recorded state opens into what was sealed"
    );
}
