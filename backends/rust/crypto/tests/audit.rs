// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The audit log's chain and checkpoints (ADR 0109, D14; docs/SCE_AUDIT_LOG.md): the bytes of
//! `tests/host_features/audit_cross_language.json`, computed apart from this crate, which the C++
//! host is held to as well; a stream appended and verified; a changed entry, a truncation and a
//! forged checkpoint each told; and retention that leaves the rest verifiable.

use std::path::Path;
use std::sync::Mutex;

use sce_rust_crypto::audit::{
    chained, checkpoint_payload, checkpoint_verifies, entry_item, retain, retention_cut,
    sign_checkpoint, verify, Verdict, Writer,
};
use sce_rust_crypto::bytes_of_hex;
use sce_rust_crypto::cose::public_key_of;
use sce_rust_crypto::keyring::{Keyring, AUDIT_PSEUDONYM_KEY_ID, AUDIT_SIGNING_KEY_ID};
use sce_rust_crypto::random::ScriptedRandom;
use sce_rust_session_store::audit::GENESIS;
use sce_rust_session_store::{
    AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore, Failure, Identity, MemoryStore,
};
use serde_json::Value;

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/audit_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

fn hex(value: &Value) -> Vec<u8> {
    bytes_of_hex(value.as_str().expect("a hexadecimal text")).expect("hexadecimal")
}

fn array<const N: usize>(value: &Value) -> [u8; N] {
    hex(value)
        .try_into()
        .expect("as many bytes as the field holds")
}

#[test]
fn every_row_and_the_checkpoint_of_the_vectors_are_their_bytes() {
    let vectors = vectors();
    let stream = vectors["stream"].as_str().expect("a stream");
    let seed: [u8; 32] = array(&vectors["seed"]);
    let public_key: [u8; 32] = array(&vectors["public_key"]);
    assert_eq!(public_key_of(&seed), public_key);
    let mut chain = GENESIS;
    let rows = vectors["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 3);
    for row in rows {
        let sequence = row["sequence"].as_u64().expect("a sequence");
        let at_ms = row["at_ms"].as_u64().expect("a moment");
        let entry = row["entry"].as_str().expect("an entry").as_bytes();
        assert_eq!(
            entry_item(stream, sequence, at_ms, entry),
            hex(&row["item_hex"]),
            "the item of row {sequence}"
        );
        chain = chained(&chain, stream, sequence, at_ms, entry);
        assert_eq!(
            chain.to_vec(),
            hex(&row["chain_hex"]),
            "the chain of row {sequence}"
        );
    }
    let checkpoint = &vectors["checkpoint"];
    assert_eq!(
        checkpoint_payload(stream, 3, &chain),
        hex(&checkpoint["payload_hex"])
    );
    let signed = sign_checkpoint(&seed, stream, 3, &chain);
    assert_eq!(
        signed,
        hex(&checkpoint["cose_sign1_hex"]),
        "an Ed25519 signature is deterministic, so the object is its bytes"
    );
    let kept = AuditCheckpoint {
        stream: stream.to_string(),
        sequence: 3,
        chain,
        signature: signed,
    };
    assert!(checkpoint_verifies(&public_key, &kept));
    assert!(
        !checkpoint_verifies(&public_key_of(&[7; 32]), &kept),
        "another key's public key verifies nothing of it"
    );
    let moved = AuditCheckpoint {
        sequence: 2,
        ..kept
    };
    assert!(
        !checkpoint_verifies(&public_key, &moved),
        "a signature moved to another sequence does not verify"
    );
}

/// A key ring in memory whose audit key is the vectors' seed: the first 32 bytes the random source
/// gives are drawn for it.
fn keyring_and_random() -> (Keyring, ScriptedRandom) {
    let mut random = ScriptedRandom::new();
    random.set(&hex(&vectors()["seed"]));
    (Keyring::in_memory(), random)
}

/// Append `entries` to `stream` of `store` at one second apart from `from_ms`, a checkpoint every
/// `every` rows.
fn append_all(
    store: &dyn AuditStore,
    keyring: &Keyring,
    random: &mut ScriptedRandom,
    stream: &str,
    entries: &[&str],
    from_ms: u64,
    every: u64,
) {
    for (n, entry) in entries.iter().enumerate() {
        let mut writer = Writer {
            store,
            keyring,
            random: &mut *random,
            now_ms: from_ms + 1_000 * n as u64,
        };
        let sequence = writer
            .append(stream, entry.as_bytes(), every, None)
            .expect("an append");
        assert_eq!(sequence, n as u64 + 1);
    }
}

#[test]
fn a_stream_appended_is_verified_from_its_first_row() {
    let store = MemoryStore::new();
    let (keyring, mut random) = keyring_and_random();
    append_all(
        &store,
        &keyring,
        &mut random,
        "s",
        &["{\"n\":1}", "{\"n\":2}", "{\"n\":3}"],
        1_000,
        2,
    );
    let public_key = keyring
        .public_key(AUDIT_SIGNING_KEY_ID)
        .expect("the audit key");
    assert_eq!(public_key, array::<32>(&vectors()["public_key"]));
    assert_eq!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Verified {
            from: 1,
            through: 3,
            checkpoints: 1
        },
        "the checkpoint at 2, every two rows"
    );
    let mut writer = Writer {
        store: &store,
        keyring: &keyring,
        random: &mut random,
        now_ms: 9_000,
    };
    assert_eq!(writer.checkpoint_head("s").expect("a stop"), Some(3));
    assert_eq!(
        writer.checkpoint_head("s").expect("a second stop"),
        None,
        "the head is signed already"
    );
    assert_eq!(
        writer.checkpoint_head("empty").expect("a stream of none"),
        None
    );
    assert_eq!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Verified {
            from: 1,
            through: 3,
            checkpoints: 2
        }
    );
    assert!(
        keyring
            .store()
            .key(AUDIT_PSEUDONYM_KEY_ID)
            .expect("a read")
            .is_none(),
        "no pseudonym was asked for, so no pseudonymisation key was drawn"
    );
}

