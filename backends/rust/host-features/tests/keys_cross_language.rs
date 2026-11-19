// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the Rust and the C++ reference hosts agree on for keyed digests and keys,
// byte for byte (docs/SCE_HOST_FEATURES.md, sections 12 and 13; docs/adr/0109,
// D6 and D7): `tests/host_features/keys_cross_language.json`, which the C++ host
// is held to by `AStandardHostKeys.KeysAndTagsCrossBetweenTheHosts`
// (tests/integration/AStandardHostKeysTest.cpp). This host tags every message
// of the file, derives every key of it under the file's naming key, generates
// every key from the file's random bytes, exports every public key, and signs
// with a generated key; what either host writes is then what the other reads.

use std::path::Path;

use sce_rust_host_features::{
    CoseKeys, HttpRequest, HttpResponse, Invocation, Outcome, Params, ReferenceHost, ScriptedClock,
    ScriptedRandom, Transport, TransportFailure,
};
use serde_json::{json, Value};

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure)
    }
}

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/keys_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

/// The member `name` of `value`, a text.
fn member<'v>(value: &'v Value, name: &str) -> &'v str {
    value[name]
        .as_str()
        .unwrap_or_else(|| panic!("{name} is a text"))
}

fn cases<'v>(vectors: &'v Value, name: &str) -> &'v [Value] {
    vectors[name]
        .as_array()
        .unwrap_or_else(|| panic!("{name} is a list of cases"))
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

/// The result record of a completion of `operation` started as the `start`-th
/// start of one `<invoke>`.
fn done(
    host: &mut ReferenceHost<Unreachable>,
    operation: &str,
    start: u64,
    pairs: &[(&str, String)],
) -> Value {
    match host
        .start(
            operation,
            &Invocation::new("session", "op", start),
            &params(pairs),
        )
        .expect("a standard type is served")
    {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("{operation} did not complete: {other:?}"),
    }
}

/// A host with no key, its random source giving `random` first.
fn host(random: &[u8]) -> ReferenceHost<Unreachable> {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let mut source = ScriptedRandom::new();
    source.set(random);
    host.set_random_source(Box::new(source));
    host
}

#[test]
fn a_tag_is_the_one_rfc_4231_gives() {
    let vectors = vectors();
    let mac = cases(&vectors, "mac");
    assert_eq!(mac.len(), 6, "RFC 4231's test cases 1 to 4, 6 and 7");
    let mut keys = CoseKeys::default();
    for (n, case) in mac.iter().enumerate() {
        keys.mac
            .insert(format!("rfc4231-{n}"), hex(member(case, "key")));
    }
    let mut host = host(&[]);
    host.set_cose_keys(keys).expect("no reserved id");
    for (n, case) in mac.iter().enumerate() {
        let name = member(case, "name");
        let key_id = format!("rfc4231-{n}");
        let message = text(&hex(member(case, "message")));
        let tag = text(&hex(member(case, "tag")));
        assert_eq!(
            done(
                &mut host,
                "sce-host-mac",
                1,
                &[("key_id", key_id.clone()), ("message", message.clone())]
            ),
            json!({ "error": "", "tag": tag }),
            "{name}"
        );
        assert_eq!(
            done(
                &mut host,
                "sce-host-mac-verify",
                1,
                &[("key_id", key_id), ("message", message), ("tag", tag)]
            ),
            json!({ "error": "", "valid": true }),
            "{name}"
        );
    }
}

