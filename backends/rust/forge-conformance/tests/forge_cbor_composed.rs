// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// docs/adr/0101, Landing order step 4a — a CBOR map of an entry of every
// constructor of the language, generated for Rust from
// `tests/forge/conformance/codec_cbor_composed/codec_cbor_composed.scxml` with
// the enums, records and union it reaches, and held to the corpus
// (`tests/forge/conformance/numerical_reference.json`, `codec_cbor_composed`):
// each case written in its one form and read back, each variant encoding read
// to its value and written again in the one form, and each reject refused by
// the name of its class (section 3.3), leaving the cursor where it was. A
// value its types hold and no encoding carries is refused before a byte is
// written.
//
// A value is read from the corpus as the tree of docs/SCE_TEST_PROFILE.md
// section 12.5 the corpus writes it in: an integer the text of its digits, a
// byte string its upper-case hex, an enum the name of its variant (or the
// digits of a number an open enum does not name), a float a JSON number.
#![cfg(feature = "alloc")]

// Each generated unit is a sibling module, as the codec names it
// (`use super::<module>;`). A unit's API is offered whether or not this test
// reads it (the union's tag, an enum's accessors), as the numerical harness's
// module of fixtures allows.
#[allow(dead_code)]
mod cbor_composed_level {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_level.rs"));
}

#[allow(dead_code)]
mod cbor_composed_color {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_color.rs"));
}

#[allow(dead_code)]
mod cbor_composed_code {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_code.rs"));
}

#[allow(dead_code)]
mod cbor_composed_point {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_point.rs"));
}

#[allow(dead_code)]
mod cbor_composed_reading {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_reading.rs"));
}

#[allow(dead_code)]
mod cbor_composed_revised {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_revised.rs"));
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
mod cbor_composed_kind {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_kind.rs"));
}

#[allow(dead_code)]
mod cbor_composed_choice {
    include!(concat!(env!("OUT_DIR"), "/cbor_composed_choice.rs"));
}

#[allow(dead_code)]
mod codec_cbor_composed {
    include!(concat!(env!("OUT_DIR"), "/codec_cbor_composed.rs"));
}

use cbor_composed_batch::CborComposedBatchPayload as Batch;
use cbor_composed_card::CborComposedCardPayload as Card;
use cbor_composed_choice::CborComposedChoice as Choice;
use cbor_composed_code::CborComposedCode as Code;
use cbor_composed_color::CborComposedColor as Color;
use cbor_composed_level::CborComposedLevel as Level;
use cbor_composed_point::CborComposedPointPayload as Point;
use cbor_composed_reading::CborComposedReadingPayload as Reading;
use cbor_composed_revised::CborComposedRevisedPayload as Revised;
use codec_cbor_composed::{CodecCborComposed, CodecCborComposedOwned};
use sce_forge_runtime::codec::{
    Bounded, BoundedList, CodecError, Heap, SceCursor, SceString, VecSink,
};
use serde_json::Value;

/// The corpus's entry of the codec.
fn corpus() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/forge/conformance/numerical_reference.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let reference: Value = serde_json::from_str(&text).expect("the reference is JSON");
    reference["codecs"]["codec_cbor_composed"].clone()
}

/// The rows of the corpus's entry under `key`.
fn rows<'c>(corpus: &'c Value, key: &str) -> &'c [Value] {
    corpus[key]
        .as_array()
        .unwrap_or_else(|| panic!("the corpus holds its {key}"))
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

/// An integer, written as the text of its digits.
fn int<T: TryFrom<i128>>(v: &Value) -> T {
    let digits = v.as_str().expect("an integer is the text of its digits");
    let n: i128 = digits.parse().expect("the text of an integer's digits");
    T::try_from(n)
        .ok()
        .unwrap_or_else(|| panic!("{digits} is within its type"))
}

/// A byte string, written as its upper-case hex.
fn hex(v: &Value) -> Vec<u8> {
    let text = v.as_str().expect("a byte string is its hex");
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("two hex digits"))
        .collect()
}

/// `v`, or `None` where it is absent: a field or an entry left out reads as
/// `null`.
fn present(v: &Value) -> Option<&Value> {
    (!v.is_null()).then_some(v)
}

/// A list of at most `N` elements, each read by `each`.
fn list<T, const N: usize>(v: &Value, each: impl Fn(&Value) -> T) -> BoundedList<T, N> {
    BoundedList::try_from_iter(v.as_array().expect("a list is an array").iter().map(each))
        .expect("the case is within the list's bound")
}

/// The variant of a closed enum named `name`, among the values `from` makes of
/// each number of the carrier.
fn closed<C, E: Copy>(
    name: &Value,
    numbers: impl Iterator<Item = C>,
    from: fn(C) -> Option<E>,
    named: fn(E) -> &'static str,
) -> E {
    let name = name.as_str().expect("an enum is the name of its variant");
    numbers
        .filter_map(from)
        .find(|e| named(*e) == name)
        .unwrap_or_else(|| panic!("no variant is named {name}"))
}

