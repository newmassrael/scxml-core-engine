// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A sealing store (ADR 0109, D8; docs/SCE_SEALED_RECORD.md): its records are the bytes of
//! `tests/host_features/sealed_cross_language.json`, whose values libsodium computed apart from
//! this crate and which the C++ host is held to as well; every store contract it wraps keeps its
//! conformance kit under it, in memory and in the SQLite file; a row moved to another key, changed
//! by a byte or opened under another key is refused; the file holds none of what was saved; and a
//! re-seal pass moves every row to the active key, so a retired one may go.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sce_rust_crypto::bytes_of_hex;
use sce_rust_crypto::keyring::Keyring;
use sce_rust_crypto::random::ScriptedRandom;
use sce_rust_crypto::sealed::{
    keyed_digest, open_record, record_key_id, seal_record, Resealed, SealError, Sealed,
    SealingKeys, INDEX_KEY_ID, RECORD_MAGIC, SEALING_PREFIX,
};
use sce_rust_crypto::wrap::KeyEncryptionKey;
use sce_rust_session_store::audit_conformance::{self, AuditHarness};
use sce_rust_session_store::conformance::{self, Harness};
use sce_rust_session_store::credential_conformance::{self, CredentialHarness};
use sce_rust_session_store::journal_conformance::{self, JournalHarness};
use sce_rust_session_store::key_conformance::{self, KeyHarness};
use sce_rust_session_store::kv_conformance::{self, KvHarness};
use sce_rust_session_store::outbox_conformance;
use sce_rust_session_store::{
    Append, CredentialStore, KeyRow, KeyState, KeyStore, KeyUse, MemoryCredentials, MemoryKeys,
    MemoryStore, PinStore, SessionStore, SqliteStore, TokenRow, TokenStore, Write, LATEST_MOMENT,
};
use serde_json::Value;

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/sealed_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

fn hex(value: &Value) -> Vec<u8> {
    bytes_of_hex(value.as_str().expect("a hexadecimal text")).expect("hexadecimal")
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a text")
}

fn array<const N: usize>(value: &Value) -> [u8; N] {
    hex(value)
        .try_into()
        .expect("as many bytes as the field holds")
}

/// The index key of the vectors.
const INDEX: [u8; 32] = [
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
];
/// The first sealing key of the vectors and its id.
const ID_1: &str = "sealing:000102030405060708090a0b0c0d0e0f";
const KEY_1: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];
/// The second sealing key of the vectors and its id.
const ID_2: &str = "sealing:808182838485868788898a8b8c8d8e8f";
const KEY_2: [u8; 32] = [
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f,
    0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f,
];

/// The keys of a store sealed under `active`, holding `held`, its index key the vectors'.
fn keys_of(active: &str, held: &[(&str, [u8; 32])]) -> SealingKeys {
    SealingKeys::new(
        INDEX,
        active,
        held.iter().map(|(id, key)| (id.to_string(), *key)),
    )
    .expect("the active key is one of the keys held")
}

/// The keys every store of a kit is sealed under: the first key of the vectors.
fn first_keys() -> SealingKeys {
    keys_of(ID_1, &[(ID_1, KEY_1)])
}

#[test]
fn every_record_of_the_vectors_is_sealed_and_opened_to_its_bytes() {
    let records = vectors()["records"].as_array().expect("records").clone();
    assert_eq!(records.len(), 4);
    for vector in &records {
        let key: [u8; 32] = array(&vector["key"]);
        let nonce: [u8; 24] = array(&vector["nonce"]);
        let (store, indexed) = (text(&vector["store"]), hex(&vector["indexed"]));
        let record = seal_record(
            text(&vector["key_id"]),
            &key,
            &nonce,
            store,
            &indexed,
            &hex(&vector["plaintext"]),
        )
        .expect("a sealing");
        assert_eq!(record, hex(&vector["record"]), "{}", text(&vector["about"]));
        assert!(record.starts_with(RECORD_MAGIC));
        assert_eq!(record_key_id(&record), Ok(text(&vector["key_id"])));
        assert_eq!(
            open_record(&key, store, &indexed, &record)
                .expect("the record opens")
                .as_slice(),
            hex(&vector["plaintext"]).as_slice()
        );
    }
}

#[test]
fn a_record_opened_for_another_row_changed_or_under_another_key_does_not_open() {
    let vector = vectors()["records"][0].clone();
    let key: [u8; 32] = array(&vector["key"]);
    let record = hex(&vector["record"]);
    let indexed = hex(&vector["indexed"]);
    let store = text(&vector["store"]);
    let refused = |key: &[u8; 32], store: &str, indexed: &[u8], record: &[u8]| {
        open_record(key, store, indexed, record).err()
    };
    let mut other_entry = indexed.clone();
    other_entry[0] ^= 0x01;
    assert_eq!(
        refused(&key, store, other_entry.as_slice(), record.as_slice()),
        Some(SealError::Refused),
        "the record of another row"
    );
    assert_eq!(
        refused(
            &key,
            "credential_pins",
            indexed.as_slice(),
            record.as_slice()
        ),
        Some(SealError::Refused),
        "the record of another store"
    );
    for at in [RECORD_MAGIC.len() + 1, record.len() - 30, record.len() - 1] {
        let mut changed = record.clone();
        changed[at] ^= 0x01;
        assert!(
            refused(&key, store, indexed.as_slice(), changed.as_slice()).is_some(),
            "a byte changed at {at}"
        );
    }
    assert_eq!(
        refused(&KEY_2, store, indexed.as_slice(), record.as_slice()),
        Some(SealError::Refused),
        "another key"
    );
    assert_eq!(
        refused(
            &key,
            store,
            indexed.as_slice(),
            &record[..RECORD_MAGIC.len() + 20]
        ),
        Some(SealError::Malformed),
        "too short for a nonce and a tag"
    );
    assert_eq!(
        refused(&key, store, indexed.as_slice(), &b"not a record"[..]),
        Some(SealError::Malformed)
    );
    assert_eq!(
        seal_record("", &key, &[0; 24], store, &indexed, b"x"),
        Err(SealError::Unnameable),
        "a key with no id"
    );
}

#[test]
fn every_key_is_kept_under_its_keyed_digest() {
    let vectors = vectors();
    let sealed = Sealed::new(MemoryStore::new(), first_keys());
    let digests = vectors["digests"].as_array().expect("digests");
    assert_eq!(digests.len(), 4);
    for digest in digests {
        assert_eq!(
            sealed.digest_of(text(&digest["key"])),
            text(&digest["digest"]),
            "the entry of {:?}",
            digest["key"]
        );
    }
    assert_eq!(
        keyed_digest(&INDEX, &hex(&vectors["token_digest"])).to_vec(),
        hex(&vectors["token_entry"])
    );
    assert_eq!(
        sealed.indexed("r1"),
        sealed.digest_of("r1"),
        "a session is listed by its entry"
    );
}

