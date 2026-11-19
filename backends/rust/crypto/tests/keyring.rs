// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The key ring (ADR 0109, D7) over the key stores of `sce-rust-session-store`: one use per key, a
//! generation answered once, a retired key that makes nothing new, a derived key named by what
//! derived it in every process that shares the store, and material no one reads without the
//! key-encryption key, across a reopened file and a rotation of that key.
//!
//! The derivations and the generations are those of `tests/host_features/keys_cross_language.json`,
//! whose values were computed apart from this crate; the reference hosts of Rust and C++ are held
//! to the same file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sce_rust_crypto::bytes_of_hex;
use sce_rust_crypto::cose::{cose_key_ed25519, public_key_of, KEY_BYTES};
use sce_rust_crypto::keyring::{
    DerivationInput, KeyError, KeyState, KeyStore, KeyUse, Keyring, MemoryKeys, Purpose,
    ISSUING_KEY_ID,
};
use sce_rust_crypto::mac::hmac_sha256;
use sce_rust_crypto::random::ScriptedRandom;
use sce_rust_crypto::wrap::KeyEncryptionKey;
use sce_rust_session_store::SqliteStore;
use serde_json::Value;

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/keys_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

fn hex(value: &Value) -> Vec<u8> {
    bytes_of_hex(value.as_str().expect("a hexadecimal text")).expect("hexadecimal")
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a text")
}

fn kek(byte: u8) -> Box<KeyEncryptionKey> {
    Box::new(KeyEncryptionKey::new([byte; 32]))
}

/// A key ring over `store` whose key-encryption key is 32 bytes of `byte`.
fn ring(store: impl KeyStore + 'static, byte: u8) -> Keyring {
    Keyring::new(Box::new(store), kek(byte))
}

/// A source that gives `bytes` first.
fn scripted(bytes: &[u8]) -> ScriptedRandom {
    let mut random = ScriptedRandom::new();
    random.set(bytes);
    random
}

/// The generation of the vector file whose `key_use` is `key_use`.
fn generation(key_use: &str) -> Value {
    vectors()["generated"]
        .as_array()
        .expect("generations")
        .iter()
        .find(|g| g["key_use"] == key_use)
        .expect("a generation of the use")
        .clone()
}

#[test]
fn a_generated_key_has_one_use_and_its_generation_is_answered_once() {
    let keys = ring(MemoryKeys::new(), 1);
    let signing = generation("sign");
    let drawn = hex(&signing["random"]);
    let mut random = scripted(&drawn);
    let id = keys
        .generate(KeyUse::Sign, "device", "session/gen/1", 5, &mut random)
        .expect("a generation");
    assert_eq!(id, text(&signing["key_id"]));
    assert_eq!(random.queued(), 0, "the id's 16 bytes and the key's 32");
    let seed: [u8; KEY_BYTES] = drawn[16..].try_into().expect("32 bytes");
    assert_eq!(
        *keys.key(&id, Purpose::Sign).expect("it signs"),
        seed,
        "the material is the 32 bytes drawn after the id"
    );
    assert_eq!(
        keys.public_key(&id).expect("its public key"),
        public_key_of(&seed)
    );
    assert_eq!(
        cose_key_ed25519(id.as_bytes(), &keys.public_key(&id).expect("again")),
        hex(&signing["cose_key"])
    );
    for other in [
        Purpose::Seal,
        Purpose::Open,
        Purpose::Tag,
        Purpose::Verify,
        Purpose::DeriveFrom,
    ] {
        assert_eq!(
            keys.material(&id, other).err(),
            Some(KeyError::Unknown),
            "a signing key is no key of {other:?}"
        );
    }
    let row = keys.store().key(&id).expect("its row").expect("kept");
    assert_eq!(
        (row.key_use, row.label.as_str(), row.created_ms),
        (KeyUse::Sign, "device", 5)
    );

    random.set(&[0xEE; 48]);
    assert_eq!(
        keys.generate(KeyUse::Sign, "device", "session/gen/1", 9, &mut random)
            .expect("the generation repeated"),
        id,
        "a repeated generation answers its first key"
    );
    assert_eq!(random.queued(), 48, "and draws nothing");
    assert_eq!(keys.store().ids().expect("the ids"), [id]);
}

