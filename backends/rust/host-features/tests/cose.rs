// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Signing and sealing (docs/SCE_HOST_FEATURES.md, section 10) held apart from the
// scenario the lanes share: the bytes of an object to the deterministic encoding of
// RFC 9052, the COSE working group's examples for the algorithms the host fixes, the
// rules of RFC 9052 section 3 on the header buckets, and what the host refuses and
// numbers.
//
// The expected bytes were computed by an implementation that shares nothing with this
// crate (the Python `cryptography` package over OpenSSL, with a CBOR writer of its
// own), which also reproduced the working group's signature of `eddsa-sig-01.json`
// from its key before any of them was taken.

use std::collections::BTreeMap;

use sce_rust_host_features::cose::{decrypt0, derive_key, encrypt0, public_key_of, sign1, verify1};
use sce_rust_host_features::{
    CoseKeys, HttpRequest, HttpResponse, Outcome, Params, ReferenceHost, ReservedKeyId,
    ScriptedClock, Transport, TransportFailure, TrustedKey,
};

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure)
    }
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn key(text: &str) -> [u8; 32] {
    hex(text).try_into().expect("32 bytes")
}

/// The seed of RFC 8032 section 7.1, test 1, which the working group's EdDSA
/// examples sign with under the key id `11`.
const SEED_11: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

/// Its public key, as RFC 8032 gives it.
const PUBLIC_11: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

/// The key `sec-256` of the working group's ChaCha20/Poly1305 examples.
const SEC_256: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f01f2e3d4c5b6a798897a6b5c4d3e2f100";

/// The nonce `chacha-poly-enc-01.json` drew.
const IV: &str = "5c3a9950bd2852f66e6c8d4f";

const CONTENT: &[u8] = b"This is the content.";

/// `sign1` of [`CONTENT`] under the key `11`, with no external data: the protected
/// header `{1: -8, 4: h'3131'}`, an empty unprotected one.
const SIGN1_11: &str = concat!(
    "d28447a2012704423131a054546869732069732074686520636f6e74656e742e",
    "58400e21f6825a41fde57bfe971a9a817a831e079f32118289107d83762071d0",
    "dcb41489f8744467ee555f620e03a90d548458d935a5b4cd38923e8eda16ee2c",
    "1101",
);

/// The same, signed over the external data `context`.
const SIGN1_11_CONTEXT: &str = concat!(
    "d28447a2012704423131a054546869732069732074686520636f6e74656e742e",
    "5840fb5efadba5361973402ec9f69f48224d9303a3106fe7945ee3b69cfe03aa",
    "9f85a1511d1c19d925cc1471fc20d0a10b20fd916f30c85dc51044bbd1a980f3",
    "d10b",
);

/// The same, but its protected header names ES256 (-7): validly signed with the
/// Ed25519 key all the same, so only the algorithm keeps it from verifying.
const SIGN1_ES256: &str = concat!(
    "d28447a2012604423131a054546869732069732074686520636f6e74656e742e",
    "5840053e58f44fdca181fe82414db271b814219f4b2d90fb9fd516b517ae244e",
    "a26bade4e504096c9dcd111d1280dc4dbb4a3f0fa89b6126b73afc9254883995",
    "c307",
);

/// COSE WG Examples, `eddsa-examples/eddsa-sig-01.json`, its `output.cbor`: the
/// key id `11` in the unprotected bucket and a content type beside the algorithm.
const WG_EDDSA_SIG_01: &str = concat!(
    "d28445a201270300a10442313154546869732069732074686520636f6e74656e",
    "742e58407142fd2ff96d56db85bee905a76ba1d0b7321a95c8c4d3607c578193",
    "2b7afb8711497dfa751bf40b58b3bcc32300b1487f3db34085eef013bf08f4a4",
    "4d6fef0d",
);

/// COSE WG Examples, `chacha-poly-examples/chacha-poly-enc-01.json`, its
/// `output.cbor`: no key id, the nonce in the unprotected bucket. It opens under
/// the `Enc_structure` of RFC 9052 section 5.3 (the context `Encrypt0`); the file's
/// `intermediates.AAD_hex` spells the context `Encrypt1`, and does not open it.
const WG_CHACHA_POLY_ENC_01: &str = concat!(
    "d08344a1011818a1054c5c3a9950bd2852f66e6c8d4f5824ca119c45926da993",
    "d29b5d0cac9a84228c7668d492a1b9d7e32020ef21372e74def431b9",
);