/// A directory of its own for a test's database, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static MADE: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sce-crypto-sealed-{name}-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }

    fn database(&self) -> PathBuf {
        self.0.join("sealed.db")
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

/// Stores in memory, sealed under the first key; every handle shares the one store.
struct SealedInMemory {
    sessions: MemoryStore,
    credentials: MemoryCredentials,
    keys: MemoryKeys,
}

fn in_memory(_: &str) -> SealedInMemory {
    SealedInMemory {
        sessions: MemoryStore::new(),
        credentials: MemoryCredentials::new(),
        keys: MemoryKeys::new(),
    }
}

impl Harness for SealedInMemory {
    type Store = Sealed<MemoryStore>;

    fn handle(&self) -> Sealed<MemoryStore> {
        Sealed::new(self.sessions.clone(), first_keys())
    }
}

impl CredentialHarness for SealedInMemory {
    type Store = Sealed<MemoryCredentials>;

    fn handle(&self) -> Sealed<MemoryCredentials> {
        Sealed::new(self.credentials.clone(), first_keys())
    }
}

impl KeyHarness for SealedInMemory {
    type Store = Sealed<MemoryKeys>;

    fn handle(&self) -> Sealed<MemoryKeys> {
        Sealed::new(self.keys.clone(), first_keys())
    }
}

/// The values of the key-value feature are kept beside the sessions, so that a removal of a
/// session deletes its identities' first results: one store serves both kits.
impl KvHarness for SealedInMemory {
    type Store = Sealed<MemoryStore>;

    fn handle(&self) -> Sealed<MemoryStore> {
        Sealed::new(self.sessions.clone(), first_keys())
    }
}

/// A database file of its own, sealed under the first key; every handle opens a connection to it.
struct SealedOnSqlite(Scratch);

fn on_sqlite(name: &str) -> SealedOnSqlite {
    SealedOnSqlite(Scratch::new(name))
}

impl SealedOnSqlite {
    fn sealed(&self) -> Sealed<SqliteStore> {
        Sealed::new(
            SqliteStore::open(self.0.database()).expect("opening the store"),
            first_keys(),
        )
    }
}

impl Harness for SealedOnSqlite {
    type Store = Sealed<SqliteStore>;

    fn handle(&self) -> Sealed<SqliteStore> {
        self.sealed()
    }
}

impl CredentialHarness for SealedOnSqlite {
    type Store = Sealed<SqliteStore>;

    fn handle(&self) -> Sealed<SqliteStore> {
        self.sealed()
    }
}

impl KeyHarness for SealedOnSqlite {
    type Store = Sealed<SqliteStore>;

    fn handle(&self) -> Sealed<SqliteStore> {
        self.sealed()
    }
}

impl KvHarness for SealedOnSqlite {
    type Store = Sealed<SqliteStore>;

    fn handle(&self) -> Sealed<SqliteStore> {
        self.sealed()
    }
}

/// The audit log is kept beside the sessions, so that a removal of a session deletes its
/// identities: one store serves both kits.
impl AuditHarness for SealedInMemory {
    type Store = Sealed<MemoryStore>;

    fn handle(&self) -> Sealed<MemoryStore> {
        Sealed::new(self.sessions.clone(), first_keys())
    }
}

impl AuditHarness for SealedOnSqlite {
    type Store = Sealed<SqliteStore>;

    fn handle(&self) -> Sealed<SqliteStore> {
        self.sealed()
    }
}

/// The journal is appended to by the sessions' writes: one store serves both kits.
impl JournalHarness for SealedInMemory {
    type Store = Sealed<MemoryStore>;

    fn handle(&self) -> Sealed<MemoryStore> {
        Sealed::new(self.sessions.clone(), first_keys())
    }
}

impl JournalHarness for SealedOnSqlite {
    type Store = Sealed<SqliteStore>;

    fn handle(&self) -> Sealed<SqliteStore> {
        self.sealed()
    }
}

/// A test per function of a kit, each on a store of its own the harness `$made` makes.
macro_rules! kit {
    ($kit:ident, $made:ident, [$($case:ident),* $(,)?]) => {
        $(
            #[test]
            fn $case() {
                $kit::$case(&super::$made(stringify!($case)));
            }
        )*
    };
}

mod sessions_in_memory {
    use super::conformance;
    kit!(
        conformance,
        in_memory,
        [
            versions_are_compared_and_set,
            a_write_needs_the_current_fence,
            a_lease_runs_out_and_is_taken_over,
            due_sessions_are_listed_earliest_first,
            state_and_due_are_written_together,
            concurrent_writers_never_both_win,
            no_update_is_lost,
            handles_share_one_store,
            every_session_is_listed_by_its_entry_and_named_by_its_key,
            a_batch_is_decided_item_by_item,
            a_batch_sees_its_earlier_items_and_is_seen_whole_on_return,
        ]
    );
}

mod sessions_on_sqlite {
    use super::conformance;
    kit!(
        conformance,
        on_sqlite,
        [
            versions_are_compared_and_set,
            a_write_needs_the_current_fence,
            a_lease_runs_out_and_is_taken_over,
            due_sessions_are_listed_earliest_first,
            state_and_due_are_written_together,
            concurrent_writers_never_both_win,
            no_update_is_lost,
            handles_share_one_store,
            every_session_is_listed_by_its_entry_and_named_by_its_key,
            a_batch_is_decided_item_by_item,
            a_batch_sees_its_earlier_items_and_is_seen_whole_on_return,
        ]
    );
}

mod outbox_in_memory {
    use super::outbox_conformance;
    kit!(
        outbox_conformance,
        in_memory,
        [
            an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all,
            an_outbox_is_opened_once_and_numbers_on,
            an_acknowledgement_deletes_what_it_covers_in_its_incarnation,
            an_old_entry_expires_and_its_sender_is_noted,
            an_entry_and_a_note_are_replaced_only_from_what_they_hold,
            a_batch_appends_and_acknowledges_item_by_item,
        ]
    );
}

mod outbox_on_sqlite {
    use super::outbox_conformance;
    kit!(
        outbox_conformance,
        on_sqlite,
        [
            an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all,
            an_outbox_is_opened_once_and_numbers_on,
            an_acknowledgement_deletes_what_it_covers_in_its_incarnation,
            an_old_entry_expires_and_its_sender_is_noted,
            an_entry_and_a_note_are_replaced_only_from_what_they_hold,
            a_batch_appends_and_acknowledges_item_by_item,
        ]
    );
}

mod credentials_in_memory {
    use super::credential_conformance;
    kit!(
        credential_conformance,
        in_memory,
        [
            a_name_is_pinned_to_its_first_secret,
            an_unpinned_name_is_pinned_again,
            a_token_is_kept_once_under_its_digest,
            a_token_expires_at_its_moment,
            a_revoked_name_keeps_no_live_token_and_no_pin,
            handles_share_the_credentials,
            a_pin_is_replaced_only_from_the_bytes_it_holds,
            a_tokens_sealed_bytes_are_replaced_only_from_what_it_holds,
        ]
    );
}

mod credentials_on_sqlite {
    use super::credential_conformance;
    kit!(
        credential_conformance,
        on_sqlite,
        [
            a_name_is_pinned_to_its_first_secret,
            an_unpinned_name_is_pinned_again,
            a_token_is_kept_once_under_its_digest,
            a_token_expires_at_its_moment,
            a_revoked_name_keeps_no_live_token_and_no_pin,
            handles_share_the_credentials,
            a_pin_is_replaced_only_from_the_bytes_it_holds,
            a_tokens_sealed_bytes_are_replaced_only_from_what_it_holds,
        ]
    );
}

mod keys_in_memory {
    use super::key_conformance;
    kit!(
        key_conformance,
        in_memory,
        [
            a_key_is_kept_once_under_its_id,
            a_generation_makes_one_key,
            a_retired_key_stays_retired,
            a_key_is_rewrapped_in_place,
            the_first_naming_key_is_kept,
            handles_share_the_keys,
        ]
    );
}

mod keys_on_sqlite {
    use super::key_conformance;
    kit!(
        key_conformance,
        on_sqlite,
        [
            a_key_is_kept_once_under_its_id,
            a_generation_makes_one_key,
            a_retired_key_stays_retired,
            a_key_is_rewrapped_in_place,
            the_first_naming_key_is_kept,
            handles_share_the_keys,
        ]
    );
}

mod values_in_memory {
    use super::kv_conformance;
    kit!(
        kv_conformance,
        in_memory,
        [
            a_value_is_read_at_the_version_its_write_gave_it,
            a_put_if_writes_only_over_the_version_it_names,
            the_quota_is_judged_only_for_a_write_that_would_be_made,
            a_put_if_repeated_under_its_identity_answers_its_first_result,
            the_keys_are_walked_in_the_order_of_their_bytes,
            the_values_are_listed_a_page_at_a_time,
            a_value_is_replaced_only_from_the_bytes_it_holds,
            handles_share_the_values,
            a_removal_of_a_session_deletes_its_identities_first_results,
        ]
    );
}

mod values_on_sqlite {
    use super::kv_conformance;
    kit!(
        kv_conformance,
        on_sqlite,
        [
            a_value_is_read_at_the_version_its_write_gave_it,
            a_put_if_writes_only_over_the_version_it_names,
            the_quota_is_judged_only_for_a_write_that_would_be_made,
            a_put_if_repeated_under_its_identity_answers_its_first_result,
            the_keys_are_walked_in_the_order_of_their_bytes,
            the_values_are_listed_a_page_at_a_time,
            a_value_is_replaced_only_from_the_bytes_it_holds,
            handles_share_the_values,
            a_removal_of_a_session_deletes_its_identities_first_results,
        ]
    );
}

mod audit_in_memory {
    use super::audit_conformance;
    kit!(
        audit_conformance,
        in_memory,
        [
            rows_are_appended_one_past_the_head,
            a_checkpoint_is_kept_with_its_row_or_on_its_own,
            an_append_repeated_under_its_identity_answers_its_first_sequence,
            a_segment_is_deleted_whole_up_to_a_checkpoint,
            streams_are_kept_apart,
            handles_share_the_log,
            an_entry_and_a_signature_are_replaced_only_from_what_they_hold,
            a_removal_of_a_session_deletes_its_identities_and_no_row,
        ]
    );
}

mod audit_on_sqlite {
    use super::audit_conformance;
    kit!(
        audit_conformance,
        on_sqlite,
        [
            rows_are_appended_one_past_the_head,
            a_checkpoint_is_kept_with_its_row_or_on_its_own,
            an_append_repeated_under_its_identity_answers_its_first_sequence,
            a_segment_is_deleted_whole_up_to_a_checkpoint,
            streams_are_kept_apart,
            handles_share_the_log,
            an_entry_and_a_signature_are_replaced_only_from_what_they_hold,
            a_removal_of_a_session_deletes_its_identities_and_no_row,
        ]
    );
}

mod journal_in_memory {
    use super::journal_conformance;
    kit!(
        journal_conformance,
        in_memory,
        [
            an_entry_is_made_by_its_sessions_write_or_not_at_all,
            a_batch_appends_item_by_item,
            a_removed_session_keeps_its_journal,
            an_input_makes_one_entry_and_a_reader_misses_none,
            a_reader_waits_for_a_lower_number_still_open,
            a_page_is_resumed_where_it_ended,
            a_moment_is_never_below_the_one_before_it,
            an_answer_as_of_a_position_does_not_change,
            an_observed_entry_is_kept_once_for_its_origin,
            a_session_and_an_observed_stream_share_no_name,
            a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither,
            a_fact_is_kept_with_the_bytes_made_for_its_position,
            data_is_replaced_only_from_what_it_holds,
            handles_share_the_journal,
        ]
    );
}

mod journal_on_sqlite {
    use super::journal_conformance;
    kit!(
        journal_conformance,
        on_sqlite,
        [
            an_entry_is_made_by_its_sessions_write_or_not_at_all,
            a_batch_appends_item_by_item,
            a_removed_session_keeps_its_journal,
            an_input_makes_one_entry_and_a_reader_misses_none,
            a_reader_waits_for_a_lower_number_still_open,
            a_page_is_resumed_where_it_ended,
            a_moment_is_never_below_the_one_before_it,
            an_answer_as_of_a_position_does_not_change,
            an_observed_entry_is_kept_once_for_its_origin,
            a_session_and_an_observed_stream_share_no_name,
            a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither,
            a_fact_is_kept_with_the_bytes_made_for_its_position,
            data_is_replaced_only_from_what_it_holds,
            handles_share_the_journal,
        ]
    );
}

/// ADR 0114, D8: an entry is sealed for its stream and its position, its valid time inside; the
/// file holds nothing a session or another system said in the clear; and a re-seal pass moves every
/// entry to the active key and changes no answer.
mod journal_sealed {
    use sce_rust_crypto::sealed::{record_key_id, Resealed, Sealed};
    use sce_rust_session_store::{
        AsOf, AsOfCursor, Fact, FactData, Journal, JournalEntry, JournalStore, MemoryStore,
        SessionStore, Stream, StreamName, Write, LATEST_MOMENT,
    };

    use super::{first_keys, keys_of, on_sqlite, ID_1, ID_2, KEY_1, KEY_2};

    fn fact(data: &[u8], valid_ms: Option<i64>) -> Fact<'_> {
        Fact {
            schema: "account.opened",
            revision: 4,
            data: FactData::Bytes(data),
            valid_ms,
        }
    }

    /// A first write of the session `key` that appends `facts`.
    fn written(store: &impl SessionStore, key: &str, facts: &[Fact<'_>]) {
        let lease = store.acquire(key, "a", LATEST_MOMENT, 0).expect("a lease");
        store
            .write(
                key,
                &Write {
                    journal: Some(Journal {
                        machine: "ledger-of-alice",
                        facts,
                    }),
                    now_ms: 1_000,
                    ..Write::put(b"state", None, 0, lease.fence)
                },
            )
            .expect("a write that appends");
    }

    fn all(store: &impl JournalStore) -> Vec<JournalEntry> {
        store.after(0, 100).expect("the journal").entries
    }

    #[test]
    fn a_file_read_without_the_key_holds_none_of_the_facts() {
        let harness = on_sqlite("journal-medium");
        let store = harness.sealed();
        written(
            &store,
            "account-of-alice",
            &[fact(b"the balance of alice", Some(-5))],
        );
        store
            .observe(
                "bank-of-alice",
                "transfer-of-alice",
                &fact(b"the bank said", None),
                2_000,
            )
            .expect("an observation");
        let kept = store.inner().after(0, 10).expect("the wrapped journal");
        assert_eq!(kept.entries.len(), 2);
        assert_eq!(
            kept.entries[0].stream,
            StreamName::Session {
                machine: store.digest_of("ledger-of-alice"),
                key: store.digest_of("account-of-alice"),
            },
            "a stream is kept under its entries"
        );
        assert_eq!(
            kept.entries[1].origin,
            Some(store.digest_of("transfer-of-alice"))
        );
        for entry in &kept.entries {
            assert_eq!(
                (entry.schema.as_str(), entry.revision, entry.valid_ms),
                ("", 0, None),
                "a schema, a revision and a valid time are inside the record"
            );
        }
        assert_eq!(
            (kept.entries[0].position, kept.entries[0].recorded_ms),
            (1, 1_000),
            "the position and the recorded moment are the wrapped store's"
        );
        drop(store);
        let medium = harness.0.every_byte();
        for clear in [
            &b"ledger-of-alice"[..],
            b"account-of-alice",
            b"the balance of alice",
            b"account.opened",
            b"bank-of-alice",
            b"transfer-of-alice",
            b"the bank said",
        ] {
            assert!(
                !medium.windows(clear.len()).any(|window| window == clear),
                "{} is in the clear on the medium",
                String::from_utf8_lossy(clear)
            );
        }
        let entries = all(&harness.sealed());
        assert_eq!(entries[0].data, b"the balance of alice".to_vec());
        assert_eq!(
            (entries[0].schema.as_str(), entries[0].revision),
            ("account.opened", 4)
        );
        assert_eq!(entries[0].valid_ms, Some(-5));
        assert_eq!(entries[1].origin.as_deref(), Some("transfer-of-alice"));
    }

    #[test]
    fn an_entry_moved_to_another_position_or_stream_does_not_open() {
        let store = Sealed::new(MemoryStore::new(), first_keys());
        written(&store, "k", &[fact(b"one", None), fact(b"two", None)]);
        let kept = store.inner().after(0, 10).expect("the wrapped journal");
        let (one, two) = (&kept.entries[0], &kept.entries[1]);
        assert!(store
            .inner()
            .reseal_journaled(two.position, &two.data, &one.data)
            .expect("the record copied over another position of its stream"));
        assert!(
            store.after(one.position, 1).is_err(),
            "a record moved to another position is refused"
        );
        assert!(
            store.after(0, 1).is_ok(),
            "the entry it came from still opens"
        );

        // The same position of another store's other stream.
        let other = Sealed::new(MemoryStore::new(), first_keys());
        written(&other, "o", &[fact(b"one", None)]);
        let held = all(other.inner()).remove(0);
        assert_eq!(held.position, one.position);
        assert!(other
            .inner()
            .reseal_journaled(held.position, &held.data, &one.data)
            .expect("the record copied to another stream at its position"));
        assert!(
            other.after(0, 1).is_err(),
            "a record moved to another stream is refused"
        );
    }

    #[test]
    fn a_reseal_pass_moves_every_entry_to_the_active_key_and_changes_no_answer() {
        let harness = on_sqlite("journal-reseal");
        let first = harness.sealed();
        written(
            &first,
            "k",
            &[fact(b"late", None), fact(b"early", Some(-7))],
        );
        first
            .observe("bank", "tx-1", &fact(b"observed", None), 2_000)
            .expect("an observation");
        let before = all(&first);
        let asked = AsOf {
            position: 3,
            valid_ms: i64::MAX,
        };
        let stream = Stream::Session {
            machine: "ledger-of-alice",
            key: "k",
        };
        let answer = |store: &Sealed<sce_rust_session_store::SqliteStore>| {
            store
                .as_of(stream, &asked, AsOfCursor::START, 10)
                .expect("an answer")
                .entries
                .into_iter()
                .map(|entry| entry.data)
                .collect::<Vec<_>>()
        };
        let given = answer(&first);
        assert_eq!(given, vec![b"early".to_vec(), b"late".to_vec()]);
        let stale = first
            .inner()
            .after(0, 1)
            .expect("the wrapped journal")
            .entries
            .remove(0);

        first.set_keys(keys_of(ID_2, &[(ID_1, KEY_1), (ID_2, KEY_2)]));
        assert_eq!(
            first.reseal_journal().expect("a pass"),
            Resealed {
                resealed: 3,
                left: 0,
                refused: 0,
            }
        );
        for kept in all(first.inner()) {
            assert_eq!(record_key_id(&kept.data).expect("a record"), ID_2);
        }
        assert!(
            !first
                .inner()
                .reseal_journaled(stale.position, &stale.data, &stale.data)
                .expect("a replacement from the bytes before the pass"),
            "a change made in between is not overwritten"
        );
        assert_eq!(
            first.reseal_journal().expect("a second pass"),
            Resealed::default()
        );
        first.set_keys(keys_of(ID_2, &[(ID_2, KEY_2)]));
        assert_eq!(all(&first), before, "the new key alone opens every entry");
        assert_eq!(answer(&first), given, "a pass changes no answer");
    }
}

/// ADR 0109, D14 under D8: a sealed audit log is verified through the sealing store as it is
/// without one, and its file holds neither an entry nor a stream's name in the clear.
mod audit_sealed {
    use sce_rust_crypto::audit::{verify, Verdict, Writer};
    use sce_rust_crypto::keyring::{Keyring, AUDIT_SIGNING_KEY_ID};
    use sce_rust_crypto::random::ScriptedRandom;
    use sce_rust_session_store::AuditStore;

    use super::on_sqlite;

    #[test]
    fn a_sealed_stream_is_verified_and_holds_nothing_in_the_clear() {
        let harness = on_sqlite("audit-medium");
        let store = harness.sealed();
        let keyring = Keyring::in_memory();
        let mut random = ScriptedRandom::new();
        for (n, entry) in ["{\"who\":\"alice\"}", "{\"who\":\"bob\"}"]
            .iter()
            .enumerate()
        {
            let mut writer = Writer {
                store: &store,
                keyring: &keyring,
                random: &mut random,
                now_ms: 1_000 + n as u64,
            };
            writer
                .append("sign-ins-of-a-tenant", entry.as_bytes(), 2, None)
                .expect("an append");
        }
        let public_key = keyring
            .public_key(AUDIT_SIGNING_KEY_ID)
            .expect("the audit key");
        assert_eq!(
            verify(&store, "sign-ins-of-a-tenant", &public_key).expect("a reading"),
            Verdict::Verified {
                from: 1,
                through: 2,
                checkpoints: 1
            }
        );
        assert_eq!(
            store
                .audit_rows("sign-ins-of-a-tenant", 0, 10)
                .expect("the rows")[1]
                .entry,
            b"{\"who\":\"bob\"}".to_vec(),
            "an entry opens to what was appended"
        );
        drop(store);
        let medium = harness.0.every_byte();
        for clear in [&b"alice"[..], b"bob", b"sign-ins-of-a-tenant"] {
            assert!(
                !medium.windows(clear.len()).any(|window| window == clear),
                "{} is in the clear on the medium",
                String::from_utf8_lossy(clear)
            );
        }
    }
}

/// ADR 0109, D13 under D8: the file of a sealed store holds none of the keys and values a host
/// wrote, nor the session an identity names; a value moved to another key does not open; and a
/// re-seal pass moves every value to the active key, its version kept.
mod values_sealed {
    use sce_rust_crypto::sealed::{record_key_id, Sealed};
    use sce_rust_session_store::{Identity, KvStore, PutIf, SqliteStore};

    use super::{keys_of, on_sqlite, ID_1, ID_2, KEY_1, KEY_2};

    #[test]
    fn a_file_read_without_the_key_holds_none_of_the_values() {
        let harness = on_sqlite("kv-medium");
        let store = harness.sealed();
        let start = Identity {
            session: "session-of-alice",
            invoke: "claim",
            start: 1,
        };
        store
            .put("alice@example.org", b"the value of alice", None)
            .expect("a value");
        assert_eq!(
            store
                .put_if("ticket", b"seat 14", Some(0), None, Some(&start))
                .expect("a put-if under an identity"),
            PutIf::Written { version: 2 }
        );
        drop(store);
        let medium = harness.0.every_byte();
        for clear in [
            &b"alice@example.org"[..],
            b"the value of alice",
            b"seat 14",
            b"session-of-alice",
        ] {
            assert!(
                !medium.windows(clear.len()).any(|window| window == clear),
                "{} is in the clear on the medium",
                String::from_utf8_lossy(clear)
            );
        }
        let store = harness.sealed();
        assert_eq!(
            store
                .get("alice@example.org")
                .expect("read back")
                .map(|entry| entry.value),
            Some(b"the value of alice".to_vec())
        );
        assert_eq!(
            store.first_result(&start).expect("its first result"),
            Some(PutIf::Written { version: 2 })
        );
        assert_eq!(
            store.next_key("", "").expect("a walk"),
            Some("alice@example.org".to_string()),
            "the walk is by the keys, not by their entries"
        );
    }

    #[test]
    fn a_value_moved_to_another_key_does_not_open() {
        let harness = on_sqlite("kv-moved");
        let store = harness.sealed();
        store.put("a", b"of a", None).expect("a value");
        store.put("b", b"of b", None).expect("another");
        let inner: &SqliteStore = store.inner();
        let of_a = inner
            .get(&store.digest_of("a"))
            .expect("the wrapped row")
            .expect("held");
        assert!(inner
            .reseal(
                &store.digest_of("b"),
                &inner
                    .get(&store.digest_of("b"))
                    .expect("the wrapped row")
                    .expect("held")
                    .value,
                &of_a.value
            )
            .expect("the record copied over another row"));
        assert!(
            store.get("b").is_err(),
            "a record moved to another key is refused"
        );
        assert!(store.get("a").is_ok(), "the row it came from still opens");
    }

    #[test]
    fn a_reseal_pass_moves_every_value_to_the_active_key() {
        let harness = on_sqlite("kv-reseal");
        let first = harness.sealed();
        first.put("a", b"1", None).expect("a value");
        first.put("b", b"2", None).expect("another");
        drop(first);
        let rotated = Sealed::new(
            SqliteStore::open(harness.0.database()).expect("the store"),
            keys_of(ID_2, &[(ID_1, KEY_1), (ID_2, KEY_2)]),
        );
        let resealed = rotated.reseal_values().expect("a pass");
        assert_eq!(
            (resealed.resealed, resealed.left, resealed.refused),
            (2, 0, 0)
        );
        for key in ["a", "b"] {
            let row = rotated
                .inner()
                .get(&rotated.digest_of(key))
                .expect("the wrapped row")
                .expect("held");
            assert_eq!(record_key_id(&row.value).expect("a record"), ID_2);
        }
        assert_eq!(
            rotated
                .get("a")
                .expect("read back")
                .map(|entry| entry.version),
            Some(1),
            "a pass keeps the version"
        );
        assert_eq!(
            rotated.reseal_values().expect("a second pass").resealed,
            0,
            "nothing is left under the retired key"
        );
    }
}

/// A session written under a lease the writer gives back: its version.
fn saved(store: &impl SessionStore, key: &str, state: &[u8]) -> u64 {
    let lease = store
        .acquire(key, "writer", LATEST_MOMENT, 0)
        .expect("a lease");
    let version = store
        .put(key, state, Some(5_000), 0, lease.fence)
        .expect("a session written");
    store
        .release(key, lease.fence)
        .expect("the lease given back");
    version
}

/// A token of `digest` for `name`.
fn token(digest: [u8; 32], name: &str) -> TokenRow {
    TokenRow {
        digest,
        name: name.to_string(),
        issued_ms: 1_000,
        expires_ms: 61_000,
        revoked: false,
        sealed: Vec::new(),
    }
}

/// A key of `id`, labelled `label`, wrapped as `wrapped`.
fn key_row(id: &str, label: &str, wrapped: &[u8]) -> KeyRow {
    KeyRow {
        id: id.to_string(),
        key_use: KeyUse::Seal,
        algorithm: "ChaCha20-Poly1305".to_string(),
        wrapped: wrapped.to_vec(),
        label: label.to_string(),
        created_ms: 1_000,
        state: KeyState::Active,
    }
}

/// ADR 0109, D8: a row's record moved to another key's row, a record changed by a byte, and a
/// record read under another key of its id, or under keys that lack its id, are each refused; a
/// pin, a token and a key moved to another row are refused as a session is.
#[test]
fn a_row_moved_to_another_key_changed_or_read_under_another_key_is_refused() {
    let inner = MemoryStore::new();
    let sealed = Sealed::new(inner.clone(), first_keys());
    saved(&sealed, "alice", b"the state of alice");
    saved(&sealed, "bob", b"the state of bob");
    let (alice, bob) = (sealed.digest_of("alice"), sealed.digest_of("bob"));
    let record_of = |entry: &str| inner.get(entry).expect("read").expect("held");

    let lease = inner
        .acquire(&bob, "medium", LATEST_MOMENT, 0)
        .expect("a lease");
    let moved = record_of(&alice).state;
    inner
        .put(&bob, &moved, None, record_of(&bob).version, lease.fence)
        .expect("alice's record written as bob's row");
    let refused = sealed
        .get("bob")
        .expect_err("a record moved to another key");
    assert!(refused.to_string().contains("does not open"), "{refused}");
    assert!(sealed.key_of(&bob).is_err(), "nor is its key read from it");

    let mut changed = record_of(&alice).state;
    let last = changed.len() - 1;
    changed[last] ^= 0x01;
    let lease = inner
        .acquire(&alice, "medium", LATEST_MOMENT, 0)
        .expect("a lease");
    let version = record_of(&alice).version;
    inner
        .put(&alice, &changed, None, version, lease.fence)
        .expect("a byte of alice's record changed");
    assert!(sealed.get("alice").is_err(), "a changed byte");

    saved(&sealed, "carol", b"the state of carol");
    let other_key = Sealed::new(inner.clone(), keys_of(ID_1, &[(ID_1, KEY_2)]));
    assert!(other_key.get("carol").is_err(), "another key of its id");
    let lacking = Sealed::new(inner.clone(), keys_of(ID_2, &[(ID_2, KEY_2)]));
    let unknown = lacking.get("carol").expect_err("keys that lack its id");
    assert!(
        unknown.to_string().contains(ID_1),
        "the key it names is told: {unknown}"
    );
    assert_eq!(
        sealed
            .get("carol")
            .expect("carol")
            .map(|stored| stored.state),
        Some(b"the state of carol".to_vec()),
        "the right key opens it"
    );

    let pins = MemoryCredentials::new();
    let sealed_pins = Sealed::new(pins.clone(), first_keys());
    sealed_pins.pin("phone", &[0xA1; 32]).expect("phone's pin");
    let phone_pin = pins
        .pinned(&sealed_pins.digest_of("phone"))
        .expect("read")
        .expect("held");
    pins.pin(&sealed_pins.digest_of("tablet"), &phone_pin)
        .expect("phone's pin written as tablet's");
    assert!(
        sealed_pins.pinned("tablet").is_err(),
        "a pin moved to another name"
    );
    sealed_pins
        .issue(&token([0x11; 32], "phone"))
        .expect("phone's token");
    let held = pins.tokens().expect("the tokens").remove(0);
    pins.issue(&TokenRow {
        digest: keyed_digest(&INDEX, &[0x22; 32]),
        ..held
    })
    .expect("phone's token row written as another token's");
    assert!(
        sealed_pins.token(&[0x22; 32]).is_err(),
        "a token moved to another digest"
    );

    let keys = MemoryKeys::new();
    let sealed_keys = Sealed::new(keys.clone(), first_keys());
    sealed_keys
        .insert(&key_row("generated:a", "of a", b"material of a"), None)
        .expect("a key");
    let row = keys
        .key(&sealed_keys.digest_of("generated:a"))
        .expect("read")
        .expect("held");
    keys.insert(
        &KeyRow {
            id: sealed_keys.digest_of("generated:b"),
            ..row
        },
        None,
    )
    .expect("a's row written as b's");
    assert!(
        sealed_keys.key("generated:b").is_err(),
        "a key moved to another id"
    );
}

/// ADR 0109, D8: a file the sealing store wrote, read without its keys, holds none of the values
/// saved in it, no session's key, no name a credential proves, no key's id, label or material;
/// what it holds is entries and records.
#[test]
fn a_file_read_without_the_key_holds_none_of_the_saved_values() {
    let scratch = Scratch::new("at-rest");
    let secrets: [&[u8]; 9] = [
        b"alice@example.org",
        b"the balance of alice is 1234",
        b"the pin of alice's phone 0123456",
        b"phone-of-alice",
        b"a token digest of alice 01234567",
        b"generated:alice",
        b"the key alice generated",
        b"the wrapped material of alice",
        b"the naming key of the store",
    ];
    let entry = {
        let store = Sealed::new(
            SqliteStore::open(scratch.database()).expect("making the store"),
            first_keys(),
        );
        let lease = store
            .acquire("alice@example.org", "server", LATEST_MOMENT, 0)
            .expect("a lease");
        store
            .put(
                "alice@example.org",
                b"the balance of alice is 1234",
                Some(9_000),
                0,
                lease.fence,
            )
            .expect("alice's session");
        store
            .pin("phone-of-alice", b"the pin of alice's phone 0123456")
            .expect("a pin");
        store
            .issue(&token(
                *b"a token digest of alice 01234567",
                "alice@example.org",
            ))
            .expect("a token");
        store
            .insert(
                &key_row(
                    "generated:alice",
                    "the key alice generated",
                    b"the wrapped material of alice",
                ),
                Some("alice@example.org/generate/1"),
            )
            .expect("a key");
        store
            .keep_naming_key(b"the naming key of the store")
            .expect("a naming key");
        assert_eq!(
            store
                .get("alice@example.org")
                .expect("read back")
                .map(|stored| stored.state),
            Some(b"the balance of alice is 1234".to_vec()),
            "the store reads back what it sealed"
        );
        store.digest_of("alice@example.org")
    };
    let bytes = scratch.every_byte();
    let holds = |needle: &[u8]| bytes.windows(needle.len()).any(|window| window == needle);
    assert!(
        holds(entry.as_bytes()),
        "the file holds the session's entry: the read reached the rows"
    );
    assert!(holds(RECORD_MAGIC), "the file holds records");
    for secret in secrets {
        assert!(
            !holds(secret),
            "the file holds {:?} in the clear",
            String::from_utf8_lossy(secret)
        );
    }
}

/// ADR 0109, D10 under D8: an envelope a session's write appends to an outbox is a record sealed
/// for its client's entry, with the client's name and the sending session's key inside it; moved
/// to another client's outbox it does not open, and the file holds neither the envelope, nor the
/// client's name, nor the session's key, in the clear. An expiry's note is opened back to the
/// client's name and the envelope.
#[test]
fn an_outbox_envelope_is_sealed_for_its_client_and_nothing_of_it_is_in_the_clear() {
    let scratch = Scratch::new("sealed-outbox");
    let envelope: &[u8] = b"an envelope only bob's phone may read";
    let incarnation = "00112233445566778899aabbccddeeff";
    {
        let store = Sealed::new(
            SqliteStore::open(scratch.database()).expect("making the store"),
            first_keys(),
        );
        let lease = store
            .acquire("session-of-bob", "server", LATEST_MOMENT, 0)
            .expect("a lease");
        let written = store
            .write(
                "session-of-bob",
                &Write {
                    outbox: &[Append {
                        client: "phone-of-bob",
                        incarnation,
                        sender: Some("session-of-bob"),
                        envelope,
                    }],
                    now_ms: 1_000,
                    ..Write::put(b"the state of bob's session", None, 0, lease.fence)
                },
            )
            .expect("a write that appends");
        assert_eq!(written.appended[0].client, "phone-of-bob");
        let outbox = store
            .outbox("phone-of-bob")
            .expect("the outbox")
            .expect("an outbox held");
        assert_eq!(outbox.entries[0].envelope, envelope.to_vec());
        assert_eq!(
            outbox.entries[0].sender.as_deref(),
            Some("session-of-bob"),
            "the sender is read back by its key"
        );

        let held = store
            .inner()
            .outbox(&store.digest_of("phone-of-bob"))
            .expect("the wrapped store's outbox")
            .expect("held under the client's entry");
        assert_ne!(held.entries[0].envelope, envelope.to_vec());
        assert_eq!(
            held.entries[0].sender,
            Some(store.digest_of("session-of-bob")),
            "the sender is kept as its entry"
        );
        store
            .inner()
            .append(
                &[Append {
                    client: &store.digest_of("tablet-of-eve"),
                    incarnation,
                    sender: None,
                    envelope: &held.entries[0].envelope,
                }],
                2_000,
            )
            .expect("the record moved to another client's outbox");
        assert!(
            store.outbox("tablet-of-eve").is_err(),
            "a record moved to another client's outbox does not open"
        );

        let expiry = store
            .expire("phone-of-bob", 1_500, 10)
            .expect("an expiry of bob's envelope");
        assert_eq!(expiry.expired[0].client, "phone-of-bob");
        assert_eq!(expiry.expired[0].entry.envelope, envelope.to_vec());
        let notes = store.notes("session-of-bob").expect("the session's notes");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].client, "phone-of-bob");
        assert_eq!(notes[0].envelope, envelope.to_vec());
    }
    let bytes = scratch.every_byte();
    let holds = |needle: &[u8]| bytes.windows(needle.len()).any(|window| window == needle);
    assert!(holds(RECORD_MAGIC), "the file holds records");
    for secret in [
        envelope,
        b"phone-of-bob".as_slice(),
        b"session-of-bob".as_slice(),
    ] {
        assert!(
            !holds(secret),
            "the file holds {:?} in the clear",
            String::from_utf8_lossy(secret)
        );
    }
}

