// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The audit log through the reference host (docs/SCE_HOST_FEATURES.md,
// section 15; docs/adr/0109, D14): an entry redacted by both marks and
// written in its canonical form, the bytes of
// tests/host_features/audit_cross_language.json; an append repeated under one
// identity appends nothing; a stream the host keeps no policy for, a request
// the host refuses, and a stop that signs the head; and a stream on the SQLite
// store that a restart keeps verifiable.

use std::path::Path;
use std::sync::{Arc, Mutex};

use sce_rust_host_features::audit::{canonical_json, redact, PRINCIPAL_MEMBER, SECRET_MARKER};
use sce_rust_host_features::{
    AuditStreams, AuditVerdict, HttpRequest, HttpResponse, Invocation, KeyError, Outcome, Params,
    Personal, Reading, ReferenceHost, ScriptedClock, ScriptedRandom, StreamPolicy, Transport,
    TransportFailure,
};
use serde_json::Value;

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

const APPEND: &str = "sce-host-audit-append";

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/audit_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

fn hex(value: &Value) -> Vec<u8> {
    sce_rust_crypto::bytes_of_hex(value.as_str().expect("hexadecimal text")).expect("hexadecimal")
}

/// The streams the tests keep: `sign-ins`, checkpointed every two rows.
fn streams(personal: Personal) -> AuditStreams {
    [(
        "sign-ins".to_string(),
        StreamPolicy {
            checkpoint_every: 2,
            retain_days: 30,
            personal,
        },
    )]
    .into_iter()
    .collect()
}

/// A host whose clock reads `utc_ms`, whose random source gives the vectors'
/// pseudonymisation key and then their audit seed, and which keeps
/// `sign-ins`; the clock is shared, so a test moves it.
fn host(personal: Personal) -> (ReferenceHost<Unreachable>, Arc<Mutex<ScriptedClock>>) {
    let clock = Arc::new(Mutex::new(ScriptedClock::new()));
    let mut reference = ReferenceHost::new(Box::new(Arc::clone(&clock)), Unreachable);
    let vectors = vectors();
    let mut random = ScriptedRandom::new();
    let mut drawn = hex(&vectors["pseudonym_key"]);
    drawn.extend(hex(&vectors["seed"]));
    random.set(&drawn);
    reference.set_random_source(Box::new(random));
    reference.set_audit_streams(streams(personal));
    (reference, clock)
}

fn set_clock(clock: &Mutex<ScriptedClock>, utc_ms: u64) {
    clock.lock().expect("the clock").reading = Some(Reading {
        utc_ms: utc_ms as i64,
        monotonic_ms: 0,
    });
}

/// The request of an append of `received` to `stream` under the vectors' marks.
fn append(stream: &str, received: &str) -> Params {
    [
        ("stream", stream),
        ("entry", received),
        ("sensitivity", vectors()["marks"].as_str().expect("marks")),
    ]
    .iter()
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect()
}

fn done(outcome: Option<Outcome>) -> Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

