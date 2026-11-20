// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A record signed and sealed by name (sce:payload, docs/adr/0109, D9), which the
// Rust and the C++ lanes agree on byte for byte:
// `tests/host_features/payload_record_cross_language.json`, which the C++ lane is
// held to by tests/integration/APayloadRecordTest.cpp. The runtime's record
// codec writes the record's deterministic CBOR and reads it back; this host signs
// and seals those bytes to the file's objects; and a completion of a verification
// or an opening is read as the record, as a machine reads it, or as the
// operation's failure `not_a_record` for bytes that are no note.

use std::path::Path;

use sce_rust_host_features::cose::{public_key_of, IV_BYTES};
use sce_rust_host_features::{
    CoseKeys, HttpRequest, HttpResponse, Invocation, NonceSource, Outcome, Params, ReferenceHost,
    ScriptedClock, Transport, TransportFailure, TrustedKey,
};
use sce_rust_runtime::record_cbor;
use serde_json::Value;

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

/// A random source that gives the nonce the file fixes.
struct Fixed([u8; IV_BYTES]);

impl NonceSource for Fixed {
    fn next_nonce(&mut self) -> Option<[u8; IV_BYTES]> {
        Some(self.0)
    }
}

/// The trust set of the file's signing key.
const TRUST_SET: &str = "devices";

/// The completion of a verification the build writes for the failure
/// `not_a_record`: every other field at its zero, the record absent.
const VERIFY_FAILURE: &str = r#"{"error":"not_a_record","key_id":"","payload":null,"valid":false}"#;

/// The same, of an opening.
const OPEN_FAILURE: &str = r#"{"error":"not_a_record","opened":false,"plaintext":null}"#;

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/payload_record_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

fn hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "{text} is not hex");
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn key(text: &str) -> [u8; 32] {
    hex(text).try_into().expect("32 bytes")
}

/// The member `name` of `value`, a text.
fn member<'v>(value: &'v Value, name: &str) -> &'v str {
    value[name]
        .as_str()
        .unwrap_or_else(|| panic!("{name} is a text"))
}

/// The text a byte string travels as, one character to a byte.
fn text(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

fn params(pairs: &[(&str, String)]) -> Params {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect()
}

/// The result record of a completion, as the text a host completes with.
fn done(outcome: Option<Outcome>) -> String {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => data,
        other => panic!("the host did not complete: {other:?}"),
    }
}

/// A host that holds the file's signing key and sealing key.
fn signer(vectors: &Value) -> ReferenceHost<Unreachable> {
    let signing = &vectors["signing_key"];
    let sealing = &vectors["sealing_key"];
    let mut keys = CoseKeys::default();
    keys.signing.insert(
        member(signing, "id").to_string(),
        key(member(signing, "seed")),
    );
    keys.symmetric.insert(
        member(sealing, "id").to_string(),
        key(member(sealing, "key")),
    );
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    host.set_cose_keys(keys).expect("no reserved id");
    host
}

/// A host that holds no key but the signer's public key, in the trust set
/// [`TRUST_SET`]: what a client verifies its server's objects with.
fn verifier(vectors: &Value) -> ReferenceHost<Unreachable> {
    let signing = &vectors["signing_key"];
    let public_key = key(member(signing, "public_key"));
    assert_eq!(
        public_key_of(&key(member(signing, "seed"))),
        public_key,
        "the file's public key is the one its seed gives"
    );
    let mut keys = CoseKeys::default();
    keys.trust.insert(
        TRUST_SET.to_string(),
        vec![TrustedKey {
            id: member(signing, "id").to_string(),
            public_key,
        }],
    );
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    host.set_cose_keys(keys).expect("no reserved id");
    host
}

/// The completion of a verification of `object` by `host`, read as the note.
fn verified(host: &mut ReferenceHost<Unreachable>, vectors: &Value, object: &[u8]) -> Value {
    let data = done(host.start(
        "sce-host-cose-verify1",
        &Invocation::new("session", "verify_note", 1),
        &params(&[
            ("trust_set", TRUST_SET.to_string()),
            ("cose", text(object)),
            ("external_aad", String::new()),
        ]),
    ));
    let read = record_cbor::open_completion(
        &data,
        "payload",
        "valid",
        member(vectors, "description"),
        VERIFY_FAILURE,
    );
    serde_json::from_str(&read).expect("a completion is JSON")
}