#[test]
fn a_retired_key_verifies_opens_and_exports_and_makes_nothing_new() {
    let keys = ring(MemoryKeys::new(), 1);
    let mut random = ScriptedRandom::new();
    let sign = keys
        .generate(KeyUse::Sign, "", "g-sign", 0, &mut random)
        .expect("a signing key");
    let seal = keys
        .generate(KeyUse::Seal, "", "g-seal", 0, &mut random)
        .expect("a sealing key");
    let mac = keys
        .generate(KeyUse::Mac, "", "g-mac", 0, &mut random)
        .expect("a MAC key");
    let derive = keys
        .generate(KeyUse::Derive, "", "g-derive", 0, &mut random)
        .expect("a key to derive from");
    for id in [&sign, &seal, &mac, &derive] {
        assert!(keys.retire(id).expect("retiring it"), "{id} was active");
        assert!(!keys.retire(id).expect("again"), "{id} is retired already");
        assert_eq!(
            keys.store().key(id).expect("its row").map(|row| row.state),
            Some(KeyState::Retired)
        );
    }
    for (id, makes, still) in [
        (&sign, Purpose::Sign, Purpose::Export),
        (&seal, Purpose::Seal, Purpose::Open),
        (&mac, Purpose::Tag, Purpose::Verify),
    ] {
        assert_eq!(keys.material(id, makes).err(), Some(KeyError::Retired));
        assert!(keys.material(id, still).is_ok(), "{still:?} under {id}");
    }
    assert_eq!(
        keys.material(&derive, Purpose::DeriveFrom).err(),
        Some(KeyError::Retired)
    );
    assert_eq!(keys.retire("generated:nobody"), Err(KeyError::Unknown));
}

#[test]
fn a_derivation_is_named_by_what_derived_it_in_every_process_that_shares_the_store() {
    let vectors = vectors();
    let store = MemoryKeys::new();
    let first = ring(store.clone(), 1);
    let box_key = hex(&vectors["sealing_keys"][0]["key"]);
    let mut random = scripted(&hex(&vectors["naming_key"]));
    let derived = vectors["derived"].as_array().expect("derivations");
    assert!(
        derived.len() >= 4,
        "a test that read no derivation would pass"
    );
    for case in derived {
        let name = text(&case["name"]);
        let secret = hex(&case["secret"]);
        let input_key = text(&case["input_key_id"]);
        let (input, material) = if input_key.is_empty() {
            (DerivationInput::Secret(&secret), secret.clone())
        } else {
            (DerivationInput::Key(input_key), box_key.clone())
        };
        let id = first
            .derive(
                input,
                &material,
                &hex(&case["salt"]),
                &hex(&case["info"]),
                0,
                &mut random,
            )
            .expect("a derivation");
        assert_eq!(id, text(&case["key_id"]), "{name}");
        assert_eq!(
            first
                .material(&id, Purpose::Open)
                .expect("the derived key")
                .as_slice(),
            hex(&case["key"]),
            "{name}: HKDF-SHA-256 of its input, salt and info"
        );
    }
    assert_eq!(random.queued(), 0, "the naming key was drawn once");

    // A second process on the same store, the same key-encryption key: the naming key is the
    // store's, so the same derivation is the same id, and nothing is drawn.
    let second = ring(store, 1);
    let case = &derived[0];
    let secret = hex(&case["secret"]);
    let mut none = scripted(&[]);
    let again = second
        .derive(
            DerivationInput::Secret(&secret),
            &secret,
            &hex(&case["salt"]),
            &hex(&case["info"]),
            0,
            &mut none,
        )
        .expect("the derivation in another process");
    assert_eq!(again, text(&case["key_id"]));
    assert_eq!(
        second.store().ids().expect("the ids").len(),
        derived.len(),
        "one key for one derivation"
    );
    assert!(second.material(&again, Purpose::Seal).is_ok());
    assert!(second.material(&again, Purpose::DeriveFrom).is_ok());
    assert_eq!(
        second.material(&again, Purpose::Tag).err(),
        Some(KeyError::Unknown),
        "a derived key does not tag"
    );
}