fn level(v: &Value) -> Level {
    closed(
        v,
        i8::MIN..=i8::MAX,
        Level::from_underlying,
        Level::declared_name,
    )
}

fn color(v: &Value) -> Color {
    closed(
        v,
        u8::MIN..=u8::MAX,
        Color::from_underlying,
        Color::declared_name,
    )
}

/// A value of the open enum: the variant of its name, or the number its
/// digits write, which no variant needs to name.
fn code(v: &Value) -> Code {
    let text = v.as_str().expect("an enum is a name or digits");
    match text.parse::<u16>() {
        Ok(number) => Code::from_underlying(number),
        Err(_) => (u16::MIN..=u16::MAX)
            .map(Code::from_underlying)
            .find(|c| c.declared_name() == Some(text))
            .unwrap_or_else(|| panic!("no variant is named {text}")),
    }
}

fn point(v: &Value) -> Point {
    Point {
        x: int(&v["x"]),
        y: int(&v["y"]),
    }
}

fn reading(v: &Value) -> Reading {
    Reading {
        samples: list(&v["samples"], int::<u8>),
        label: present(&v["label"]).map(|t| {
            SceString::from_text(t.as_str().expect("a text")).expect("within the label's bound")
        }),
        at: point(&v["at"]),
        color: color(&v["color"]),
    }
}

/// A record of the schema that grew, every field of its second revision
/// written, `memo` where it is present.
fn revised(v: &Value) -> Revised {
    Revised {
        id: int(&v["id"]),
        gain: int(&v["gain"]),
        mode: level(&v["mode"]),
        label: SceString::from_text(v["label"].as_str().expect("a text"))
            .expect("within the label's bound"),
        tags: list(&v["tags"], int::<u8>),
        memo: present(&v["memo"]).map(|t| {
            SceString::from_text(t.as_str().expect("a text")).expect("within the memo's bound")
        }),
    }
}

/// A value of the union: an object of one member, named by its case.
fn choice(v: &Value) -> Choice {
    let object = v.as_object().expect("a union is an object");
    assert_eq!(object.len(), 1, "a union is an object of one member: {v}");
    let (case, record) = object.iter().next().expect("one member");
    match case.as_str() {
        "idle" => Choice::Idle,
        "card" => Choice::Card(Card {
            token: hex(&record["token"]),
            amount: int(&record["amount"]),
        }),
        "batch" => Choice::Batch(Batch {
            items: list(&record["items"], int::<u16>),
        }),
        other => panic!("no case is named {other}"),
    }
}

/// The value the corpus writes as `v`.
fn value(v: &Value) -> CodecCborComposed<'_> {
    CodecCborComposed {
        small: int(&v["small"]),
        wide: int(&v["wide"]),
        ratio: v["ratio"].as_f64().expect("a float is a number") as f32,
        precise: v["precise"].as_f64().expect("a float is a number"),
        level: level(&v["level"]),
        color: color(&v["color"]),
        code: code(&v["code"]),
        octets: list(&v["octets"], int::<u8>),
        sparse: list(&v["sparse"], |e| present(e).map(int::<u16>)),
        grid: list(&v["grid"], |row| list(row, int::<u8>)),
        flags: list(&v["flags"], |b| b.as_bool().expect("a truth value")),
        reading: reading(&v["reading"]),
        spare: present(&v["spare"]).map(point),
        choice: choice(&v["choice"]),
        choices: list(&v["choices"], choice),
        maybe: present(&v["maybe"]).map(choice),
        note: present(&v["note"])
            .map(|t| Bounded::new(t.as_str().expect("a text")).expect("within the note's bound")),
        revised: present(&v["revised"]).map(revised),
    }
}

/// The refusal the corpus names `name` (docs/adr/0101, section 3.3).
fn refusal(name: &Value) -> CodecError {
    match name.as_str().expect("a refusal is named") {
        "need-more-bytes" => CodecError::NeedMoreBytes,
        "invalid-utf8" => CodecError::InvalidUtf8,
        "undeclared-enum-value" => CodecError::UndeclaredEnumValue,
        "cbor-malformed" => CodecError::CborMalformed,
        "cbor-wrong-type" => CodecError::CborWrongType,
        "cbor-duplicate-key" => CodecError::CborDuplicateKey,
        "cbor-unknown-variant" => CodecError::CborUnknownVariant,
        "cbor-union-entries" => CodecError::CborUnionEntries,
        "cbor-required-key-missing" => CodecError::CborRequiredKeyMissing,
        "cbor-too-deep" => CodecError::CborTooDeep,
        "cbor-out-of-range" => CodecError::CborOutOfRange,
        other => panic!("no refusal of a CBOR decode is named {other}"),
    }
}

fn encoded(value: &CodecCborComposed<'_>) -> Vec<u8> {
    value
        .encode_to_vec()
        .expect("a value the corpus writes has an encoding")
}