/// `encrypt0` of [`CONTENT`] under `sec-256` with the nonce [`IV`] and the key id
/// `sec-256`: the working group's ciphertext, since the key id is unprotected and
/// so outside what is authenticated.
const ENCRYPT0_SEC_256: &str = concat!(
    "d08344a1011818a204477365632d323536054c5c3a9950bd2852f66e6c8d4f58",
    "24ca119c45926da993d29b5d0cac9a84228c7668d492a1b9d7e32020ef21372e",
    "74def431b9",
);

fn signer_11() -> Vec<TrustedKey> {
    vec![TrustedKey {
        id: "11".to_string(),
        public_key: key(PUBLIC_11),
    }]
}

/// `object` with its byte at `at` (the unprotected bucket's map head, an empty
/// map `a0`) replaced by `with`.
fn with_unprotected(object: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    assert_eq!(object[at], 0xa0, "the unprotected bucket is empty there");
    let mut changed = object[..at].to_vec();
    changed.extend_from_slice(with);
    changed.extend_from_slice(&object[at + 1..]);
    changed
}

#[test]
fn a_seed_makes_the_public_key_rfc_8032_gives() {
    assert_eq!(public_key_of(&key(SEED_11)), key(PUBLIC_11));
}

#[test]
fn a_signature_is_the_deterministic_cose_sign1() {
    assert_eq!(sign1(&key(SEED_11), b"11", CONTENT, b""), hex(SIGN1_11));
    assert_eq!(
        sign1(&key(SEED_11), b"11", CONTENT, b"context"),
        hex(SIGN1_11_CONTEXT)
    );
}

#[test]
fn a_signed_object_verifies_under_its_signer_and_its_external_data() {
    assert_eq!(
        verify1(&signer_11(), &hex(SIGN1_11), b""),
        Some(("11".to_string(), CONTENT.to_vec()))
    );
    assert_eq!(
        verify1(&signer_11(), &hex(SIGN1_11_CONTEXT), b"context"),
        Some(("11".to_string(), CONTENT.to_vec()))
    );
    assert_eq!(
        verify1(&signer_11(), &hex(SIGN1_11_CONTEXT), b""),
        None,
        "the external data is part of what was signed"
    );
}

#[test]
fn the_working_groups_eddsa_example_verifies() {
    assert_eq!(
        verify1(&signer_11(), &hex(WG_EDDSA_SIG_01), b""),
        Some(("11".to_string(), CONTENT.to_vec()))
    );
}

#[test]
fn an_object_that_is_not_what_was_signed_does_not_verify() {
    let mut tampered = hex(SIGN1_11);
    *tampered.last_mut().expect("bytes") ^= 0x01;
    assert_eq!(verify1(&signer_11(), &tampered, b""), None);
    assert_eq!(
        verify1(&signer_11(), &hex(SIGN1_ES256), b""),
        None,
        "an object naming another algorithm does not verify"
    );
    let untagged = hex(SIGN1_11)[1..].to_vec();
    assert_eq!(verify1(&signer_11(), &untagged, b""), None);
    let mut trailing = hex(SIGN1_11);
    trailing.push(0x00);
    assert_eq!(verify1(&signer_11(), &trailing, b""), None);
}

/// The unprotected bucket is outside the signature, so each of these objects still
/// carries a valid signature: what keeps them from verifying is the rule.
#[test]
fn a_header_in_both_buckets_or_a_critical_one_does_not_verify() {
    let signed = hex(SIGN1_11);
    // The unprotected bucket's map head follows the tag, the array head and the
    // protected bucket's eight bytes.
    let at = 10;
    let kid_twice = with_unprotected(&signed, at, &[0xa1, 0x04, 0x42, 0x31, 0x31]);
    assert_eq!(verify1(&signer_11(), &kid_twice, b""), None);
    let critical = with_unprotected(&signed, at, &[0xa1, 0x02, 0x81, 0x04]);
    assert_eq!(verify1(&signer_11(), &critical, b""), None);
    let unknown = with_unprotected(&signed, at, &[0xa1, 0x18, 0x63, 0x01]);
    assert_eq!(
        verify1(&signer_11(), &unknown, b""),
        Some(("11".to_string(), CONTENT.to_vec())),
        "a header these operations do not read is passed over"
    );
}

#[test]
fn a_key_id_selects_its_signer_alone() {
    let mut trusted = vec![TrustedKey {
        id: "other".to_string(),
        public_key: key(PUBLIC_11),
    }];
    assert_eq!(
        verify1(&trusted, &hex(SIGN1_11), b""),
        None,
        "the object says it was signed by `11`, which the set does not hold"
    );
    trusted.extend(signer_11());
    assert_eq!(
        verify1(&trusted, &hex(SIGN1_11), b""),
        Some(("11".to_string(), CONTENT.to_vec()))
    );
}

