// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The bytes this crate signs and seals are a contract with every host that checks them: another
// lane, a later version of this one, a relay that never reads them. A change to what is signed
// or how, or to what a key and a nonce are made from, makes every signature and sealed text that
// exists stop verifying, and passes every test that signs and verifies in one process. These
// vectors are the guard: each is a fixed input and the bytes it must give, written down, so that
// a change shows as a difference here and is a decision, not an accident.

use sce_rust_trust::encoding;
use sce_rust_trust::message;
use sce_rust_trust::secrecy;
use sce_rust_trust::{Certificate, Identity};
use serde_json::json;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn alice() -> Identity {
    Identity::from_seed([1; 32])
}

fn bob() -> Identity {
    Identity::from_seed([2; 32])
}

fn write() -> serde_json::Value {
    json!({"id":1,"field":3,"value":"10","text":"","wallTime":"1000","counter":0,"nodeId":"1"})
}

fn note() -> serde_json::Value {
    json!({"id":1,"field":3,"wallTime":"1000","counter":0,"nodeId":"2","by":"1","at":"2000"})
}

#[test]
fn a_public_key_is_written_the_same_way() {
    assert_eq!(
        alice().public().to_text(),
        "iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w"
    );
}

#[test]
fn the_bytes_a_write_is_signed_as_do_not_move() {
    let signing = message::signing("home", "todo.write", &write())
        .unwrap()
        .unwrap();
    assert_eq!(
        hex(&signing.message),
        "00000000000000117363652d74727573742f6576656e742f310000000000000004686f6d65000000000000000a746f646f2e7772697465000000000000000131000000000000000133000000000000000231300000000000000000000000000000000431303030000000000000000130000000000000000131"
    );
    assert_eq!(
        encoding::encode(&alice().sign(&signing.message)),
        "TvLfTlPvOoWYoTzD2snS56UPCTKkx0bUENgrNaJE8mSRm66YK5EwifYb8d_squIc19KZuUdxGwGisE3Bp5ulBg"
    );
}

#[test]
fn the_bytes_a_note_and_a_floor_are_signed_as_do_not_move() {
    let signing = message::signing("home", "todo.supersede", &note())
        .unwrap()
        .unwrap();
    assert_eq!(
        hex(&signing.message),
        "00000000000000117363652d74727573742f6576656e742f310000000000000004686f6d65000000000000000e746f646f2e737570657273656465000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000132000000000000000131000000000000000432303030"
    );
    assert_eq!(
        hex(&message::note_message("home", &signing.subject, 1, 2000)),
        hex(&signing.message),
        "a note is written again from what a host keeps of it as it was written"
    );
    assert_eq!(
        hex(&message::floor_message("home", 30_000, 7)),
        "00000000000000117363652d74727573742f666c6f6f722f310000000000000004686f6d6500000000000000053330303030000000000000000137"
    );
}

#[test]
fn a_certificate_is_signed_as_it_always_was() {
    let certificate = Certificate::issue(&alice(), "home", 2, 2, bob().public(), 1, 500);
    assert_eq!(
        certificate.to_json(),
        r#"{"calendar":"home","from":"500","issuer":"iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w","key":"gTl3Dqh9F19Wo1Rmw0x-zMuNipG07jeiXfYPW4_Js5Q","node":"2","role":2,"serial":"1","sig":"tf36tv3R8JuPmlHkRtrre_88ahh3Qonx-TbhGom4KAT7GZqofYrV3_u6uY38LwqUDdqVm5gTggdP9O-Zj5YQAA"}"#
    );
}

#[test]
fn sealed_content_and_a_wrapped_key_are_written_the_same_way() {
    let stamp = message::write_stamp(&write()).unwrap();
    assert_eq!(
        hex(&stamp),
        "000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000131"
    );
    assert_eq!(
        secrecy::seal("home", 2, &[7; 32], &stamp, -42, "buy milk"),
        "~AAAAAnNxtIUL1OyQ6GF4hwegfPTIqgLpiEpReyh7OBrx6XkO"
    );
    let shared = alice().agree(&bob().public()).unwrap();
    assert_eq!(
        hex(&shared),
        "4181d7302557342bdb6d061c4b1eebea828ecb625c3368b7111680793307220b"
    );
    assert_eq!(
        Some(shared),
        bob().agree(&alice().public()),
        "both ends agree on it"
    );
    assert_eq!(
        secrecy::wrap(
            "home",
            &shared,
            &alice().public(),
            &bob().public(),
            2,
            &[9; 32]
        ),
        "6qcEeMiF4TB7xBNwzD3SVc4fixcWZKA1b_DY843nMgTye24qRZZcNPAaf2LZMZIK"
    );
}