/// A directory of its own for a test's database, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static MADE: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sce-crypto-keyring-{name}-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }

    fn database(&self) -> PathBuf {
        self.0.join("keys.db")
    }

    /// Every byte of every file in the directory: the database, its log and its index.
    fn every_byte(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in std::fs::read_dir(&self.0).expect("the scratch directory") {
            let path = entry.expect("an entry").path();
            if path.is_file() {
                bytes.extend(std::fs::read(path).expect("a file"));
            }
        }
        bytes
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_material_is_unreadable_without_its_key_encryption_key_across_a_reopened_file() {
    let scratch = Scratch::new("reopen");
    let signing = generation("sign");
    let drawn = hex(&signing["random"]);
    let seed = drawn[16..].to_vec();
    let id = {
        let keys = ring(
            SqliteStore::open(scratch.database()).expect("making the store"),
            1,
        );
        keys.generate(KeyUse::Sign, "", "g", 0, &mut scripted(&drawn))
            .expect("a generation")
    };
    assert!(
        !scratch
            .every_byte()
            .windows(seed.len())
            .any(|window| window == seed.as_slice()),
        "the file holds the key in the clear"
    );

    let wrong = ring(SqliteStore::open(scratch.database()).expect("reopening"), 2);
    assert!(
        matches!(
            wrong.material(&id, Purpose::Sign),
            Err(KeyError::Unavailable(_))
        ),
        "the key opened under another key-encryption key"
    );
    let right = ring(SqliteStore::open(scratch.database()).expect("reopening"), 1);
    assert_eq!(
        right
            .material(&id, Purpose::Sign)
            .expect("the key")
            .as_slice(),
        seed.as_slice()
    );
    assert_eq!(
        right
            .generate(KeyUse::Sign, "", "g", 0, &mut scripted(&[]))
            .expect("the generation after the restart"),
        id,
        "a generation repeated after a restart answers its first key"
    );
}

#[test]
fn a_rotated_key_encryption_key_rewraps_every_key_and_the_naming_key() {
    let store = MemoryKeys::new();
    let mut keys = ring(store.clone(), 1);
    let mut random = ScriptedRandom::new();
    let sign = keys
        .generate(KeyUse::Sign, "", "g", 0, &mut random)
        .expect("a signing key");
    let derived = keys
        .derive(
            DerivationInput::Secret(b"s"),
            b"s",
            b"",
            b"",
            0,
            &mut random,
        )
        .expect("a derived key");
    let seed = keys
        .material(&sign, Purpose::Sign)
        .expect("the seed")
        .to_vec();
    let before = keys
        .material(&derived, Purpose::Open)
        .expect("the key")
        .to_vec();

    keys.rewrap_all(kek(2)).expect("the rotation");
    assert_eq!(
        keys.material(&sign, Purpose::Sign)
            .expect("after")
            .as_slice(),
        seed.as_slice()
    );
    let old = ring(store.clone(), 1);
    assert!(
        matches!(
            old.material(&sign, Purpose::Sign),
            Err(KeyError::Unavailable(_))
        ),
        "the old key-encryption key opens nothing"
    );
    let rotated = ring(store, 2);
    assert_eq!(
        rotated
            .material(&derived, Purpose::Open)
            .expect("the derived key")
            .as_slice(),
        before.as_slice()
    );
    assert_eq!(
        rotated
            .derive(
                DerivationInput::Secret(b"s"),
                b"s",
                b"",
                b"",
                0,
                &mut scripted(&[])
            )
            .expect("the same derivation"),
        derived,
        "the naming key was rewrapped too"
    );
    keys.rewrap_all(kek(2))
        .expect("a rotation run again with the same keys changes nothing");
}

#[test]
fn a_key_ring_in_memory_generates_with_the_systems_bytes() {
    let keys = Keyring::in_memory();
    let mut random = sce_rust_crypto::random::SystemRandom;
    let one = keys
        .generate(KeyUse::Mac, "", "a", 0, &mut random)
        .expect("a key");
    let two = keys
        .generate(KeyUse::Mac, "", "b", 0, &mut random)
        .expect("another");
    assert_ne!(one, two, "two generations draw two ids");
    assert!(one.starts_with("generated:") && one.len() == "generated:".len() + 32);
}

/// ADR 0109, D3: the issuing key is drawn once, at the store's first issue, kept as a MAC key of
/// the ring under its key-encryption key, and tags under it in every process that shares the
/// store; retired by the operator, it tags nothing more.
#[test]
fn the_issuing_key_is_drawn_once_and_kept_across_a_reopened_file() {
    let scratch = Scratch::new("issuing");
    let key = [0x3C; KEY_BYTES];
    let tag = {
        let keys = ring(
            SqliteStore::open(scratch.database()).expect("making the store"),
            1,
        );
        let mut random = scripted(&key);
        let tag = keys
            .issuing_tag(b"an identity", 7, &mut random)
            .expect("the first issue");
        assert_eq!(random.queued(), 0, "the issuing key's 32 bytes were drawn");
        assert_eq!(
            tag,
            hmac_sha256(&key, b"an identity"),
            "the tag is the HMAC-SHA-256 under the key drawn"
        );
        assert_eq!(
            keys.issuing_tag(b"an identity", 8, &mut scripted(&[0xEE; KEY_BYTES]))
                .expect("a second issue"),
            tag,
            "a second issue draws no second key"
        );
        assert_ne!(
            keys.issuing_tag(b"another identity", 8, &mut scripted(&[]))
                .expect("another message"),
            tag
        );
        let row = keys
            .store()
            .key(ISSUING_KEY_ID)
            .expect("reading the issuing key")
            .expect("the issuing key is a key of the ring");
        assert_eq!(
            (row.key_use, row.created_ms, row.state),
            (KeyUse::Mac, 7, KeyState::Active)
        );
        tag
    };
    assert!(
        !scratch
            .every_byte()
            .windows(KEY_BYTES)
            .any(|window| window == key.as_slice()),
        "the file holds the issuing key in the clear"
    );
    let reopened = ring(SqliteStore::open(scratch.database()).expect("reopening"), 1);
    assert_eq!(
        reopened
            .issuing_tag(b"an identity", 9, &mut scripted(&[0xEE; KEY_BYTES]))
            .expect("an issue after the restart"),
        tag,
        "the issuing key kept before the restart is the one after it"
    );
    reopened
        .store()
        .retire(ISSUING_KEY_ID)
        .expect("the operator retires the issuing key");
    assert_eq!(
        reopened.issuing_tag(b"an identity", 9, &mut scripted(&[])),
        Err(KeyError::Retired),
        "a retired issuing key issues nothing more"
    );
}