/// ADR 0109, D8: the keys a sealing store seals and indexes under are keys of the key ring kept
/// in its key store, wrapped under the key-encryption key: drawn once, the same at every later
/// read; a rotation makes a new active key and retires the one before, which still opens.
#[test]
fn the_keys_are_kept_in_the_key_ring_and_a_rotation_retires_the_one_before() {
    let ring = Keyring::new(
        Box::new(MemoryKeys::new()),
        Box::new(KeyEncryptionKey::new([7; 32])),
    );
    let mut random = ScriptedRandom::new();
    let first = SealingKeys::from_keyring(&ring, 1_000, &mut random).expect("the first keys");
    assert!(first.active().starts_with(SEALING_PREFIX), "{first:?}");
    assert_eq!(first.ids().count(), 1);
    let again = SealingKeys::from_keyring(&ring, 2_000, &mut random).expect("the keys again");
    assert_eq!(again.active(), first.active(), "a second read draws none");
    let index = ring
        .store()
        .key(INDEX_KEY_ID)
        .expect("read")
        .expect("the index key is kept");
    assert_eq!(index.key_use, KeyUse::Mac);
    assert!(
        !format!("{first:?}").contains("0x"),
        "the keys are never printed"
    );

    let rotated = SealingKeys::rotate(&ring, 3_000, &mut random).expect("a rotation");
    assert_ne!(rotated, first.active());
    let keys = SealingKeys::from_keyring(&ring, 4_000, &mut random).expect("the keys after it");
    assert_eq!(keys.active(), rotated, "the new key seals");
    assert_eq!(keys.ids().count(), 2, "the retired one still opens");
    assert_eq!(
        ring.store()
            .key(first.active())
            .expect("read")
            .map(|row| row.state),
        Some(KeyState::Retired)
    );

    let before = Sealed::new(MemoryStore::new(), first.clone());
    saved(&before, "k", b"sealed before the rotation");
    let after = Sealed::new(before.inner().clone(), keys);
    assert_eq!(
        after.digest_of("k"),
        before.digest_of("k"),
        "a rotation keeps the index key"
    );
    assert_eq!(
        after.get("k").expect("read").map(|stored| stored.state),
        Some(b"sealed before the rotation".to_vec()),
        "a row sealed under the retired key opens"
    );
}