#[test]
fn a_derived_key_is_named_alike_in_both_hosts() {
    let vectors = vectors();
    let mut keys = CoseKeys::default();
    for sealing in cases(&vectors, "sealing_keys") {
        keys.symmetric.insert(
            member(sealing, "id").to_string(),
            hex(member(sealing, "key"))
                .try_into()
                .expect("a 32-byte key"),
        );
    }
    let mut host = host(&hex(member(&vectors, "naming_key")));
    host.set_cose_keys(keys).expect("no reserved id");
    let derived = cases(&vectors, "derived");
    assert!(!derived.is_empty(), "a test that read no case would pass");
    for case in derived {
        let name = member(case, "name");
        let key_id = member(case, "key_id");
        assert_eq!(
            done(
                &mut host,
                "sce-host-key-derive",
                1,
                &[
                    ("secret", text(&hex(member(case, "secret")))),
                    ("input_key_id", member(case, "input_key_id").to_string()),
                    ("salt", text(&hex(member(case, "salt")))),
                    ("info", text(&hex(member(case, "info")))),
                ]
            ),
            json!({ "error": "", "key_id": key_id }),
            "{name}"
        );
        // The key kept is HKDF-SHA-256's: an object sealed under it here opens
        // under the file's key with the primitives alone.
        let sealed = done(
            &mut host,
            "sce-host-cose-encrypt0",
            1,
            &[
                ("key_id", key_id.to_string()),
                ("plaintext", "under a derived key".to_string()),
                ("external_aad", String::new()),
            ],
        );
        let object: Vec<u8> = sealed["cose"]
            .as_str()
            .expect("a byte string's text")
            .chars()
            .map(|c| u8::try_from(c).expect("a byte"))
            .collect();
        let key: [u8; 32] = hex(member(case, "key")).try_into().expect("32 bytes");
        assert_eq!(
            sce_rust_host_features::cose::decrypt0(&key, &object, b""),
            Some(b"under a derived key".to_vec()),
            "{name}: the derived key is the file's"
        );
    }
}

#[test]
fn a_generated_key_and_its_cose_key_are_the_files() {
    let vectors = vectors();
    for case in cases(&vectors, "generated") {
        let name = member(case, "name");
        let key_id = member(case, "key_id");
        let mut host = host(&hex(member(case, "random")));
        assert_eq!(
            done(
                &mut host,
                "sce-host-key-generate",
                1,
                &[
                    ("key_use", member(case, "key_use").to_string()),
                    ("label", name.to_string()),
                ]
            ),
            json!({ "error": "", "key_id": key_id }),
            "{name}"
        );
        match member(case, "key_use") {
            "sign" => {
                assert_eq!(
                    done(
                        &mut host,
                        "sce-host-key-public",
                        1,
                        &[("key_id", key_id.to_string())]
                    ),
                    json!({ "error": "", "cose_key": text(&hex(member(case, "cose_key"))) }),
                    "{name}"
                );
                assert_eq!(
                    done(
                        &mut host,
                        "sce-host-cose-sign1",
                        1,
                        &[
                            ("key_id", key_id.to_string()),
                            ("payload", text(&hex(member(case, "payload")))),
                            ("external_aad", String::new()),
                        ]
                    ),
                    json!({ "error": "", "cose": text(&hex(member(case, "cose"))) }),
                    "{name}: the object the other host verifies under the exported key"
                );
            }
            "mac" => assert_eq!(
                done(
                    &mut host,
                    "sce-host-mac",
                    1,
                    &[
                        ("key_id", key_id.to_string()),
                        ("message", text(&hex(member(case, "message")))),
                    ]
                ),
                json!({ "error": "", "tag": text(&hex(member(case, "tag"))) }),
                "{name}"
            ),
            other => panic!("{name}: no expectation is written for a key of use {other}"),
        }
    }
}

#[test]
fn a_configured_signing_key_exports_the_files_cose_key() {
    let vectors = vectors();
    let mut keys = CoseKeys::default();
    let cose_keys = cases(&vectors, "cose_keys");
    for case in cose_keys {
        keys.signing.insert(
            member(case, "key_id").to_string(),
            hex(member(case, "seed"))
                .try_into()
                .expect("a 32-byte seed"),
        );
    }
    let mut host = host(&[]);
    host.set_cose_keys(keys).expect("no reserved id");
    for case in cose_keys {
        let key_id = member(case, "key_id");
        assert_eq!(
            done(
                &mut host,
                "sce-host-key-public",
                1,
                &[("key_id", key_id.to_string())]
            ),
            json!({ "error": "", "cose_key": text(&hex(member(case, "cose_key"))) }),
            "{key_id}"
        );
    }
}
