// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-22.9

//! A request signed by the C++ client, verified by the Rust server (SCE_MESH.md section 22.9;
//! docs/adr/0109, D9): `tests/mesh/https_signature_cross_language.json`, whose signed fields the C++
//! client's signer writes byte for byte (tests/mesh/HttpsSignatureTest.cpp), is taken whole by the
//! verification the Rust server runs under the `key` authenticator, and refused with a covered
//! field changed. The Rust signer writes the same bytes, so the two lanes are one profile.

use std::path::PathBuf;

use sce_rust_crypto::bytes_of_hex;
use sce_rust_crypto::ed25519::KeyPair;
use sce_rust_mesh::https::signature::{
    digest_matches, field_value, key_of_offer, normalize_target_uri, Refusal, RequestSignature,
    RequestSigner, CONTENT_DIGEST_HEADER, KEY_HEADER,
};

/// The shared vector.
fn vector() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/mesh/https_signature_cross_language.json");
    let text = std::fs::read_to_string(&path).expect("the cross-language vector");
    serde_json::from_str(&text).expect("the vector is JSON")
}

/// A list of `[name, value]` pairs as header fields.
fn fields(list: &serde_json::Value) -> Vec<(String, String)> {
    list.as_array()
        .expect("a list of fields")
        .iter()
        .map(|pair| {
            (
                pair[0].as_str().expect("a name").to_string(),
                pair[1].as_str().expect("a value").to_string(),
            )
        })
        .collect()
}

/// The vector's request as the server receives it: its own fields and the signed ones.
fn received(vector: &serde_json::Value) -> Vec<(String, String)> {
    let mut headers = fields(&vector["headers"]);
    headers.extend(fields(&vector["signed"]));
    headers
}

/// The request the C++ client signed verifies under the key it offers, at its own moment, over the
/// target the server reads; its body is the one its digest covers. Red if the Rust verification
/// reads one byte of the signature base otherwise than the C++ signer writes it.
#[test]
fn the_request_the_cpp_client_signed_verifies_at_the_rust_server() {
    let v = vector();
    let target = v["target_uri"].as_str().expect("a target");
    assert_eq!(
        normalize_target_uri(v["url"].as_str().expect("a URL")).as_deref(),
        Some(target)
    );
    let created = v["created"].as_i64().expect("a moment");
    let body = bytes_of_hex(v["body"].as_str().expect("a body")).expect("hex");
    let headers = received(&v);
    let field = |name: &str| field_value(&headers, name);
    assert!(digest_matches(
        &field(CONTENT_DIGEST_HEADER).expect("a digest"),
        &body
    ));
    let offered = key_of_offer(&field(KEY_HEADER).expect("an offered key")).expect("a key");
    assert_eq!(
        offered.to_vec(),
        bytes_of_hex(v["public_key"].as_str().expect("a public key")).expect("hex")
    );
    let signature = RequestSignature::read(&field).expect("a signature of the profile");
    assert_eq!(
        signature.verify(&offered, target, &field, created, 300),
        Ok(())
    );

    let mut acknowledged = headers.clone();
    for (name, value) in acknowledged.iter_mut() {
        if name == "Sce-Mesh-Ack" {
            *value = "4".to_string();
        }
    }
    let changed = |name: &str| field_value(&acknowledged, name);
    assert_eq!(
        signature.verify(&offered, target, &changed, created, 300),
        Err(Refusal::Signature),
        "a changed acknowledgement"
    );
}

/// The Rust signer, given the vector's seed, target, fields and moment, writes exactly the fields
/// the C++ client writes. Red if the two lanes sign two profiles.
#[test]
fn the_rust_signer_writes_what_the_cpp_client_writes() {
    let v = vector();
    let seed: [u8; 32] = bytes_of_hex(v["seed"].as_str().expect("a seed"))
        .expect("hex")
        .try_into()
        .expect("32 bytes");
    let signer = RequestSigner::new(KeyPair::from_seed(&seed));
    let signed = signer
        .sign(
            v["target_uri"].as_str().expect("a target"),
            &fields(&v["headers"]),
            v["created"].as_i64().expect("a moment"),
        )
        .expect("every covered field");
    assert_eq!(signed.to_vec(), fields(&v["signed"]));
}
