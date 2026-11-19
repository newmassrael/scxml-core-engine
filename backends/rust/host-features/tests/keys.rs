// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Random bytes, keyed digests and keys through the reference host
// (docs/SCE_HOST_FEATURES.md, sections 11 to 13; docs/adr/0109, D5 to D7): each
// operation's answers, failures and refusals, a key's one use, a generation
// answered once under its identity, and a retired key that makes nothing new.

use std::collections::BTreeMap;

use sce_rust_host_features::{
    CoseKeys, HttpRequest, HttpResponse, Invocation, Outcome, Params, RandomSource, ReferenceHost,
    ScriptedClock, ScriptedRandom, Transport, TransportFailure,
};

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

/// A random source that gives nothing.
struct Exhausted;

impl RandomSource for Exhausted {
    fn fill(&mut self, _: &mut [u8]) -> bool {
        false
    }
}

/// The seed of RFC 8032 section 7.1, test 1.
const SEED_11: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];

/// The `COSE_Key` of its public key under the key id `11`: `{1: 1, 2: h'3131', -1: 6, -2: x}`.
const COSE_KEY_11: &str =
    "a40101024231312006215820d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

/// The text a byte string travels as, one character to a byte.
fn text(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

fn params(pairs: &[(&str, &str)]) -> Params {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// A host whose configured keys are the signing key `11`, the sealing key `box`
/// and the MAC key `jefe` (RFC 4231 test case 2's), and whose random source gives
/// `random` first.
fn host(random: &[u8]) -> ReferenceHost<Unreachable> {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let mut keys = CoseKeys::default();
    keys.signing.insert("11".to_string(), SEED_11);
    keys.symmetric.insert("box".to_string(), [0x40; 32]);
    keys.mac.insert("jefe".to_string(), b"Jefe".to_vec());
    host.set_cose_keys(keys).expect("no reserved id");
    let mut source = ScriptedRandom::new();
    source.set(random);
    host.set_random_source(Box::new(source));
    host
}

fn start(
    host: &mut ReferenceHost<Unreachable>,
    operation: &str,
    start: u64,
    pairs: &[(&str, &str)],
) -> Option<Outcome> {
    host.start(
        operation,
        &Invocation::new("session", "op", start),
        &params(pairs),
    )
}

fn done(outcome: Option<Outcome>) -> serde_json::Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

fn refused(outcome: Option<Outcome>) -> bool {
    matches!(outcome, Some(Outcome::Refuse(_)))
}

fn generate(host: &mut ReferenceHost<Unreachable>, start_number: u64, key_use: &str) -> String {
    let fields = done(start(
        host,
        "sce-host-key-generate",
        start_number,
        &[("key_use", key_use), ("label", "a label")],
    ));
    assert_eq!(fields["error"], "", "{fields}");
    fields["key_id"].as_str().expect("an id").to_string()
}

// ── Random bytes ───────────────────────────────────────────────────────

#[test]
fn random_bytes_are_drawn_from_the_hosts_source() {
    let mut host = host(&[0x00, 0x7f, 0x80, 0xff, 0x41]);
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-random-bytes",
            1,
            &[("length", "4")]
        )),
        serde_json::json!({ "error": "", "bytes": text(&[0x00, 0x7f, 0x80, 0xff]) })
    );
    let next = done(start(
        &mut host,
        "sce-host-random-bytes",
        2,
        &[("length", "256")],
    ));
    let drawn = next["bytes"].as_str().expect("a byte string's text");
    assert_eq!(drawn.chars().count(), 256, "the bound of the result");
    assert_eq!(drawn.chars().next(), Some('A'), "the byte queued last");
}

#[test]
fn two_draws_from_the_system_differ() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let mut draw = |n: u64| {
        done(start(
            &mut host,
            "sce-host-random-bytes",
            n,
            &[("length", "32")],
        ))["bytes"]
            .clone()
    };
    assert_ne!(draw(1), draw(2));
}