#[test]
fn an_entry_is_redacted_by_both_marks_to_the_bytes_of_the_vectors() {
    let vectors = vectors();
    let key = hex(&vectors["pseudonym_key"]);
    let mut pseudonym = |text: &str| -> Result<String, KeyError> {
        Ok(sce_rust_crypto::hex(&sce_rust_crypto::mac::hmac_sha256(
            &key,
            text.as_bytes(),
        )))
    };
    for row in vectors["rows"].as_array().expect("rows") {
        let received = row["received"].as_str().expect("an entry received");
        assert_eq!(
            redact(
                received,
                vectors["marks"].as_str().expect("marks"),
                Personal::Pseudonymised,
                &mut pseudonym
            )
            .expect("a key")
            .expect("an entry the marks apply to"),
            row["entry"].as_str().expect("the entry written"),
        );
    }
    let clear = redact(
        r#"{"password":"p","account":"a@b"}"#,
        "/account=personal;/password=secret",
        Personal::Clear,
        &mut pseudonym,
    )
    .expect("a key")
    .expect("redacted");
    assert_eq!(
        clear,
        format!(r#"{{"account":"a@b","password":"{SECRET_MARKER}"}}"#),
        "a stream in the clear keeps the personal value and still never the secret"
    );
    let absent = redact(
        r#"{"account":null,"password":null}"#,
        "/account=personal;/password=secret",
        Personal::Pseudonymised,
        &mut pseudonym,
    )
    .expect("a key")
    .expect("redacted");
    assert_eq!(
        absent,
        format!(r#"{{"account":null,"password":"{SECRET_MARKER}"}}"#),
        "an absent personal value names no one; an absent secret is still marked"
    );
    for (entry, marks) in [
        ("[1]", ""),
        (r#"{"a":1}"#, "/b=secret"),
        (r#"{"a":1}"#, "/a=personal"),
        (r#"{"a":"x"}"#, "a=secret"),
        (r#"{"a":"x"}"#, "/a=hidden"),
    ] {
        assert_eq!(
            redact(entry, marks, Personal::Pseudonymised, &mut pseudonym).expect("a key"),
            None,
            "{entry} under `{marks}` is no entry the marks apply to"
        );
    }
}

#[test]
fn the_canonical_form_sorts_members_and_escapes_control_characters() {
    let value: Value =
        serde_json::from_str("{\"b\":[1,true,null],\"a\":{\"z\":\"x\\u0001\\n\\\"\",\"y\":2}}")
            .expect("JSON");
    assert_eq!(
        canonical_json(&value),
        "{\"a\":{\"y\":2,\"z\":\"x\\u0001\\n\\\"\"},\"b\":[1,true,null]}"
    );
}

#[test]
fn an_append_is_redacted_chained_and_answered_with_its_sequence() {
    let vectors = vectors();
    let (mut host, clock) = host(Personal::Pseudonymised);
    let rows = vectors["rows"].as_array().expect("rows");
    for (n, row) in rows.iter().enumerate() {
        set_clock(&clock, row["at_ms"].as_u64().expect("a moment"));
        let start = Invocation::new("session-1", "audit", n as u64 + 1);
        let answered = done(host.start(
            APPEND,
            &start,
            &append("sign-ins", row["received"].as_str().expect("an entry")),
        ));
        assert_eq!(answered["error"], "");
        assert_eq!(answered["sequence"], (n + 1).to_string());
    }
    let written = host
        .audit_store()
        .audit_rows("sign-ins", 0, 10)
        .expect("the rows");
    for (row, vector) in written.iter().zip(rows) {
        assert_eq!(
            row.entry,
            vector["entry"].as_str().expect("an entry").as_bytes()
        );
        assert_eq!(
            row.chain.to_vec(),
            hex(&vector["chain_hex"]),
            "the chain of row {}",
            row.sequence
        );
    }
    let checkpoints = host
        .audit_store()
        .audit_checkpoints("sign-ins")
        .expect("the checkpoints");
    assert_eq!(checkpoints.len(), 1, "a checkpoint every two rows");
    assert_eq!(checkpoints[0].sequence, 2);
    assert_eq!(
        host.audit_public_key().expect("the audit key").to_vec(),
        hex(&vectors["public_key"]),
        "the audit key is the second 32 bytes the host drew"
    );
    assert_eq!(
        host.audit_stop(1_767_225_603_000).expect("a stop"),
        [("sign-ins".to_string(), 3)],
        "the stop signs the head the periodic checkpoints did not"
    );
    let at_stop = host
        .audit_store()
        .audit_checkpoints("sign-ins")
        .expect("the checkpoints");
    assert_eq!(
        at_stop[1].signature,
        hex(&vectors["checkpoint"]["cose_sign1_hex"]),
        "the stop's checkpoint is the vectors' object"
    );
    assert_eq!(
        host.audit_verify("sign-ins").expect("a reading"),
        AuditVerdict::Verified {
            from: 1,
            through: 3,
            checkpoints: 2
        }
    );
}

/// docs/adr/0116, D7: an append made for a person is written with who it was made for, the
/// instance and the person, its subject as a personal value is written (a keyed digest in a stream
/// that pseudonymises one, as it is in one that does not); an append made for nobody names no one;
/// and an entry that names the member itself is refused, so a document neither writes who an
/// append was made for nor leaves it out. Red if the host writes the entry as the document gave
/// it, writes the subject in the clear in a stream that pseudonymises, or takes the member from
/// the document.
#[test]
fn an_append_made_for_a_person_is_written_with_who_it_was_made_for() {
    let alice = sce_rust_runtime::Principal {
        issuer: "https://id.example".to_string(),
        subject: "alice".to_string(),
        proved_ms: 1_700_000_000_000,
        method: "pwd".to_string(),
        instance: "a1".to_string(),
    };
    let entry = r#"{"account":"a","password":"p","result":"r","attempt":1}"#;
    for personal in [Personal::Pseudonymised, Personal::Clear] {
        let (mut host, clock) = host(personal);
        set_clock(&clock, 1_000);
        for (start, acting) in [(1, Some(&alice)), (2, None)] {
            let answered = done(host.start_for(
                APPEND,
                &Invocation::new("session-1", "audit", start),
                &append("sign-ins", entry),
                acting,
            ));
            assert_eq!(answered["error"], "");
        }
        let written: Vec<Value> = host
            .audit_store()
            .audit_rows("sign-ins", 0, 10)
            .expect("the rows")
            .iter()
            .map(|row| serde_json::from_slice(&row.entry).expect("an entry is JSON"))
            .collect();
        let named = &written[0][PRINCIPAL_MEMBER];
        assert_eq!(
            (
                &named["instance"],
                &named["issuer"],
                &named["method"],
                &named["proved_ms"]
            ),
            (
                &Value::from("a1"),
                &Value::from("https://id.example"),
                &Value::from("pwd"),
                &Value::from("1700000000000")
            ),
            "who the append was made for, under {personal:?}"
        );
        let subject = named["subject"].as_str().expect("a subject");
        match personal {
            Personal::Pseudonymised => assert!(
                subject.starts_with("sce-personal:") && !subject.contains("alice"),
                "a subject in the clear in a stream that pseudonymises: {subject}"
            ),
            Personal::Clear => assert_eq!(subject, "alice"),
        }
        assert_eq!(
            written[0]["account"], written[1]["account"],
            "the entry's own members are written as they are for nobody"
        );
        assert!(
            written[1].get(PRINCIPAL_MEMBER).is_none(),
            "an append made for nobody names someone: {}",
            written[1]
        );
    }
    let (mut host, clock) = host(Personal::Pseudonymised);
    set_clock(&clock, 1_000);
    let forged = r#"{"account":"a","password":"p","sce:principal":{"subject":"mallory"}}"#;
    for acting in [None, Some(&alice)] {
        assert!(
            matches!(
                host.start_for(
                    APPEND,
                    &Invocation::new("session-1", "forged", 1),
                    &append("sign-ins", forged),
                    acting,
                ),
                Some(Outcome::Refuse(_))
            ),
            "an entry that names who it was made for was taken"
        );
    }
}

#[test]
fn an_append_repeated_under_its_identity_appends_nothing() {
    let (mut host, clock) = host(Personal::Pseudonymised);
    set_clock(&clock, 1_000);
    let start = Invocation::new("session-1", "audit", 1);
    let entry = r#"{"account":"a","password":"p","result":"r","attempt":1}"#;
    let first = done(host.start(APPEND, &start, &append("sign-ins", entry)));
    let again = done(host.start(APPEND, &start, &append("sign-ins", entry)));
    assert_eq!(first, again, "the first sequence");
    assert_eq!(
        host.audit_store()
            .audit_rows("sign-ins", 0, 10)
            .expect("the rows")
            .len(),
        1,
        "nothing appended twice"
    );
    let next = done(host.start(
        APPEND,
        &Invocation::new("session-1", "audit", 2),
        &append("sign-ins", entry),
    ));
    assert_eq!(next["sequence"], "2", "another start is another entry");
}

/// docs/adr/0109, D16: each entry the host appends is reported, and a repeat
/// under one identity, which appends nothing, is not; an append to a stream
/// the host keeps no policy for is a failure of its own code. Red if the host
/// reports the repeat's answer as an append, or reports none.
#[test]
fn each_entry_appended_is_reported_and_a_repeat_is_not() {
    use sce_rust_metrics::recording::Recording;
    use sce_rust_metrics::{Counter, HostFailure, HostOperation, Metrics};

    let (mut host, clock) = host(Personal::Pseudonymised);
    let recording = Arc::new(Recording::new());
    host.set_metrics(Arc::clone(&recording) as Arc<dyn Metrics>);
    set_clock(&clock, 1_000);
    let start = Invocation::new("session-1", "audit", 1);
    let entry = r#"{"account":"a","password":"p","result":"r","attempt":1}"#;
    done(host.start(APPEND, &start, &append("sign-ins", entry)));
    done(host.start(APPEND, &start, &append("sign-ins", entry)));
    assert_eq!(recording.counted(Counter::AuditAppends), 1);
    done(host.start(
        APPEND,
        &Invocation::new("session-1", "audit", 2),
        &append("sign-ins", entry),
    ));
    assert_eq!(recording.counted(Counter::AuditAppends), 2);
    done(host.start(
        APPEND,
        &Invocation::new("session-1", "audit", 3),
        &append("elsewhere", entry),
    ));
    assert_eq!(recording.counted(Counter::AuditAppends), 2);
    assert_eq!(
        recording.counted(Counter::HostOperationFailures {
            operation: HostOperation::AuditAppend,
            code: HostFailure::UnknownStream,
        }),
        1
    );
}

#[test]
fn a_stream_the_host_keeps_no_policy_for_and_a_request_it_refuses_append_nothing() {
    let (mut host, clock) = host(Personal::Pseudonymised);
    set_clock(&clock, 1_000);
    let entry = r#"{"account":"a","password":"p","result":"r","attempt":1}"#;
    let unknown = done(host.start(
        APPEND,
        &Invocation::new("s", "audit", 1),
        &append("other", entry),
    ));
    assert_eq!(unknown["error"], "unknown_stream");
    assert_eq!(unknown["sequence"], "0", "every other field at its zero");
    for refused in [
        append("", entry),
        append(&"s".repeat(65), entry),
        append("sign-ins", "[]"),
        append("sign-ins", r#"{"account":"a"}"#),
    ] {
        assert!(
            matches!(
                host.start(APPEND, &Invocation::new("s", "audit", 2), &refused),
                Some(Outcome::Refuse(_))
            ),
            "{refused:?} is refused"
        );
    }
    assert!(host
        .audit_store()
        .audit_rows("sign-ins", 0, 10)
        .expect("the rows")
        .is_empty());
}

#[test]
fn a_stream_on_a_durable_store_is_verified_after_a_restart() {
    let dir = std::env::temp_dir().join(format!(
        "sce-host-features-audit-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let database = dir.join("host.db");
    // The first process: three appends and a stop, and then it is gone.
    let public_key = {
        let (mut host, clock) = host(Personal::Pseudonymised);
        host.set_audit_store(Arc::new(
            sce_rust_session_store::SqliteStore::open(&database).expect("the store"),
        ));
        for n in 1..=3u64 {
            set_clock(&clock, 1_000 * n);
            done(host.start(
                APPEND,
                &Invocation::new("s", "audit", n),
                &append("sign-ins", r#"{"account":"a","password":"p"}"#),
            ));
        }
        host.audit_stop(10_000).expect("a stop");
        host.audit_public_key().expect("the audit key")
    };
    let store = sce_rust_session_store::SqliteStore::open(&database).expect("the store again");
    assert_eq!(
        sce_rust_crypto::audit::verify(&store, "sign-ins", &public_key).expect("a reading"),
        AuditVerdict::Verified {
            from: 1,
            through: 3,
            checkpoints: 2
        },
        "the rows, the checkpoint at 2 and the stop's at 3 outlive the process"
    );
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}