/// The completion of an opening of `object` by `host`, read as the note.
fn opened(host: &mut ReferenceHost<Unreachable>, vectors: &Value, object: &[u8]) -> Value {
    let data = done(host.start(
        "sce-host-cose-decrypt0",
        &Invocation::new("session", "open_note", 1),
        &params(&[
            ("key_id", member(&vectors["sealing_key"], "id").to_string()),
            ("cose", text(object)),
            ("external_aad", String::new()),
        ]),
    ));
    let read = record_cbor::open_completion(
        &data,
        "plaintext",
        "opened",
        member(vectors, "description"),
        OPEN_FAILURE,
    );
    serde_json::from_str(&read).expect("a completion is JSON")
}

#[test]
fn the_record_codec_writes_the_files_bytes_and_reads_them_back() {
    let vectors = vectors();
    let description = member(&vectors, "description");
    let written = record_cbor::encode(&vectors["record"].to_string(), description)
        .expect("the file's record is a note");
    assert_eq!(written, hex(member(&vectors, "record_cbor")));
    let read = record_cbor::decode(&written, description).expect("the bytes of a note");
    let read: Value = serde_json::from_str(&read).expect("a record is JSON");
    assert_eq!(read, vectors["record"]);
    assert!(
        record_cbor::decode(&hex(member(&vectors["not_a_record"], "bytes")), description).is_err(),
        "bytes that are no note are refused"
    );
}

#[test]
fn the_host_signs_and_seals_the_records_bytes_byte_for_byte() {
    let vectors = vectors();
    let mut host = signer(&vectors);
    let record = text(&hex(member(&vectors, "record_cbor")));
    let signed: Value = serde_json::from_str(&done(host.start(
        "sce-host-cose-sign1",
        &Invocation::new("session", "sign_note", 1),
        &params(&[
            ("key_id", member(&vectors["signing_key"], "id").to_string()),
            ("payload", record.clone()),
            ("external_aad", String::new()),
        ]),
    )))
    .expect("a result is JSON");
    assert_eq!(signed["cose"], text(&hex(member(&vectors, "signed"))));
    let nonce: [u8; IV_BYTES] = hex(member(&vectors, "nonce"))
        .try_into()
        .expect("a nonce is 12 bytes");
    host.set_nonce_source(Box::new(Fixed(nonce)));
    let sealed: Value = serde_json::from_str(&done(host.start(
        "sce-host-cose-encrypt0",
        &Invocation::new("session", "seal_note", 1),
        &params(&[
            ("key_id", member(&vectors["sealing_key"], "id").to_string()),
            ("plaintext", record),
            ("external_aad", String::new()),
        ]),
    )))
    .expect("a result is JSON");
    assert_eq!(sealed["cose"], text(&hex(member(&vectors, "sealed"))));
}

#[test]
fn a_host_that_holds_only_the_public_key_verifies_the_record() {
    let vectors = vectors();
    let mut host = verifier(&vectors);
    let read = verified(&mut host, &vectors, &hex(member(&vectors, "signed")));
    assert_eq!(read["error"], "");
    assert_eq!(read["valid"], true);
    assert_eq!(read["key_id"], member(&vectors["signing_key"], "id"));
    assert_eq!(
        read["payload"], vectors["record"],
        "the machine receives the note"
    );
}

#[test]
fn a_tampered_object_does_not_verify_and_carries_no_record() {
    let vectors = vectors();
    let mut host = verifier(&vectors);
    let read = verified(&mut host, &vectors, &hex(member(&vectors, "tampered")));
    assert_eq!(
        read["error"], "",
        "a signature that does not verify is an answer"
    );
    assert_eq!(read["valid"], false);
    assert_eq!(read["payload"], Value::Null);
}

#[test]
fn bytes_that_verify_or_open_and_are_no_record_are_the_failure_not_a_record() {
    let vectors = vectors();
    let mut host = verifier(&vectors);
    let read = verified(
        &mut host,
        &vectors,
        &hex(member(&vectors["not_a_record"], "signed")),
    );
    assert_eq!(
        read,
        serde_json::from_str::<Value>(VERIFY_FAILURE).expect("JSON")
    );
    let mut host = signer(&vectors);
    let read = opened(
        &mut host,
        &vectors,
        &hex(member(&vectors["not_a_record"], "sealed")),
    );
    assert_eq!(
        read,
        serde_json::from_str::<Value>(OPEN_FAILURE).expect("JSON")
    );
}

#[test]
fn the_sealed_record_opens_as_the_record() {
    let vectors = vectors();
    let mut host = signer(&vectors);
    let read = opened(&mut host, &vectors, &hex(member(&vectors, "sealed")));
    assert_eq!(read["error"], "");
    assert_eq!(read["opened"], true);
    assert_eq!(read["plaintext"], vectors["record"]);
}