#[test]
fn a_seal_is_the_deterministic_cose_encrypt0_for_its_nonce() {
    let iv: [u8; 12] = hex(IV).try_into().expect("12 bytes");
    assert_eq!(
        encrypt0(&key(SEC_256), b"sec-256", &iv, CONTENT, b""),
        hex(ENCRYPT0_SEC_256)
    );
}

#[test]
fn the_working_groups_chacha_poly_example_opens() {
    assert_eq!(
        decrypt0(&key(SEC_256), &hex(WG_CHACHA_POLY_ENC_01), b""),
        Some(CONTENT.to_vec())
    );
    assert_eq!(
        decrypt0(&key(SEC_256), &hex(ENCRYPT0_SEC_256), b""),
        Some(CONTENT.to_vec())
    );
    assert_eq!(
        decrypt0(&key(SEC_256), &hex(WG_CHACHA_POLY_ENC_01), b"x"),
        None,
        "the external data is part of what was authenticated"
    );
    assert_eq!(decrypt0(&[7u8; 32], &hex(WG_CHACHA_POLY_ENC_01), b""), None);
}

#[test]
fn a_partial_nonce_does_not_open() {
    // The unprotected bucket `{5: iv}` gains `6: h'00'`.
    let mut object = hex(WG_CHACHA_POLY_ENC_01);
    assert_eq!(object[7], 0xa1, "the unprotected bucket holds one header");
    object[7] = 0xa2;
    let after_iv = 8 + 2 + 12;
    object.splice(after_iv..after_iv, [0x06, 0x41, 0x00]);
    assert_eq!(decrypt0(&key(SEC_256), &object, b""), None);
}

/// RFC 5869, appendix A.1: the first 32 bytes of its OKM.
#[test]
fn a_derived_key_is_hkdf_sha_256() {
    let input = [0x0bu8; 22];
    let salt: Vec<u8> = (0x00..=0x0c).collect();
    let info: Vec<u8> = (0xf0..=0xf9).collect();
    assert_eq!(
        derive_key(&input, &salt, &info),
        key("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf")
    );
}

// ── Through the host ───────────────────────────────────────────────────

