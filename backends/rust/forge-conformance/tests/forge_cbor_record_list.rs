// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// docs/adr/0085, Landing order step 3a — a CBOR map whose entries hold a
// bounded list of records, generated for Rust from
// `sce-build/tests/fixtures/cbor_codec/codec_cbor_record_list.scxml` and held
// to the bytes RFC 8949 and docs/adr/0086 section 9.2 fix for its values: each
// list a definite-length array, each record a map of every field keyed by its
// id as text and written with the shorter key first, an absent list not
// written. Each record is the payload type of the event schema's own unit
// (`cbor_causal_entry`), generated beside the codec. A read refuses a list
// past its bound and a malformed record, each leaving the cursor where it was;
// a list past its bound cannot be made.
//
// The same values and bytes are the cross-lane oracle's
// (`tests/forge/conformance/numerical_reference.json`, `codec_cbor_record_list`),
// which every lane that lowers the codec encodes byte for byte.
#![cfg(feature = "alloc")]

mod cbor_causal_entry {
    include!(concat!(env!("OUT_DIR"), "/cbor_causal_entry.rs"));
}

mod codec_cbor_record_list {
    include!(concat!(env!("OUT_DIR"), "/codec_cbor_record_list.rs"));
}

use cbor_causal_entry::CborCausalEntryPayload as Entry;
use codec_cbor_record_list::{CodecCborRecordList, CodecCborRecordListOwned};
use sce_forge_runtime::codec::{Bounded, BoundedList, CodecError, Heap, SceCursor, SceString};

const ID: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];
const A: &str = "node-a";
const B: &str = "node-b";

fn entry(author: &str, seq: u64) -> Entry {
    Entry {
        author: SceString::from_text(author).expect("within string[16]"),
        seq,
    }
}

/// `seen` holds one record and `context` two.
fn value() -> CodecCborRecordList<'static> {
    CodecCborRecordList {
        id: (&ID).into(),
        source: Bounded::new("n").expect("within the source's bound"),
        seen: BoundedList::from_slice(&[entry(A, 1)]).expect("within the bound of two"),
        context: Some(
            BoundedList::from_slice(&[entry(A, 2), entry(B, 300)])
                .expect("within the bound of three"),
        ),
    }
}

/// One record's map: `seq` (three bytes) before `author` (six), the order of
/// their keys' encodings (RFC 8949 section 4.2.1), whatever the order the
/// schema declares them in; `seq` is the number's head and bytes as written,
/// and the author a text of six bytes.
fn record(author: &str, seq: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xa2, 0x63, b's', b'e', b'q'];
    bytes.extend_from_slice(seq);
    bytes.extend_from_slice(&[0x66, b'a', b'u', b't', b'h', b'o', b'r', 0x66]);
    bytes.extend_from_slice(author.as_bytes());
    bytes
}

/// `{0: h'000102…0f', 1: "n", 3: [A 1], 4: [A 2, B 300]}`.
fn expected() -> Vec<u8> {
    let mut bytes = vec![0xa4, 0x00, 0x50];
    bytes.extend_from_slice(&ID);
    bytes.extend_from_slice(&[0x01, 0x61, b'n']);
    bytes.extend_from_slice(&[0x03, 0x81]);
    bytes.extend_from_slice(&record(A, &[0x01]));
    bytes.extend_from_slice(&[0x04, 0x82]);
    bytes.extend_from_slice(&record(A, &[0x02]));
    bytes.extend_from_slice(&record(B, &[0x19, 0x01, 0x2c]));
    bytes
}

/// The frame of `expected` with `seen` written as `items`, a count head and
/// its records.
fn with_seen(items: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xa3, 0x00, 0x50];
    bytes.extend_from_slice(&ID);
    bytes.extend_from_slice(&[0x01, 0x61, b'n', 0x03]);
    bytes.extend_from_slice(items);
    bytes
}

fn decode(bytes: &[u8]) -> Result<CodecCborRecordList<'_>, CodecError> {
    CodecCborRecordList::decode(&mut SceCursor::new(bytes))
}

#[test]
fn a_list_of_records_is_written_as_an_array_of_maps() {
    assert_eq!(value().encode_to_vec(), expected());
}

