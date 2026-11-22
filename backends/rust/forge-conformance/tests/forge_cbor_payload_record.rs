// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// docs/adr/0109, D9 — a record signed by name has two writers: the runtime's
// record codec (`sce_rust_runtime::record_cbor`), which a machine writes the
// record with from its payload JSON, and the codec ADR 0101 prints for the
// same schema, which a peer that reads the signed record is built from. Each
// case of the corpus (`tests/forge/conformance/numerical_reference.json`,
// `codec_cbor_payload_record`) is written by both to its one set of bytes and
// read by both back to its value, and each side reads what the other wrote, so
// the two cannot drift apart unseen.
//
// The printed side is a map of the record under key 0
// (`codec_cbor_payload_note.scxml`, `codec_cbor_payload_leaves.scxml`, beside
// the composed corpus), whose value is `a1 00` and the record's own map.
#![cfg(feature = "alloc")]

// Each generated unit is a sibling module, as the codecs name them
// (`use super::<module>;`). A unit's API is offered whether or not this test
// reads it.
#[allow(dead_code)]
mod cbor_composed_level {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_level.rs"));
}

#[allow(dead_code)]
mod cbor_composed_point {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_point.rs"));
}

#[allow(dead_code)]
mod cbor_composed_kind {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_kind.rs"));
}

#[allow(dead_code)]
mod cbor_composed_card {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_card.rs"));
}

#[allow(dead_code)]
mod cbor_composed_batch {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_batch.rs"));
}

#[allow(dead_code)]
mod cbor_composed_choice {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_choice.rs"));
}

#[allow(dead_code)]
mod cbor_payload_leaves {
    include!(concat!(env!("OUT_DIR"), "/cbor_payload_leaves.rs"));
}

#[allow(dead_code)]
mod codec_cbor_payload_leaves {
    include!(concat!(env!("OUT_DIR"), "/codec_cbor_payload_leaves.rs"));
}

#[allow(dead_code)]
mod schema_cose_note {
    include!(concat!(env!("OUT_DIR"), "/schema_cose_note.rs"));
}

#[allow(dead_code)]
mod codec_cbor_payload_note {
    include!(concat!(env!("OUT_DIR"), "/codec_cbor_payload_note.rs"));
}

use codec_cbor_payload_leaves::CodecCborPayloadLeaves;
use codec_cbor_payload_note::CodecCborPayloadNote;
use sce_forge_runtime::codec::SceCursor;
use sce_rust_runtime::record_cbor;
use serde_json::Value;

/// The head of the printed codec's map of one entry and the key of that entry,
/// which the record's own map follows.
const UNDER_KEY_0: [u8; 2] = [0xa1, 0x00];

/// The corpus's entry of the records.
fn corpus() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/forge/conformance/numerical_reference.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let reference: Value = serde_json::from_str(&text).expect("the reference is JSON");
    reference["codecs"]["codec_cbor_payload_record"].clone()
}

fn cases(corpus: &Value) -> &[Value] {
    let cases = corpus["cases"]
        .as_array()
        .expect("the corpus holds its cases");
    assert!(
        cases.len() >= 6,
        "the corpus holds a case of each record and edge"
    );
    cases
}

fn bytes_of(v: &Value) -> Vec<u8> {
    v.as_array()
        .expect("bytes are an array")
        .iter()
        .map(|b| {
            b.as_u64()
                .and_then(|b| u8::try_from(b).ok())
                .expect("a byte")
        })
        .collect()
}

/// The record a case is of, and the description of its type.
fn record_of<'c>(corpus: &'c Value, case: &'c Value) -> (&'c str, &'c str) {
    let record = case["record"].as_str().expect("a case names its record");
    let description = corpus["records"][record]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("the corpus describes the record {record}"));
    (record, description)
}

/// The bytes of a byte string a payload spells as its Latin-1 text.
fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).expect("a Latin-1 character"))
        .collect()
}