fn host() -> ReferenceHost<Unreachable> {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let mut keys = CoseKeys::default();
    keys.signing.insert("11".to_string(), key(SEED_11));
    keys.symmetric.insert("sec-256".to_string(), key(SEC_256));
    keys.trust.insert("wg".to_string(), signer_11());
    host.set_cose_keys(keys).expect("no reserved id");
    host
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

fn done(outcome: Option<Outcome>) -> serde_json::Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

fn refused(outcome: Option<Outcome>) -> bool {
    matches!(outcome, Some(Outcome::Refuse(_)))
}

fn derive(host: &mut ReferenceHost<Unreachable>, secret: &str, input: &str) -> Option<Outcome> {
    host.start(
        "sce-host-key-derive",
        "derive",
        &params(&[
            ("secret", secret),
            ("input_key_id", input),
            ("salt", "salt"),
            ("info", "cose key"),
        ]),
    )
}

#[test]
fn the_host_signs_the_object_the_encoder_writes() {
    let mut host = host();
    let fields = done(host.start(
        "sce-host-cose-sign1",
        "sign",
        &params(&[
            ("key_id", "11"),
            ("payload", "This is the content."),
            ("external_aad", ""),
        ]),
    ));
    assert_eq!(fields["error"], "");
    assert_eq!(fields["cose"], text(&hex(SIGN1_11)));
}

#[test]
fn an_empty_key_id_or_a_character_past_u00ff_is_refused() {
    let mut host = host();
    assert!(refused(host.start(
        "sce-host-cose-sign1",
        "sign",
        &params(&[("key_id", ""), ("payload", "p"), ("external_aad", "")]),
    )));
    assert!(refused(host.start(
        "sce-host-cose-sign1",
        "sign",
        &params(&[
            ("key_id", "11"),
            ("payload", "\u{100}"),
            ("external_aad", "")
        ]),
    )));
    assert!(refused(host.start(
        "sce-host-cose-verify1",
        "verify",
        &params(&[("trust_set", ""), ("cose", ""), ("external_aad", "")]),
    )));
    for key_operation in ["sce-host-cose-encrypt0", "sce-host-cose-decrypt0"] {
        assert!(refused(host.start(
            key_operation,
            "op",
            &params(&[
                ("key_id", ""),
                ("plaintext", ""),
                ("cose", ""),
                ("external_aad", "")
            ]),
        )));
    }
}

#[test]
fn a_key_the_host_does_not_hold_fails_with_every_field_at_its_zero() {
    let mut host = host();
    let fields = done(host.start(
        "sce-host-cose-verify1",
        "verify",
        &params(&[
            ("trust_set", "strangers"),
            ("cose", &text(&hex(SIGN1_11))),
            ("external_aad", ""),
        ]),
    ));
    assert_eq!(
        fields,
        serde_json::json!({ "error": "unknown_key", "valid": false, "key_id": "", "payload": "" })
    );
    let fields = done(host.start(
        "sce-host-cose-encrypt0",
        "seal",
        &params(&[
            ("key_id", "nobody"),
            ("plaintext", "p"),
            ("external_aad", ""),
        ]),
    ));
    assert_eq!(
        fields,
        serde_json::json!({ "error": "unknown_key", "cose": "" })
    );
}

#[test]
fn a_derivation_takes_the_next_number_and_a_refused_or_failed_one_takes_none() {
    let mut host = host();
    assert_eq!(
        done(derive(&mut host, "shared secret", ""))["key_id"],
        "derived:1"
    );
    assert!(
        refused(derive(&mut host, "shared secret", "sec-256")),
        "both inputs"
    );
    assert!(refused(derive(&mut host, "", "")), "neither input");
    assert_eq!(
        done(derive(&mut host, "", "nobody"))["error"],
        "unknown_key"
    );
    assert_eq!(
        done(derive(&mut host, "", "sec-256"))["key_id"],
        "derived:2"
    );
}

#[test]
fn a_seal_opens_with_a_key_derived_alike_and_not_with_another() {
    let mut host = host();
    assert_eq!(
        done(derive(&mut host, "shared secret", ""))["key_id"],
        "derived:1"
    );
    assert_eq!(
        done(derive(&mut host, "shared secret", ""))["key_id"],
        "derived:2"
    );
    assert_eq!(
        done(derive(&mut host, "", "derived:1"))["key_id"],
        "derived:3"
    );
    let sealed = done(host.start(
        "sce-host-cose-encrypt0",
        "seal",
        &params(&[
            ("key_id", "derived:1"),
            ("plaintext", "sealed"),
            ("external_aad", ""),
        ]),
    ));
    assert_eq!(sealed["error"], "");
    let cose = sealed["cose"].as_str().expect("a byte string's text");
    let open = |host: &mut ReferenceHost<Unreachable>, key_id: &str| {
        done(host.start(
            "sce-host-cose-decrypt0",
            "open",
            &params(&[("key_id", key_id), ("cose", cose), ("external_aad", "")]),
        ))
    };
    assert_eq!(
        open(&mut host, "derived:2"),
        serde_json::json!({ "error": "", "opened": true, "plaintext": "sealed" })
    );
    assert_eq!(
        open(&mut host, "derived:3"),
        serde_json::json!({ "error": "", "opened": false, "plaintext": "" })
    );
}

#[test]
fn a_new_configuration_keeps_the_derived_keys_and_refuses_their_ids() {
    let mut host = host();
    assert_eq!(
        done(derive(&mut host, "shared secret", ""))["key_id"],
        "derived:1"
    );
    let mut reserved = CoseKeys::default();
    reserved
        .symmetric
        .insert("derived:7".to_string(), key(SEC_256));
    assert_eq!(
        host.set_cose_keys(reserved),
        Err(ReservedKeyId("derived:7".to_string()))
    );
    host.set_cose_keys(CoseKeys::default())
        .expect("an empty configuration");
    assert_eq!(
        done(host.start(
            "sce-host-cose-sign1",
            "sign",
            &params(&[("key_id", "11"), ("payload", ""), ("external_aad", "")]),
        ))["error"],
        "unknown_key",
        "the configured keys were replaced"
    );
    assert_eq!(
        done(host.start(
            "sce-host-cose-encrypt0",
            "seal",
            &params(&[
                ("key_id", "derived:1"),
                ("plaintext", ""),
                ("external_aad", "")
            ]),
        ))["error"],
        "",
        "a derived key outlives the configuration it was derived under"
    );
}

#[test]
fn the_host_holds_no_key_until_it_is_given_one() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let requests: BTreeMap<&str, Params> = BTreeMap::from([
        (
            "sce-host-cose-sign1",
            params(&[("key_id", "11"), ("payload", ""), ("external_aad", "")]),
        ),
        (
            "sce-host-cose-verify1",
            params(&[("trust_set", "wg"), ("cose", ""), ("external_aad", "")]),
        ),
        (
            "sce-host-cose-decrypt0",
            params(&[("key_id", "sec-256"), ("cose", ""), ("external_aad", "")]),
        ),
    ]);
    for (operation, request) in &requests {
        assert_eq!(
            done(host.start(operation, "op", request))["error"],
            "unknown_key",
            "{operation}"
        );
    }
}