#[test]
fn every_case_is_written_in_its_one_form_and_read_back() {
    let corpus = corpus();
    let cases = rows(&corpus, "cases");
    assert_eq!(cases.len(), 3, "the smallest, the largest and the vectors");
    for (i, case) in cases.iter().enumerate() {
        let want = value(&case["decoded"]);
        let bytes = bytes_of(&case["encoded"]);
        assert_eq!(encoded(&want), bytes, "case {i}: {}", case["note"]);
        let mut cursor = SceCursor::new(&bytes);
        assert_eq!(
            CodecCborComposed::decode(&mut cursor),
            Ok(want),
            "case {i}: {}",
            case["note"]
        );
        assert_eq!(cursor.remaining(), 0, "case {i}: read whole");
    }
}

#[test]
fn a_variant_encoding_is_read_and_written_again_in_its_one_form() {
    let corpus = corpus();
    let variants = rows(&corpus, "variants");
    assert!(!variants.is_empty());
    for variant in variants {
        let why = &variant["why"];
        let bytes = bytes_of(&variant["encoded"]);
        let read = CodecCborComposed::decode(&mut SceCursor::new(&bytes))
            .unwrap_or_else(|e| panic!("{why}: refused with {e:?}"));
        assert_eq!(read, value(&variant["decoded"]), "{why}");
        assert_eq!(encoded(&read), bytes_of(&variant["canonical"]), "{why}");
    }
}

#[test]
fn what_is_no_value_of_the_type_is_refused_by_the_name_of_its_class() {
    let corpus = corpus();
    let rejects = rows(&corpus, "rejects");
    assert!(!rejects.is_empty());
    for reject in rejects {
        let why = &reject["why"];
        let bytes = bytes_of(&reject["encoded"]);
        let mut cursor = SceCursor::new(&bytes);
        assert_eq!(
            CodecCborComposed::decode(&mut cursor),
            Err(refusal(&reject["error"])),
            "{why}"
        );
        assert_eq!(cursor.remaining(), bytes.len(), "{why}: the cursor moved");
    }
}

/// The `card` case of a token of `len` bytes, which the card's unit holds in
/// the language's own vector and its type bounds at four.
fn card_of(len: u8) -> Choice {
    Choice::Card(Card {
        token: (1..=len).collect(),
        amount: 0,
    })
}

/// A change to a value of the codec that leaves it one its types hold and no
/// encoding carries, with why it has none.
type Unencodable = (&'static str, fn(&mut CodecCborComposed<'_>));

#[test]
fn a_value_its_types_hold_and_no_encoding_carries_is_refused_before_a_byte() {
    let corpus = corpus();
    let smallest = &rows(&corpus, "cases")[0]["decoded"];
    let changes: [Unencodable; 5] = [
        ("no flag, below the lower count 1", |v| {
            v.flags = BoundedList::new();
        }),
        ("no union in choices, below the lower count 1", |v| {
            v.choices = BoundedList::new();
        }),
        ("a card of a long token in choice", |v| {
            v.choice = card_of(5);
        }),
        ("a card of a long token in maybe", |v| {
            v.maybe = Some(card_of(5));
        }),
        ("a card of a long token in choices", |v| {
            v.choices = BoundedList::new();
            v.choices
                .push(Choice::Idle)
                .expect("within the bound of three");
            v.choices
                .push(card_of(5))
                .expect("within the bound of three");
        }),
    ];
    for (why, change) in changes {
        let mut v = value(smallest);
        change(&mut v);
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(
            v.encode(&mut VecSink::new(&mut out)),
            Err(CodecError::CborOutOfRange),
            "{why}"
        );
        assert!(out.is_empty(), "{why}: the encode wrote before it refused");
    }
    // A token at its bound has an encoding.
    let mut v = value(smallest);
    v.choice = card_of(4);
    assert!(v.encode_to_vec().is_ok());
}

#[test]
fn a_nan_is_written_as_the_one_half_precision_nan() {
    let corpus = corpus();
    let mut v = value(&rows(&corpus, "cases")[0]["decoded"]);
    v.precise = f64::NAN;
    v.ratio = f32::NAN;
    let bytes = encoded(&v);
    // Key 2 and key 3, each `f9 7e 00` (docs/adr/0086, section 9.2).
    assert!(
        bytes
            .windows(6)
            .any(|w| w == [0x02, 0xf9, 0x7e, 0x00, 0x03, 0xf9]),
        "{bytes:02x?}"
    );
    assert!(
        bytes.windows(4).any(|w| w == [0x03, 0xf9, 0x7e, 0x00]),
        "{bytes:02x?}"
    );
}

#[test]
fn the_owned_form_holds_every_entry_and_writes_the_same_bytes() {
    let corpus = corpus();
    let largest = &rows(&corpus, "cases")[1];
    let bytes = bytes_of(&largest["encoded"]);
    let read = CodecCborComposed::decode(&mut SceCursor::new(&bytes)).expect("the case reads");
    let owned: CodecCborComposedOwned<Heap> = read
        .clone()
        .try_into_owned_in::<Heap>()
        .expect("every entry is within its bound");
    assert_eq!(owned.as_borrowed(), read);
    assert_eq!(
        owned
            .encode_to_vec()
            .expect("a value read from bytes has an encoding"),
        bytes
    );
}