#[test]
fn a_length_of_none_or_past_the_bound_is_refused_and_a_dry_source_fails() {
    let mut host = host(&[]);
    for length in ["0", "257", "", "x", "-1"] {
        assert!(
            refused(start(
                &mut host,
                "sce-host-random-bytes",
                1,
                &[("length", length)]
            )),
            "length {length:?}"
        );
    }
    host.set_random_source(Box::new(Exhausted));
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-random-bytes",
            1,
            &[("length", "8")]
        )),
        serde_json::json!({ "error": "unavailable", "bytes": "" })
    );
}

// ── Keyed digests ──────────────────────────────────────────────────────

/// RFC 4231 test case 2: the key `Jefe`.
const JEFE_MESSAGE: &str = "what do ya want for nothing?";
const JEFE_TAG: &str = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";

#[test]
fn a_tag_is_hmac_sha_256_and_verifies_whole_and_nothing_else_does() {
    let mut host = host(&[]);
    let tag = text(&hex(JEFE_TAG));
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-mac",
            1,
            &[("key_id", "jefe"), ("message", JEFE_MESSAGE)]
        )),
        serde_json::json!({ "error": "", "tag": tag })
    );
    let verify = |host: &mut ReferenceHost<Unreachable>, message: &str, tag: &str| {
        done(start(
            host,
            "sce-host-mac-verify",
            1,
            &[("key_id", "jefe"), ("message", message), ("tag", tag)],
        ))
    };
    assert_eq!(
        verify(&mut host, JEFE_MESSAGE, &tag),
        serde_json::json!({ "error": "", "valid": true })
    );
    let mut flipped = hex(JEFE_TAG);
    flipped[0] ^= 0x01;
    for (why, message, tag) in [
        ("a changed tag", JEFE_MESSAGE, text(&flipped)),
        ("another message", "what do ya want?", tag.clone()),
        ("a truncated tag", JEFE_MESSAGE, text(&hex(JEFE_TAG)[..16])),
        ("no tag", JEFE_MESSAGE, String::new()),
    ] {
        assert_eq!(
            verify(&mut host, message, &tag),
            serde_json::json!({ "error": "", "valid": false }),
            "{why} is an answer, not a failure"
        );
    }
}

#[test]
fn a_key_that_does_not_tag_fails_and_a_request_past_its_record_is_refused() {
    let mut host = host(&[]);
    for key_id in ["nobody", "11", "box"] {
        assert_eq!(
            done(start(
                &mut host,
                "sce-host-mac",
                1,
                &[("key_id", key_id), ("message", "m")]
            )),
            serde_json::json!({ "error": "unknown_key", "tag": "" }),
            "{key_id} is no MAC key"
        );
        assert_eq!(
            done(start(
                &mut host,
                "sce-host-mac-verify",
                1,
                &[("key_id", key_id), ("message", "m"), ("tag", "")]
            )),
            serde_json::json!({ "error": "unknown_key", "valid": false })
        );
    }
    assert!(refused(start(
        &mut host,
        "sce-host-mac",
        1,
        &[("key_id", ""), ("message", "m")]
    )));
    assert!(refused(start(
        &mut host,
        "sce-host-mac",
        1,
        &[("key_id", "jefe"), ("message", &"m".repeat(4097))]
    )));
    assert!(refused(start(
        &mut host,
        "sce-host-mac-verify",
        1,
        &[
            ("key_id", "jefe"),
            ("message", "m"),
            ("tag", &"t".repeat(33))
        ]
    )));
}

// ── Keys ───────────────────────────────────────────────────────────────

/// The id's 16 bytes and the key's 32 of one generation.
fn drawn(first: u8) -> Vec<u8> {
    (first..first + 48).collect()
}