/// ADR 0109, D8: once the active key changes, a re-seal pass writes every row sealed under the
/// key before again under it, each through the store's own write: a session under a lease the
/// pass takes and gives back, from the version it read, and one another process holds is left for
/// a later pass; pins, tokens, keys and the naming key alike. Then a store given the new key alone
/// reads everything.
#[test]
fn a_reseal_pass_moves_every_row_to_the_active_key() {
    let (sessions, credentials, key_rows) = (
        MemoryStore::new(),
        MemoryCredentials::new(),
        MemoryKeys::new(),
    );
    let one = first_keys();
    let s = Sealed::new(sessions.clone(), one.clone());
    let c = Sealed::new(credentials.clone(), one.clone());
    let k = Sealed::new(key_rows.clone(), one);
    saved(&s, "a", b"the state of a");
    saved(&s, "b", b"the state of b");
    c.pin("phone", &[0xA1; 32]).expect("a pin");
    c.issue(&token([0x11; 32], "phone")).expect("a token");
    k.insert(&key_row("generated:a", "of a", b"material of a"), None)
        .expect("a key");
    k.keep_naming_key(b"the naming key").expect("a naming key");

    let both = keys_of(ID_2, &[(ID_1, KEY_1), (ID_2, KEY_2)]);
    s.set_keys(both.clone());
    c.set_keys(both.clone());
    k.set_keys(both);
    let held = s
        .acquire("b", "another process", LATEST_MOMENT, 0)
        .expect("b held elsewhere");
    let sealed_under = |key: &str| {
        let record = sessions
            .get(&s.digest_of(key))
            .expect("read")
            .expect("held")
            .state;
        record_key_id(&record).expect("a record").to_string()
    };
    assert_eq!(
        s.reseal_sessions("resealer", 1_000, 10)
            .expect("a re-seal pass"),
        Resealed {
            resealed: 1,
            left: 1,
            refused: 0,
        },
        "the session another process holds is left"
    );
    assert_eq!(sealed_under("a"), ID_2);
    assert_eq!(sealed_under("b"), ID_1);
    s.acquire("a", "server", 1_000, 20)
        .expect("the pass gave back the lease it took");
    s.release("b", held.fence).expect("b given back");
    assert_eq!(
        s.reseal_sessions("resealer", 1_000, 30)
            .expect("a second pass"),
        Resealed {
            resealed: 1,
            left: 0,
            refused: 0,
        }
    );
    assert_eq!(sealed_under("b"), ID_2);
    assert_eq!(
        s.reseal_sessions("resealer", 1_000, 40)
            .expect("a third pass"),
        Resealed::default(),
        "nothing is left under the key before"
    );
    assert_eq!(
        c.reseal_credentials().expect("the credentials' pass"),
        Resealed {
            resealed: 2,
            left: 0,
            refused: 0,
        },
        "the pin and the token"
    );
    assert_eq!(
        k.reseal_keys().expect("the keys' pass"),
        Resealed {
            resealed: 2,
            left: 0,
            refused: 0,
        },
        "the key and the naming key"
    );

    let only = keys_of(ID_2, &[(ID_2, KEY_2)]);
    let s = Sealed::new(sessions, only.clone());
    let c = Sealed::new(credentials, only.clone());
    let k = Sealed::new(key_rows, only);
    for (key, state) in [("a", &b"the state of a"[..]), ("b", &b"the state of b"[..])] {
        let stored = s.get(key).expect("read").expect("held");
        assert_eq!(stored.state, state);
        assert_eq!(stored.earliest_due, Some(5_000), "its due is kept");
    }
    assert_eq!(
        c.pinned("phone").expect("the pin"),
        Some([0xA1; 32].to_vec())
    );
    assert_eq!(
        c.token(&[0x11; 32]).expect("the token"),
        Some(token([0x11; 32], "phone"))
    );
    assert_eq!(
        k.key("generated:a").expect("the key"),
        Some(key_row("generated:a", "of a", b"material of a"))
    );
    assert_eq!(
        k.naming_key().expect("the naming key"),
        Some(b"the naming key".to_vec())
    );
    assert_eq!(
        c.revoke("phone").expect("revoking phone").count(),
        2,
        "the revocation finds the token and the pin by the name's entry"
    );
}