#[test]
fn an_append_repeated_under_its_identity_appends_nothing() {
    let store = MemoryStore::new();
    let (keyring, mut random) = keyring_and_random();
    let start = Identity {
        session: "session",
        invoke: "audit",
        start: 1,
    };
    let mut writer = Writer {
        store: &store,
        keyring: &keyring,
        random: &mut random,
        now_ms: 1_000,
    };
    assert_eq!(
        writer.append("s", b"{}", 1, Some(&start)).expect("first"),
        1
    );
    assert_eq!(
        writer
            .append("s", b"{}", 1, Some(&start))
            .expect("repeated"),
        1,
        "the first sequence"
    );
    assert_eq!(store.audit_head("s").expect("the head").sequence, 1);
    assert_eq!(store.audit_checkpoints("s").expect("one").len(), 1);
}

/// A store whose rows are read back changed, as a medium written by hand gives them: `change`
/// replaces the entry of one row, and `cut_after` hides every row past a sequence.
struct Tampered {
    inner: MemoryStore,
    change: Mutex<Option<u64>>,
    cut_after: Mutex<Option<u64>>,
}

impl AuditStore for Tampered {
    fn audit_head(&self, stream: &str) -> Result<AuditHead, Failure> {
        self.inner.audit_head(stream)
    }

    fn audit_append(
        &self,
        row: &AuditRow,
        checkpoint: Option<&[u8]>,
        identity: Option<&Identity<'_>>,
    ) -> Result<AuditAppend, Failure> {
        self.inner.audit_append(row, checkpoint, identity)
    }

    fn audit_checkpoint(&self, checkpoint: &AuditCheckpoint) -> Result<bool, Failure> {
        self.inner.audit_checkpoint(checkpoint)
    }

    fn audit_rows(&self, stream: &str, after: u64, limit: usize) -> Result<Vec<AuditRow>, Failure> {
        let change = *self.change.lock().expect("the tamper");
        let cut_after = *self.cut_after.lock().expect("the tamper");
        Ok(self
            .inner
            .audit_rows(stream, after, limit)?
            .into_iter()
            .filter(|row| cut_after.is_none_or(|cut| row.sequence <= cut))
            .map(|mut row| {
                if change == Some(row.sequence) {
                    row.entry = b"{\"n\":\"changed\"}".to_vec();
                }
                row
            })
            .collect())
    }

    fn audit_checkpoints(&self, stream: &str) -> Result<Vec<AuditCheckpoint>, Failure> {
        self.inner.audit_checkpoints(stream)
    }

    fn audit_drop_through(&self, stream: &str, through: u64) -> Result<u64, Failure> {
        self.inner.audit_drop_through(stream, through)
    }

    fn audit_first(&self, identity: &Identity<'_>) -> Result<Option<u64>, Failure> {
        self.inner.audit_first(identity)
    }
}