#[test]
fn a_generated_key_is_named_by_its_drawn_bytes_and_answered_once_under_its_identity() {
    let mut random = drawn(0x00);
    random.extend(drawn(0x80));
    let mut host = host(&random);
    let first = generate(&mut host, 1, "sign");
    assert_eq!(first, "generated:000102030405060708090a0b0c0d0e0f");
    assert_eq!(
        generate(&mut host, 1, "sign"),
        first,
        "the same start, repeated by a restore, answers its first key"
    );
    assert_eq!(
        generate(&mut host, 2, "mac"),
        "generated:808182838485868788898a8b8c8d8e8f",
        "the next start draws the next bytes"
    );
    for (key_use, label) in [
        ("derived", ""),
        ("verify", ""),
        ("", ""),
        ("sign", &"l".repeat(65)[..]),
    ] {
        assert!(
            refused(start(
                &mut host,
                "sce-host-key-generate",
                3,
                &[("key_use", key_use), ("label", label)]
            )),
            "use {key_use:?}, a label of {} bytes",
            label.len()
        );
    }
    host.set_random_source(Box::new(Exhausted));
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-key-generate",
            4,
            &[("key_use", "seal"), ("label", "")]
        )),
        serde_json::json!({ "error": "unavailable", "key_id": "" })
    );
}

#[test]
fn each_generated_key_serves_its_one_use() {
    let mut host = host(&[]);
    let sign = generate(&mut host, 1, "sign");
    let seal = generate(&mut host, 2, "seal");
    let mac = generate(&mut host, 3, "mac");
    let derive = generate(&mut host, 4, "derive");

    let sign_with = |host: &mut ReferenceHost<Unreachable>, key_id: &str| {
        done(start(
            host,
            "sce-host-cose-sign1",
            9,
            &[("key_id", key_id), ("payload", "p"), ("external_aad", "")],
        ))["error"]
            .clone()
    };
    let seal_with = |host: &mut ReferenceHost<Unreachable>, key_id: &str| {
        done(start(
            host,
            "sce-host-cose-encrypt0",
            9,
            &[("key_id", key_id), ("plaintext", "p"), ("external_aad", "")],
        ))["error"]
            .clone()
    };
    let tag_with = |host: &mut ReferenceHost<Unreachable>, key_id: &str| {
        done(start(
            host,
            "sce-host-mac",
            9,
            &[("key_id", key_id), ("message", "m")],
        ))["error"]
            .clone()
    };
    let derive_from = |host: &mut ReferenceHost<Unreachable>, key_id: &str| {
        done(start(
            host,
            "sce-host-key-derive",
            9,
            &[
                ("secret", ""),
                ("input_key_id", key_id),
                ("salt", ""),
                ("info", ""),
            ],
        ))["error"]
            .clone()
    };
    for (key_id, serves) in [
        (&sign, "sign"),
        (&seal, "seal"),
        (&mac, "mac"),
        (&derive, "derive"),
    ] {
        let expect = |use_: &str| if use_ == serves { "" } else { "unknown_key" };
        assert_eq!(sign_with(&mut host, key_id), expect("sign"), "{serves}");
        assert_eq!(seal_with(&mut host, key_id), expect("seal"), "{serves}");
        assert_eq!(tag_with(&mut host, key_id), expect("mac"), "{serves}");
        assert_eq!(derive_from(&mut host, key_id), expect("derive"), "{serves}");
    }
}

#[test]
fn a_signing_key_exports_its_public_key_as_a_cose_key() {
    let mut host = host(&drawn(0x00));
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-key-public",
            1,
            &[("key_id", "11")]
        )),
        serde_json::json!({ "error": "", "cose_key": text(&hex(COSE_KEY_11)) })
    );
    let generated = generate(&mut host, 1, "sign");
    let exported = done(start(
        &mut host,
        "sce-host-key-public",
        2,
        &[("key_id", &generated)],
    ));
    assert_eq!(exported["error"], "");
    let cose_key = exported["cose_key"].as_str().expect("a byte string's text");
    assert!(
        cose_key.starts_with(&text(&hex("a4010102582a"))),
        "a map of four, kty OKP, and a 42-byte kid: {cose_key:?}"
    );
    assert!(cose_key.contains(&generated), "the kid is the key's id");
    for key_id in ["box", "jefe", "nobody"] {
        assert_eq!(
            done(start(
                &mut host,
                "sce-host-key-public",
                3,
                &[("key_id", key_id)]
            )),
            serde_json::json!({ "error": "unknown_key", "cose_key": "" }),
            "{key_id} has no public key"
        );
    }
    assert!(refused(start(
        &mut host,
        "sce-host-key-public",
        3,
        &[("key_id", "")]
    )));
}

