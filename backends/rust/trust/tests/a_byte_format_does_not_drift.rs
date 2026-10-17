// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The bytes this crate signs and seals are a contract with every host that checks them: another
// lane, a later version of this one, a relay that never reads them. A change to what is signed
// or how, or to what a key and a nonce are made from, makes every signature and sealed text that
// exists stop verifying, and passes every test that signs and verifies in one process. These
// vectors are the guard: each is a fixed input and the bytes it must give, written down, so that
// a change shows as a difference here and is a decision, not an accident.

use sce_rust_trust::checkpoint::{Checkpoint, Head};
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Log, Profile};
use sce_rust_trust::message;
use sce_rust_trust::secrecy;
use sce_rust_trust::Identity;
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
    json!({"id":1,"field":3,"wallTime":"1000","counter":0,"nodeId":"2","by":"1"})
}

fn link() -> message::Link {
    message::Link {
        seq: 3,
        prev: [7; 32],
        log: 4,
        log_hash: [9; 32],
    }
}

#[test]
fn an_operation_is_signed_and_named_as_it_always_was() {
    let signing = message::signing("home", "todo.write", &write())
        .unwrap()
        .unwrap();
    let operation = message::operation_bytes(&signing.message, &link());
    assert_eq!(
        hex(&operation),
        "00000000000000157363652d74727573742f6f7065726174696f6e2f31000000000000007900000000000000117363652d74727573742f6576656e742f310000000000000004686f6d65000000000000000a746f646f2e7772697465000000000000000131000000000000000133000000000000000231300000000000000000000000000000000431303030000000000000000130000000000000000131000000000000000133000000000000002b42776348427763484277634842776348427763484277634842776348427763484277634842776348427763000000000000000134000000000000002b43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b"
    );
    assert_eq!(
        encoding::encode(&message::operation_id(&operation)),
        "B4xt_c5Ap-C332dYB2GKHI0xMM2sOIMUZmrRkvkWo1s"
    );
    assert_eq!(
        encoding::encode(&alice().sign(&operation)),
        "zylbw8cFt5qwJInbIYV9yiO6wqCInc-hRPYbmqG3i6odRU5mNMyeBlVxxnUJsc3RFYj_7IY0iFaOlmr5fk9yAA"
    );
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
fn the_bytes_a_note_is_signed_as_do_not_move() {
    let signing = message::signing("home", "todo.supersede", &note())
        .unwrap()
        .unwrap();
    assert_eq!(
        hex(&signing.message),
        "00000000000000117363652d74727573742f6576656e742f310000000000000004686f6d65000000000000000e746f646f2e737570657273656465000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000132000000000000000131"
    );
}

fn checkpoint() -> Checkpoint {
    let mut checkpoint = Checkpoint {
        list: "home".to_string(),
        device: 2,
        log: 3,
        log_hash: [4; 32],
        before: 30_000,
        digest: 7,
        heads: vec![
            Head {
                node: 1,
                seq: 5,
                id: [6; 32],
            },
            Head {
                node: 2,
                seq: 2,
                id: [8; 32],
            },
        ],
        signature: [0; 64],
    };
    checkpoint.signature = bob().sign(&checkpoint.signing_bytes());
    checkpoint
}

/// What a member signs of what its copy holds, and how it travels.
#[test]
fn a_checkpoint_is_signed_and_written_as_it_always_was() {
    let checkpoint = checkpoint();
    assert_eq!(
        hex(&checkpoint.signing_bytes()),
        "00000000000000167363652d74727573742f636865636b706f696e742f310000000000000004686f6d65000000000000000132000000000000000133000000000000002b4241514542415145424151454241514542415145424151454241514542415145424151454241514542415100000000000000053330303030000000000000000137000000000000000132000000000000000131000000000000000135000000000000002b42675947426759474267594742675947426759474267594742675947426759474267594742675947426759000000000000000132000000000000000132000000000000002b43416749434167494341674943416749434167494341674943416749434167494341674943416749434167"
    );
    assert_eq!(
        encoding::encode(&checkpoint.signature),
        "c4JzEw-wPKwIb2G5FR_juV9_TelC_4psND1-PPXSxUBvNslhbgDpkN2M71qK1BXp5V6rXIXqbJi-BWXBZDs4DQ"
    );
    assert_eq!(
        checkpoint.to_json(),
        r#"{"before":"30000","device":"2","digest":"7","heads":[["1","5","BgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgYGBgY"],["2","2","CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg"]],"list":"home","mh":"3","mhh":"BAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQ","sig":"c4JzEw-wPKwIb2G5FR_juV9_TelC_4psND1-PPXSxUBvNslhbgDpkN2M71qK1BXp5V6rXIXqbJi-BWXBZDs4DQ"}"#
    );
}

#[test]
fn the_entries_of_the_membership_log_are_signed_and_named_as_they_always_were() {
    let genesis = Log::genesis_draft(
        "home",
        Body::Genesis {
            administrators: vec![alice().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&alice())
    .finish();
    assert_eq!(
        genesis.to_json(),
        r#"{"body":{"administrators":["iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w"],"kind":"genesis","profile":"untrusted","quorum":1},"index":"0","list":"home","prev":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","signatures":[["iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w","CvLbFDHup1OP4T_0XwUoVjFrcjndaK_r7GEX_KGOnJMa0CNwQ4E_QNpvahk9nhEUQslj43cL_XX98_9lAWrwCw"]]}"#
    );
    assert_eq!(
        hex(&genesis.signing_bytes()),
        "00000000000000167363652d74727573742f6d656d626572736869702f310000000000000004686f6d65000000000000000130000000000000002b41414141414141414141414141414141414141414141414141414141414141414141414141414141414141000000000000000767656e657369730000000000000001310000000000000009756e74727573746564000000000000002b696f6a6a3358514a385a583955747374504c70646373706e436238646c4249623833534941625150623177"
    );
    assert_eq!(
        encoding::encode(&genesis.hash()),
        "25wtRx2B5zlZ0VD19DLiMaDr7onv8f8-fbJ7yWLVnIU"
    );

    let mut log = Log::open("home");
    log.append(&genesis).unwrap();
    let grant_with = |final_seq| {
        log.draft(Body::Grant {
            node: 2,
            role: 2,
            key: bob().public(),
            final_seq,
        })
        .sign(&alice())
        .finish()
    };
    let plain = grant_with(None);
    assert_eq!(
        plain.to_json(),
        r#"{"body":{"final_seq":null,"key":"gTl3Dqh9F19Wo1Rmw0x-zMuNipG07jeiXfYPW4_Js5Q","kind":"grant","node":"2","role":2},"index":"1","list":"home","prev":"25wtRx2B5zlZ0VD19DLiMaDr7onv8f8-fbJ7yWLVnIU","signatures":[["iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w","IKLw6ZY3Yy46yuZTr0xkcMb00MvhRGetlmm9v_Wpp1MP68Brxm1nnU0O9zlLh4V5CsUAC0BZviRkI1yKZnunDQ"]]}"#
    );
    assert_eq!(
        encoding::encode(&plain.hash()),
        "gSe3pLI5kAJDSJGGcKVfZBkFt138Cz9WhpZaX6RU93k"
    );
    let cut = grant_with(Some(2));
    assert_eq!(
        cut.to_json(),
        r#"{"body":{"final_seq":"2","key":"gTl3Dqh9F19Wo1Rmw0x-zMuNipG07jeiXfYPW4_Js5Q","kind":"grant","node":"2","role":2},"index":"1","list":"home","prev":"25wtRx2B5zlZ0VD19DLiMaDr7onv8f8-fbJ7yWLVnIU","signatures":[["iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w","lqtXbVLN0migRRLG5QPTVTOA-r6JwUwiFcSlg-ZfBIEL_UN3g78N5YeIp8T3Dz-jgsD3TE4CpYKdh0cBVY5mCg"]]}"#
    );
    assert_eq!(
        encoding::encode(&cut.hash()),
        "QoOmOYYogq4LMskNDuXYjW9QtRkS4X5__-Aa-Ki_ozk"
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