/// A stream of six rows, checkpointed every three, signed at its stop.
fn six_rows() -> (Tampered, [u8; 32]) {
    let store = Tampered {
        inner: MemoryStore::new(),
        change: Mutex::new(None),
        cut_after: Mutex::new(None),
    };
    let (keyring, mut random) = keyring_and_random();
    let entries = [
        "{\"n\":1}",
        "{\"n\":2}",
        "{\"n\":3}",
        "{\"n\":4}",
        "{\"n\":5}",
        "{\"n\":6}",
    ];
    append_all(&store, &keyring, &mut random, "s", &entries, 1_000, 4);
    let mut writer = Writer {
        store: &store,
        keyring: &keyring,
        random: &mut random,
        now_ms: 10_000,
    };
    assert_eq!(writer.checkpoint_head("s").expect("a stop"), Some(6));
    let public_key = keyring
        .public_key(AUDIT_SIGNING_KEY_ID)
        .expect("the audit key");
    (store, public_key)
}

#[test]
fn a_changed_entry_is_told_at_its_row() {
    let (store, public_key) = six_rows();
    assert!(matches!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Verified { through: 6, .. }
    ));
    *store.change.lock().expect("the tamper") = Some(2);
    assert_eq!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Changed { sequence: 2 }
    );
}

#[test]
fn a_truncation_is_told_by_the_checkpoint_past_the_last_row() {
    let (store, public_key) = six_rows();
    *store.cut_after.lock().expect("the tamper") = Some(5);
    assert_eq!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Truncated {
            sequence: 6,
            last: 5
        },
        "the stop's checkpoint at 6 holds a row the store no longer gives"
    );
}

#[test]
fn a_checkpoint_under_another_key_is_told_as_forged() {
    let (store, _) = six_rows();
    assert_eq!(
        verify(&store, "s", &public_key_of(&[9; 32])).expect("a reading"),
        Verdict::Forged { sequence: 4 },
        "the first checkpoint, at 4, does not verify under another key"
    );
}

#[test]
fn retention_deletes_whole_segments_and_keeps_the_rest_verifiable() {
    let store = MemoryStore::new();
    let (keyring, mut random) = keyring_and_random();
    let entries: Vec<String> = (1..=7).map(|n| format!("{{\"n\":{n}}}")).collect();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    // Rows at 1 000, 2 000, ... 7 000 ms; checkpoints at 2, 4 and 6.
    append_all(&store, &keyring, &mut random, "s", &entries, 1_000, 2);
    let public_key = keyring
        .public_key(AUDIT_SIGNING_KEY_ID)
        .expect("the audit key");
    assert_eq!(
        retention_cut(&store, "s", 500).expect("a cut"),
        None,
        "no row is old enough"
    );
    // Rows 1 to 5 are older than 5 500 ms; the latest checkpoint before row 6 is at 4.
    assert_eq!(retention_cut(&store, "s", 5_500).expect("a cut"), Some(4));
    assert_eq!(retain(&store, "s", 5_500).expect("a retention"), 4);
    assert_eq!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Verified {
            from: 5,
            through: 7,
            checkpoints: 2
        },
        "the checkpoint at the cut anchors what remains"
    );
    let mut writer = Writer {
        store: &store,
        keyring: &keyring,
        random: &mut random,
        now_ms: 8_000,
    };
    assert_eq!(writer.append("s", b"{\"n\":8}", 2, None).expect("on"), 8);
    assert!(matches!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Verified {
            from: 5,
            through: 8,
            ..
        }
    ));
    // Every row older: the cut is the latest checkpoint, and the rows past it stay.
    assert_eq!(retain(&store, "s", 100_000).expect("a retention"), 4);
    assert!(matches!(
        verify(&store, "s", &public_key).expect("a reading"),
        Verdict::Verified {
            from: 9,
            through: 8,
            ..
        }
    ));
    assert!(
        store.audit_rows("s", 0, 10).expect("the rows").is_empty(),
        "rows 5 to 8 went with the segment that ends at 8"
    );
}

#[test]
fn an_anchor_that_is_no_signature_is_told_as_forged() {
    let store = MemoryStore::new();
    // Rows 1 and 2 of a stream, of which the store lost row 1 and holds no checkpoint.
    let chain1 = chained(&GENESIS, "s", 1, 1, b"{}");
    let chain2 = chained(&chain1, "s", 2, 2, b"{}");
    for (sequence, chain) in [(1, chain1), (2, chain2)] {
        store
            .audit_append(
                &AuditRow {
                    stream: "s".to_string(),
                    sequence,
                    at_ms: sequence,
                    entry: b"{}".to_vec(),
                    chain,
                },
                (sequence == 1).then_some(&b"not a signature"[..]),
                None,
            )
            .expect("an append");
    }
    assert!(
        store.audit_drop_through("s", 1).is_ok(),
        "a store keeps any bytes as a checkpoint"
    );
    assert_eq!(
        verify(&store, "s", &public_key_of(&[1; 32])).expect("a reading"),
        Verdict::Forged { sequence: 1 },
        "the anchor that is left is no signature"
    );
}