#[test]
fn a_retired_key_makes_nothing_new_and_still_verifies_and_exports() {
    let mut host = host(&[]);
    let sign = generate(&mut host, 1, "sign");
    let mac = generate(&mut host, 2, "mac");
    let tag = done(start(
        &mut host,
        "sce-host-mac",
        3,
        &[("key_id", &mac), ("message", "m")],
    ))["tag"]
        .as_str()
        .expect("a tag")
        .to_string();
    for key_id in [&sign, &mac] {
        assert_eq!(
            done(start(
                &mut host,
                "sce-host-key-retire",
                4,
                &[("key_id", key_id)]
            )),
            serde_json::json!({ "error": "", "retired": true })
        );
        assert_eq!(
            done(start(
                &mut host,
                "sce-host-key-retire",
                5,
                &[("key_id", key_id)]
            )),
            serde_json::json!({ "error": "", "retired": false }),
            "retired already"
        );
    }
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-cose-sign1",
            6,
            &[("key_id", &sign), ("payload", "p"), ("external_aad", "")]
        )),
        serde_json::json!({ "error": "key_retired", "cose": "" })
    );
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-key-public",
            6,
            &[("key_id", &sign)]
        ))["error"],
        "",
        "a retired key's public key still verifies what it signed"
    );
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-mac",
            6,
            &[("key_id", &mac), ("message", "m")]
        )),
        serde_json::json!({ "error": "key_retired", "tag": "" })
    );
    assert_eq!(
        done(start(
            &mut host,
            "sce-host-mac-verify",
            6,
            &[("key_id", &mac), ("message", "m"), ("tag", &tag)]
        )),
        serde_json::json!({ "error": "", "valid": true }),
        "a retired MAC key still verifies"
    );
    for key_id in ["11", "generated:00000000000000000000000000000000"] {
        assert_eq!(
            done(start(
                &mut host,
                "sce-host-key-retire",
                7,
                &[("key_id", key_id)]
            )),
            serde_json::json!({ "error": "unknown_key", "retired": false }),
            "{key_id} is no key of the host's key ring"
        );
    }
}

/// What an injected failure of one operation is held to: its result record with
/// every field at its zero, and the request that starts it.
type ZeroOf = (serde_json::Value, Vec<(&'static str, &'static str)>);

#[test]
fn an_injected_failure_answers_each_record_at_its_zero() {
    let mut host = host(&[]);
    let zero: BTreeMap<&str, ZeroOf> = BTreeMap::from([
        (
            "sce-host-random-bytes",
            (serde_json::json!({ "bytes": "" }), vec![("length", "1")]),
        ),
        (
            "sce-host-mac",
            (
                serde_json::json!({ "tag": "" }),
                vec![("key_id", "jefe"), ("message", "")],
            ),
        ),
        (
            "sce-host-mac-verify",
            (
                serde_json::json!({ "valid": false }),
                vec![("key_id", "jefe"), ("message", ""), ("tag", "")],
            ),
        ),
        (
            "sce-host-key-generate",
            (
                serde_json::json!({ "key_id": "" }),
                vec![("key_use", "mac"), ("label", "")],
            ),
        ),
        (
            "sce-host-key-public",
            (
                serde_json::json!({ "cose_key": "" }),
                vec![("key_id", "11")],
            ),
        ),
        (
            "sce-host-key-retire",
            (
                serde_json::json!({ "retired": false }),
                vec![("key_id", "11")],
            ),
        ),
    ]);
    for (operation, (mut fields, request)) in zero {
        host.set_faults(BTreeMap::from([(
            operation.to_string(),
            "unavailable".to_string(),
        )]));
        fields["error"] = serde_json::json!("unavailable");
        assert_eq!(
            done(start(&mut host, operation, 1, &request)),
            fields,
            "{operation}"
        );
    }
}