/// ADR 0109, D8: the outboxes and the audit log are re-sealed as every other row is: each
/// envelope of an outbox and of an expiry's note, each audit row's entry and each checkpoint's
/// signature is written again under the active key, by a replacement from the bytes the pass
/// read, its number, its moment, its sender and its chain kept, so the stream still verifies. A
/// replacement made from the bytes a pass already replaced writes nothing, and a store given the
/// new key alone reads everything.
#[test]
fn a_reseal_pass_moves_every_envelope_and_audit_record_to_the_active_key() {
    use sce_rust_crypto::audit::{verify, Verdict, Writer};
    use sce_rust_crypto::keyring::AUDIT_SIGNING_KEY_ID;
    use sce_rust_session_store::AuditStore;

    let store = MemoryStore::new();
    let s = Sealed::new(store.clone(), first_keys());
    saved(&s, "k", b"the sending session");
    let incarnation = "00112233445566778899aabbccddeeff";
    let append =
        |client: &'static str, sender: Option<&'static str>, envelope: &'static [u8]| Append {
            client,
            incarnation,
            sender,
            envelope,
        };
    s.append(
        &[
            append("phone", Some("k"), &b"one"[..]),
            append("phone", Some("k"), &b"two"[..]),
            append("tablet", None, &b"three"[..]),
        ],
        1_000,
    )
    .expect("three appends");
    s.expire("phone", 1_500, 1)
        .expect("an expiry of the first, which writes a note for its sender");
    let keyring = Keyring::in_memory();
    let mut random = ScriptedRandom::new();
    for (n, entry) in [&b"{\"n\":1}"[..], &b"{\"n\":2}"[..]].iter().enumerate() {
        let mut writer = Writer {
            store: &s,
            keyring: &keyring,
            random: &mut random,
            now_ms: 2_000 + n as u64,
        };
        writer
            .append("log", entry, 2, None)
            .expect("an append, the second with its checkpoint");
    }
    let log = s.digest_of("log");
    let rows_before = store.audit_rows(&log, 0, 10).expect("the wrapped rows");
    let stale_envelope = store
        .outbox(&s.digest_of("phone"))
        .expect("read")
        .expect("held")
        .entries[0]
        .envelope
        .clone();

    s.set_keys(keys_of(ID_2, &[(ID_1, KEY_1), (ID_2, KEY_2)]));
    assert_eq!(
        s.reseal_outboxes().expect("the outboxes' pass"),
        Resealed {
            resealed: 3,
            left: 0,
            refused: 0,
        },
        "the entry of each client and the note"
    );
    assert_eq!(
        s.reseal_audit().expect("the audit log's pass"),
        Resealed {
            resealed: 3,
            left: 0,
            refused: 0,
        },
        "the two rows and the checkpoint"
    );

    for client in ["phone", "tablet"] {
        let outbox = store
            .outbox(&s.digest_of(client))
            .expect("read")
            .expect("held");
        for entry in &outbox.entries {
            assert_eq!(record_key_id(&entry.envelope).expect("a record"), ID_2);
        }
    }
    let notes = store.notes(&s.digest_of("k")).expect("the wrapped notes");
    assert_eq!(notes.len(), 1);
    assert_eq!(record_key_id(&notes[0].envelope).expect("a record"), ID_2);
    let rows = store.audit_rows(&log, 0, 10).expect("the wrapped rows");
    for row in &rows {
        assert_eq!(record_key_id(&row.entry).expect("a record"), ID_2);
    }
    assert_eq!(
        rows.iter()
            .map(|row| (row.sequence, row.at_ms, row.chain))
            .collect::<Vec<_>>(),
        rows_before
            .iter()
            .map(|row| (row.sequence, row.at_ms, row.chain))
            .collect::<Vec<_>>(),
        "the sequences, the moments and the chain are kept"
    );
    let checkpoints = store
        .audit_checkpoints(&log)
        .expect("the wrapped checkpoints");
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(
        record_key_id(&checkpoints[0].signature).expect("a record"),
        ID_2
    );

    assert!(
        !store
            .reseal_entry(&s.digest_of("phone"), 2, &stale_envelope, &stale_envelope)
            .expect("a replacement from the bytes before the pass"),
        "a change made in between is not overwritten"
    );
    assert!(
        !store
            .audit_reseal_row(&log, 1, &rows_before[0].entry, &rows_before[0].entry)
            .expect("a replacement from the bytes before the pass"),
        "a change made in between is not overwritten"
    );
    assert_eq!(
        s.reseal_outboxes().expect("a second pass"),
        Resealed::default()
    );
    assert_eq!(
        s.reseal_audit().expect("a second pass"),
        Resealed::default()
    );

    let only = Sealed::new(store, keys_of(ID_2, &[(ID_2, KEY_2)]));
    assert_eq!(
        only.outbox("phone")
            .expect("read")
            .expect("held")
            .entries
            .iter()
            .map(|entry| (
                entry.number,
                entry.at_ms,
                entry.sender.as_deref(),
                entry.envelope.clone()
            ))
            .collect::<Vec<_>>(),
        vec![(2, 1_000, Some("k"), b"two".to_vec())]
    );
    assert_eq!(
        only.outbox("tablet").expect("read").expect("held").entries[0].envelope,
        b"three".to_vec()
    );
    assert_eq!(
        only.notes("k").expect("the notes")[0].envelope,
        b"one".to_vec()
    );
    let public_key = keyring
        .public_key(AUDIT_SIGNING_KEY_ID)
        .expect("the audit key");
    assert_eq!(
        verify(&only, "log", &public_key).expect("a reading"),
        Verdict::Verified {
            from: 1,
            through: 2,
            checkpoints: 1
        },
        "the stream verifies under the new key alone"
    );
}