#[test]
fn what_was_written_is_read_back() {
    assert_eq!(decode(&expected()), Ok(value()));
}

#[test]
fn an_absent_list_is_not_written_and_an_empty_one_is_an_empty_array() {
    let mut v = value();
    v.context = None;
    v.seen = BoundedList::new();
    let want = with_seen(&[0x80]);
    assert_eq!(v.encode_to_vec(), want);
    assert_eq!(decode(&want), Ok(v));
}

#[test]
fn a_record_is_read_with_its_keys_in_any_order_and_an_unknown_one_skipped() {
    // `author` before `seq`, and a key `x` the schema does not declare.
    let mut items = vec![0x81, 0xa3, 0x66, b'a', b'u', b't', b'h', b'o', b'r', 0x66];
    items.extend_from_slice(A.as_bytes());
    items.extend_from_slice(&[0x61, b'x', 0x82, 0x01, 0x02, 0x63, b's', b'e', b'q', 0x01]);
    let mut v = value();
    v.context = None;
    assert_eq!(decode(&with_seen(&items)), Ok(v));
}

#[test]
fn a_list_past_its_bound_is_refused_before_a_record_is_read() {
    // Three records where `seen` holds at most two.
    let mut items = vec![0x83];
    for seq in 1..=3u8 {
        items.extend_from_slice(&record(A, &[seq]));
    }
    let bytes = with_seen(&items);
    let mut cursor = SceCursor::new(&bytes);
    assert_eq!(
        CodecCborRecordList::decode(&mut cursor),
        Err(CodecError::CborOutOfRange)
    );
    assert_eq!(cursor.remaining(), bytes.len(), "the cursor moved");
    // At the bound it is read.
    let mut two = vec![0x82];
    two.extend_from_slice(&record(A, &[0x01]));
    two.extend_from_slice(&record(B, &[0x02]));
    assert!(decode(&with_seen(&two)).is_ok());
}

#[test]
fn a_list_past_its_bound_cannot_be_made() {
    let three = [entry(A, 1), entry(A, 2), entry(A, 3)];
    assert!(BoundedList::<Entry, 2>::from_slice(&three).is_err());
    assert!(BoundedList::<Entry, 2>::from_slice(&three[..2]).is_ok());
}

#[test]
fn a_malformed_record_is_refused_and_leaves_the_cursor_where_it_was() {
    let refused = |items: &[u8], error: CodecError| {
        let bytes = with_seen(items);
        let mut cursor = SceCursor::new(&bytes);
        assert_eq!(
            CodecCborRecordList::decode(&mut cursor),
            Err(error),
            "{bytes:02x?}"
        );
        assert_eq!(
            cursor.remaining(),
            bytes.len(),
            "the cursor moved on {error:?}"
        );
    };

    // A record without `seq`.
    let mut missing = vec![0x81, 0xa1, 0x66, b'a', b'u', b't', b'h', b'o', b'r', 0x66];
    missing.extend_from_slice(A.as_bytes());
    refused(&missing, CodecError::CborRequiredKeyMissing);

    // `seq` given twice.
    let mut twice = vec![0x81, 0xa3, 0x63, b's', b'e', b'q', 0x01];
    twice.extend_from_slice(&record(A, &[0x02])[1..]);
    refused(&twice, CodecError::CborMalformed);

    // An author of seventeen bytes, past its bound of sixteen.
    let mut long = vec![0x81, 0xa2, 0x63, b's', b'e', b'q', 0x01];
    long.extend_from_slice(&[0x66, b'a', b'u', b't', b'h', b'o', b'r', 0x71]);
    long.extend_from_slice(&[b'a'; 17]);
    refused(&long, CodecError::CborOutOfRange);

    // An item that is no map, and an array of indefinite length.
    refused(&[0x81, 0x01], CodecError::CborMalformed);
    refused(&[0x9f, 0xff], CodecError::CborMalformed);
}

#[test]
fn the_owned_form_holds_the_lists_and_writes_the_same_bytes() {
    let owned: CodecCborRecordListOwned<Heap> = value()
        .try_into_owned_in::<Heap>()
        .expect("every entry is within its bound");
    assert_eq!(owned.as_borrowed(), value());
    assert_eq!(owned.encode_to_vec(), expected());
}