/// The record `encoded` holds, read by the printed codec of `record` and written
/// by it again: the record's own map, after the head and the key the printed
/// map adds. A note's fields are held to `payload` as the printed codec reads
/// them.
fn printed(record: &str, encoded: &[u8], payload: &Value) -> Vec<u8> {
    let mut framed = UNDER_KEY_0.to_vec();
    framed.extend_from_slice(encoded);
    let mut cursor = SceCursor::new(&framed);
    let written = match record {
        "note" => {
            let read = CodecCborPayloadNote::decode(&mut cursor)
                .unwrap_or_else(|e| panic!("the printed codec reads the note: {e:?}"));
            assert_eq!(
                u64::from(read.note.serial),
                payload["serial"].as_u64().expect("serial")
            );
            assert_eq!(
                read.note.owner.as_str(),
                payload["owner"].as_str().expect("owner")
            );
            assert_eq!(
                read.note.blob,
                latin1(payload["blob"].as_str().expect("blob")).as_slice()
            );
            assert_eq!(
                read.note.active,
                payload["active"].as_bool().expect("active")
            );
            assert_eq!(
                i64::from(read.note.offset),
                payload["offset"].as_i64().expect("offset")
            );
            read.encode_to_vec()
                .unwrap_or_else(|e| panic!("the printed codec writes the note: {e:?}"))
        }
        "leaves" => {
            let read = CodecCborPayloadLeaves::decode(&mut cursor)
                .unwrap_or_else(|e| panic!("the printed codec reads the record: {e:?}"));
            assert_eq!(
                u64::from(read.leaves.tiny),
                payload["tiny"].as_u64().expect("tiny")
            );
            assert_eq!(read.leaves.on, payload["on"].as_bool().expect("on"));
            read.encode_to_vec()
                .unwrap_or_else(|e| panic!("the printed codec writes the record: {e:?}"))
        }
        other => panic!("the corpus names a record {other} this test has no codec of"),
    };
    assert_eq!(
        cursor.remaining(),
        0,
        "the printed codec read the whole map"
    );
    assert_eq!(
        &written[..2],
        &UNDER_KEY_0,
        "the printed map holds one entry under key 0"
    );
    written[2..].to_vec()
}

#[test]
fn the_record_codec_writes_and_reads_each_case_as_the_corpus_does() {
    let corpus = corpus();
    for case in cases(&corpus) {
        let note = case["note"].as_str().unwrap_or("");
        let (_, description) = record_of(&corpus, case);
        let encoded = bytes_of(&case["encoded"]);
        let written = record_cbor::encode(&case["payload"].to_string(), description)
            .unwrap_or_else(|e| panic!("{note}: the record codec writes it: {e}"));
        assert_eq!(written, encoded, "{note}: the record codec's bytes");
        let read = record_cbor::decode(&encoded, description)
            .unwrap_or_else(|e| panic!("{note}: the record codec reads it: {e}"));
        let read: Value = serde_json::from_str(&read).expect("a record is JSON");
        assert_eq!(read, case["payload"], "{note}: the record codec's value");
    }
}

#[test]
fn the_printed_codec_writes_and_reads_each_case_as_the_corpus_does() {
    let corpus = corpus();
    for case in cases(&corpus) {
        let note = case["note"].as_str().unwrap_or("");
        let (record, _) = record_of(&corpus, case);
        let encoded = bytes_of(&case["encoded"]);
        assert_eq!(
            printed(record, &encoded, &case["payload"]),
            encoded,
            "{note}: the printed codec's bytes"
        );
    }
}

#[test]
fn each_writer_writes_what_the_other_reads() {
    let corpus = corpus();
    for case in cases(&corpus) {
        let note = case["note"].as_str().unwrap_or("");
        let (record, description) = record_of(&corpus, case);
        // The record codec writes; the printed codec reads it and writes the
        // same bytes again.
        let by_runtime = record_cbor::encode(&case["payload"].to_string(), description)
            .unwrap_or_else(|e| panic!("{note}: the record codec writes it: {e}"));
        assert_eq!(
            printed(record, &by_runtime, &case["payload"]),
            by_runtime,
            "{note}: the printed codec reads the record codec's bytes"
        );
        // The printed codec writes; the record codec reads it to the value.
        let by_printed = printed(record, &bytes_of(&case["encoded"]), &case["payload"]);
        let read = record_cbor::decode(&by_printed, description)
            .unwrap_or_else(|e| panic!("{note}: the record codec reads the printed bytes: {e}"));
        let read: Value = serde_json::from_str(&read).expect("a record is JSON");
        assert_eq!(read, case["payload"], "{note}: the record codec's value");
    }
}
