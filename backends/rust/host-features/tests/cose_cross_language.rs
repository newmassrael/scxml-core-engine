// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The objects the Rust and the C++ reference hosts agree on, byte for byte
// (docs/SCE_HOST_FEATURES.md, section 10): `tests/host_features/cose_cross_language.json`,
// which the C++ host is held to by `ACoseObjectCrossesBetweenTheHosts` in
// tests/integration/AStandardHostCoseTest.cpp. This host writes every object of
// the file through its operations, a seal under the nonce the case fixes, and
// verifies and opens every object of it; the C++ host does the same, so an object
// either writes is one the other reads.

use std::path::Path;

use sce_rust_host_features::cose::{public_key_of, IV_BYTES};
use sce_rust_host_features::{
    CoseKeys, HttpRequest, HttpResponse, Invocation, NonceSource, Outcome, Params, ReferenceHost,
    ScriptedClock, Transport, TransportFailure, TrustedKey,
};
use serde_json::Value;

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

/// A random source that gives the nonce a case fixes.
struct Fixed([u8; IV_BYTES]);

impl NonceSource for Fixed {
    fn next_nonce(&mut self) -> Option<[u8; IV_BYTES]> {
        Some(self.0)
    }
}

/// The trust set of both signing keys of the file, in its order.
const TRUST_SET: &str = "devices";

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/cose_cross_language.json");
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

fn cases<'v>(vectors: &'v Value, operation: &str) -> &'v [Value] {
    vectors[operation]
        .as_array()
        .unwrap_or_else(|| panic!("{operation} is a list of cases"))
}

/// The text a byte string travels as, one character to a byte.
fn text(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

/// The host, given the file's signing keys, its sealing key, and the trust set
/// [`TRUST_SET`] of every signing key's public key.
fn host(vectors: &Value) -> ReferenceHost<Unreachable> {
    let mut keys = CoseKeys::default();
    let mut trusted = Vec::new();
    for signer in cases(vectors, "signing_keys") {
        let id = member(signer, "id").to_string();
        let seed = key(member(signer, "seed"));
        let public_key = key(member(signer, "public_key"));
        assert_eq!(
            public_key_of(&seed),
            public_key,
            "{id}: the file's public key is the one its seed gives"
        );
        keys.signing.insert(id.clone(), seed);
        trusted.push(TrustedKey { id, public_key });
    }
    for sealing in cases(vectors, "sealing_keys") {
        keys.symmetric.insert(
            member(sealing, "id").to_string(),
            key(member(sealing, "key")),
        );
    }
    keys.trust.insert(TRUST_SET.to_string(), trusted);
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    host.set_cose_keys(keys).expect("no reserved id");
    host
}

fn params(pairs: &[(&str, String)]) -> Params {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect()
}

/// The first start of the `<invoke>` `id` in one session: a COSE operation
/// keeps no first result, so every case may reuse an identity.
fn invocation(id: &str) -> Invocation {
    Invocation::new("session", id, 1)
}

/// The result record of a completion.
fn done(outcome: Option<Outcome>) -> Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

#[test]
fn the_host_signs_each_object_byte_for_byte() {
    let vectors = vectors();
    let mut host = host(&vectors);
    for case in cases(&vectors, "sign1") {
        let name = member(case, "name");
        let fields = done(host.start(
            "sce-host-cose-sign1",
            &invocation("sign"),
            &params(&[
                ("key_id", member(case, "key_id").to_string()),
                ("payload", text(&hex(member(case, "payload")))),
                ("external_aad", text(&hex(member(case, "external_aad")))),
            ]),
        ));
        assert_eq!(fields["error"], "", "{name}");
        assert_eq!(
            fields["cose"],
            text(&hex(member(case, "cose"))),
            "{name}: the object the other host verifies"
        );
    }
}

#[test]
fn the_host_seals_each_object_byte_for_byte_under_its_nonce() {
    let vectors = vectors();
    let mut host = host(&vectors);
    for case in cases(&vectors, "encrypt0") {
        let name = member(case, "name");
        let nonce: [u8; IV_BYTES] = hex(member(case, "nonce"))
            .try_into()
            .expect("a nonce is 12 bytes");
        host.set_nonce_source(Box::new(Fixed(nonce)));
        let fields = done(host.start(
            "sce-host-cose-encrypt0",
            &invocation("seal"),
            &params(&[
                ("key_id", member(case, "key_id").to_string()),
                ("plaintext", text(&hex(member(case, "plaintext")))),
                ("external_aad", text(&hex(member(case, "external_aad")))),
            ]),
        ));
        assert_eq!(fields["error"], "", "{name}");
        assert_eq!(
            fields["cose"],
            text(&hex(member(case, "cose"))),
            "{name}: the object the other host opens"
        );
    }
}

#[test]
fn the_host_verifies_each_object_and_not_over_other_external_data() {
    let vectors = vectors();
    let mut host = host(&vectors);
    for case in cases(&vectors, "sign1") {
        let name = member(case, "name");
        let cose = text(&hex(member(case, "cose")));
        let aad = hex(member(case, "external_aad"));
        let verify = |host: &mut ReferenceHost<Unreachable>, aad: &[u8]| {
            done(host.start(
                "sce-host-cose-verify1",
                &invocation("verify"),
                &params(&[
                    ("trust_set", TRUST_SET.to_string()),
                    ("cose", cose.clone()),
                    ("external_aad", text(aad)),
                ]),
            ))
        };
        let fields = verify(&mut host, aad.as_slice());
        assert_eq!(fields["error"], "", "{name}");
        assert_eq!(fields["valid"], true, "{name}");
        assert_eq!(fields["key_id"], member(case, "key_id"), "{name}");
        assert_eq!(
            fields["payload"],
            text(&hex(member(case, "payload"))),
            "{name}"
        );
        let mut other = aad;
        other.push(b'!');
        let fields = verify(&mut host, other.as_slice());
        assert_eq!(
            fields["valid"], false,
            "{name}: the external data is part of what was signed"
        );
        assert_eq!(fields["payload"], "", "{name}");
    }
}

#[test]
fn the_host_opens_each_object_and_not_one_changed_by_a_bit() {
    let vectors = vectors();
    let mut host = host(&vectors);
    for case in cases(&vectors, "encrypt0") {
        let name = member(case, "name");
        let object = hex(member(case, "cose"));
        let open = |host: &mut ReferenceHost<Unreachable>, object: &[u8]| {
            done(host.start(
                "sce-host-cose-decrypt0",
                &invocation("open"),
                &params(&[
                    ("key_id", member(case, "key_id").to_string()),
                    ("cose", text(object)),
                    ("external_aad", text(&hex(member(case, "external_aad")))),
                ]),
            ))
        };
        let fields = open(&mut host, object.as_slice());
        assert_eq!(fields["error"], "", "{name}");
        assert_eq!(fields["opened"], true, "{name}");
        assert_eq!(
            fields["plaintext"],
            text(&hex(member(case, "plaintext"))),
            "{name}"
        );
        let mut tampered = object;
        *tampered.last_mut().expect("an object has bytes") ^= 0x01;
        let fields = open(&mut host, tampered.as_slice());
        assert_eq!(
            fields["opened"], false,
            "{name}: the tag authenticates the ciphertext"
        );
        assert_eq!(fields["plaintext"], "", "{name}");
    }
}
