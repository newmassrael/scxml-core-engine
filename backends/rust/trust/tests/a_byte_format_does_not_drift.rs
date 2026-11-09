// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The bytes this crate signs and seals are a contract with every host that checks them: another
// lane, a later version of this one, a relay that never reads them. A change to what is signed
// or how, or to what a key and a nonce are made from, makes every signature and sealed text that
// exists stop verifying, and passes every test that signs and verifies in one process. These
// vectors are the guard: each is a fixed input and the bytes it must give, written down, so that
// a change shows as a difference here and is a decision, not an accident.
//
// The vectors of a change and of the operation that carries it were made again, once, when a
// change of the list became one signed unit (docs/adr/0085, landing 4c): the event domain became
// `sce-trust/event/3`, the one signed event is `record_list.change`, and an event and an operation
// are written as deterministic CBOR (docs/adr/0086, section 9), where the second domain wrote one
// event per part of a change as length-prefixed texts. The sealed text moved with the nonce and
// content domains (`sce-trust/nonce/2`, `sce-trust/content/2`): a write is one of a change's. The
// stamp a write is sealed under, its register and its change's dot, is the bytes it was. They are
// new pins of that decision, made by an encoder written apart from this crate's (deterministic
// CBOR by hand, SHA-256, Ed25519 and XChaCha20-Poly1305 of another library), which gave the
// vectors of the second domain back first; the checkpoint and the entries of the log did not
// move.

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

/// A change of node 1: it writes over the value node 2 wrote of row 1, field 3, at 900, and writes
/// the field anew.
fn change() -> serde_json::Value {
    json!({"by":"1","wallTime":"1000","counter":0,"ops":[
        {"kind":1,"id":1,"field":3,"value":"0","text":"","wallTime":"900","counter":0,"nodeId":"2"},
        {"kind":0,"id":1,"field":3,"value":"10","text":"","wallTime":"0","counter":0,"nodeId":"0"}]})
}

fn link() -> message::Link {
    message::Link {
        seq: 3,
        prev: [7; 32],
        log: 4,
        log_hash: [9; 32],
        context: vec![(2, 5)],
    }
}

/// The operation domain is the second since its link holds the operation's causal context
/// (docs/adr/0085, landing 4b), and it is the CBOR array of the domain, the change's bytes as a
/// byte string and the link as a map, its context a list of records of a node and a number. The id
/// of the first form, which wrote texts with their lengths and no context, was
/// `JInVDbQHgBWrzZxfJ9gmcgelmJ9Abr2ZLLVzY5bzdIY`.
#[test]
fn an_operation_is_signed_and_named_as_it_always_was() {
    let signing = message::signing("home", "record_list.change", &change())
        .unwrap()
        .unwrap();
    let operation = message::operation_bytes(&signing.message, &link());
    assert_eq!(
        hex(&operation),
        "83757363652d74727573742f6f7065726174696f6e2f3258c084717363652d74727573742f6576656e742f3364686f6d65727265636f72645f6c6973742e6368616e6765a462627901636f707382a862696401646b696e6401647465787460656669656c64036576616c756500666e6f646549640267636f756e746572006877616c6c54696d65190384a862696401646b696e6400647465787460656669656c64036576616c75650a666e6f646549640067636f756e746572006877616c6c54696d650067636f756e746572006877616c6c54696d651903e8a562637881a26373657105646e6f646502626d6804636d6868582009090909090909090909090909090909090909090909090909090909090909096373657103647072657658200707070707070707070707070707070707070707070707070707070707070707"
    );
    assert_eq!(
        encoding::encode(&message::operation_id(&operation)),
        "CxkFoklhMKlxvF1aTa-9WQqov8RxmBtoYvYZ6X3GboU"
    );
    assert_eq!(
        encoding::encode(&alice().sign(&operation)),
        "TlMQ5ueEMyOZ5FB100C48CdkEsa2RlcZgriymAHbW97PgZhaHe1RMMM07uxWQr35wGgrW7jI0iZI60T-0g9NBA"
    );
}

#[test]
fn a_public_key_is_written_the_same_way() {
    assert_eq!(
        alice().public().to_text(),
        "iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w"
    );
}

/// A change is the CBOR array of the event domain, the list, the event and its payload, a map of
/// every field `record_change` declares keyed in the order of their encodings (`by`, `ops`,
/// `counter`, `wallTime`), its operations an array of maps of every field `record_op` declares, in
/// the order the change holds them.
#[test]
fn the_bytes_a_change_is_signed_as_do_not_move() {
    let signing = message::signing("home", "record_list.change", &change())
        .unwrap()
        .unwrap();
    assert_eq!(
        hex(&signing.message),
        "84717363652d74727573742f6576656e742f3364686f6d65727265636f72645f6c6973742e6368616e6765a462627901636f707382a862696401646b696e6401647465787460656669656c64036576616c756500666e6f646549640267636f756e746572006877616c6c54696d65190384a862696401646b696e6400647465787460656669656c64036576616c75650a666e6f646549640067636f756e746572006877616c6c54696d650067636f756e746572006877616c6c54696d651903e8"
    );
    assert_eq!(
        encoding::encode(&alice().sign(&signing.message)),
        "FBAQ8-TDg4Dwq5gBTSfxnweTouiBcnChHaDuYkms1dJnoGRDPGBiakMj4x48CD1TJnjXiATcEx4AHnyKSYHwDA"
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
    let change = change();
    let stamp = message::write_stamp(&change, &change["ops"][1]).unwrap();
    assert_eq!(
        hex(&stamp),
        "000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000131",
        "the write's register (1, 3) and its change's dot (1000, 0, node 1)"
    );
    assert_eq!(
        secrecy::seal("home", 2, &[7; 32], &stamp, -42, "buy milk"),
        "~AAAAAjEhBfGkf5VTLxQdIkXvVG5zNJhHbV_WoUPtnSa_zY1trIoG82vyhTeW1J_2ooyqqvKdOwAETDzt2gg"
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
