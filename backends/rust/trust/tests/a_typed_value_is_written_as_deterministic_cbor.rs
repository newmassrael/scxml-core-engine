// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The deterministic CBOR of a typed value (docs/adr/0086, section 9) is what a signature will cover
// once ADR 0085's domain bump switches to it, so every byte of it is a contract with every host that
// checks one: the vectors of the ADR, written down byte for byte; the shortest head at every boundary
// of an integer and of a length (RFC 8949 section 4.2.1, preferred serialization); a record's keys in
// the order of their encodings; the defaults of a later revision; each refusal with its path; the
// length of a value at its largest against the size the ADR's calculus gives (section 2.1); and,
// over thousands of values of types drawn at random, a decoder this repository did not write
// (`ciborium`) reading back the same value, with the same bytes when it is written again, and every
// head and float of it the shortest, judged apart from the encoder.

use std::collections::BTreeMap;

use ciborium::value::Value as Cbor;
use sce_rust_trust::cbor::{
    encode, EnumType, Field, FieldDefault, Integer, RecordType, Refused, Shape, Step, Unit, Value,
    ValueType, Variant, Why,
};

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn encoded(value: &Value, ty: &ValueType) -> String {
    hex(&encode(value, ty).unwrap_or_else(|why| panic!("{value:?} is refused: {why}")))
}

fn refused(value: &Value, ty: &ValueType) -> Refused {
    match encode(value, ty) {
        Ok(bytes) => panic!("{value:?} is written as {}", hex(&bytes)),
        Err(refused) => refused,
    }
}

fn record(entries: &[(&str, Value)]) -> Value {
    Value::Record(
        entries
            .iter()
            .map(|(id, value)| (id.to_string(), value.clone()))
            .collect(),
    )
}

fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}

const fn required(shape: Shape) -> ValueType {
    ValueType::Required(shape)
}

const fn field(id: &'static str, ty: ValueType) -> Field {
    Field {
        id,
        ty,
        since: 1,
        default: None,
    }
}

const fn later(id: &'static str, ty: ValueType, default: Option<FieldDefault>) -> Field {
    Field {
        id,
        ty,
        since: 2,
        default,
    }
}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn leak_slice<T>(values: Vec<T>) -> &'static [T] {
    Box::leak(values.into_boxed_slice())
}

fn leak_str(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

const UINT8: ValueType = required(Shape::Integer(Integer::Uint8));
const UINT16: ValueType = required(Shape::Integer(Integer::Uint16));
const UINT32: ValueType = required(Shape::Integer(Integer::Uint32));
const UINT64: ValueType = required(Shape::Integer(Integer::Uint64));
const INT8: ValueType = required(Shape::Integer(Integer::Int8));
const INT16: ValueType = required(Shape::Integer(Integer::Int16));
const INT32: ValueType = required(Shape::Integer(Integer::Int32));
const INT64: ValueType = required(Shape::Integer(Integer::Int64));
const FLOAT32: ValueType = required(Shape::Float32);
const FLOAT64: ValueType = required(Shape::Float64);
const BOOL: ValueType = required(Shape::Bool);

/// `enum:E` over `uint8`, whose variant `week` declares 2 (the ADR's vector).
const PERIOD: EnumType = EnumType {
    underlying: Integer::Uint8,
    variants: &[
        Variant {
            name: "day",
            value: 1,
        },
        Variant {
            name: "week",
            value: 2,
        },
    ],
    strict: true,
};

/// The same variants, in an open enum (`sce:strict-variants="false"`).
const OPEN_PERIOD: EnumType = EnumType {
    strict: false,
    ..PERIOD
};

/// `{id, label}`: the ADR's record vector.
const ID_LABEL: RecordType = RecordType {
    fields: &[
        field("id", UINT32),
        field("label", required(Shape::Text { max: 8 })),
    ],
};

// ── The vectors of docs/adr/0086, section 9.3 ───────────────────────────────────────────────────

#[test]
fn every_vector_of_the_adr_is_written_byte_for_byte() {
    const AA_B: RecordType = RecordType {
        fields: &[field("aa", UINT8), field("b", UINT8)],
    };
    let rows: [(Value, ValueType, &str); 19] = [
        (Value::Integer(0), UINT8, "00"),
        (Value::Integer(500), UINT32, "19 01 f4"),
        (
            Value::Integer(i128::from(u64::MAX)),
            UINT64,
            "1b ff ff ff ff ff ff ff ff",
        ),
        (Value::Integer(-1), INT64, "20"),
        (Value::Integer(-500), INT32, "39 01 f3"),
        (
            Value::Integer(i128::from(i64::MIN)),
            INT64,
            "3b 7f ff ff ff ff ff ff ff",
        ),
        (Value::Float(1.5), FLOAT64, "f9 3e 00"),
        (Value::Float(f64::from(0.1f32)), FLOAT32, "fa 3d cc cc cd"),
        (Value::Float(0.1), FLOAT64, "fb 3f b9 99 99 99 99 99 9a"),
        (Value::Bool(true), BOOL, "f5"),
        (Value::Integer(2), required(Shape::Enum(&PERIOD)), "02"),
        (text("ab"), required(Shape::Text { max: 8 }), "62 61 62"),
        (
            Value::Bytes(vec![0x00, 0xff]),
            required(Shape::Bytes { max: 4 }),
            "42 00 ff",
        ),
        (
            Value::Absent,
            ValueType::Optional(Shape::Integer(Integer::Int64)),
            "f6",
        ),
        (
            Value::List(Vec::new()),
            required(Shape::List {
                elem: &UINT8,
                max: 3,
            }),
            "80",
        ),
        (
            Value::List(vec![Value::Integer(1), Value::Integer(2)]),
            required(Shape::List {
                elem: &UINT8,
                max: 3,
            }),
            "82 01 02",
        ),
        (
            Value::List(vec![
                Value::List(vec![Value::Integer(1)]),
                Value::List(Vec::new()),
            ]),
            required(Shape::List {
                elem: &ValueType::Required(Shape::List {
                    elem: &UINT8,
                    max: 2,
                }),
                max: 2,
            }),
            "82 81 01 80",
        ),
        (
            record(&[("id", Value::Integer(1)), ("label", text("a"))]),
            required(Shape::Record(&ID_LABEL)),
            "a2 62 69 64 01 65 6c 61 62 65 6c 61 61",
        ),
        // `b` first: its encoding is shorter.
        (
            record(&[("aa", Value::Integer(1)), ("b", Value::Integer(2))]),
            required(Shape::Record(&AA_B)),
            "a2 61 62 02 62 61 61 01",
        ),
    ];
    for (value, ty, expected) in &rows {
        assert_eq!(encoded(value, ty), *expected, "{value:?} as {ty:?}");
    }
}

// ── Preferred serialization at every boundary (RFC 8949 section 4.2.1) ─────────────────────────

/// Each boundary of a head's argument, and the head of major type 0 that carries it.
const HEADS: [(u64, &[u8]); 9] = [
    (0, &[0x00]),
    (23, &[0x17]),
    (24, &[0x18, 0x18]),
    (255, &[0x18, 0xff]),
    (256, &[0x19, 0x01, 0x00]),
    (65535, &[0x19, 0xff, 0xff]),
    (65536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
    (0xffff_ffff, &[0x1a, 0xff, 0xff, 0xff, 0xff]),
    (
        0x1_0000_0000,
        &[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
    ),
];

/// The head of `major` for an argument of [`HEADS`].
fn head(major: u8, argument: u64) -> Vec<u8> {
    let (_, bytes) = HEADS
        .iter()
        .find(|(n, _)| *n == argument)
        .unwrap_or_else(|| panic!("{argument} is a boundary of HEADS"));
    let mut head = bytes.to_vec();
    head[0] |= major << 5;
    head
}

#[test]
fn an_integer_has_the_shortest_head_on_both_sides_of_every_boundary() {
    for (n, _) in HEADS {
        assert_eq!(
            encoded(&Value::Integer(i128::from(n)), &UINT64),
            hex(&head(0, n)),
            "{n}"
        );
        // Major type 1 carries -1 - n.
        let negative = -1 - i128::from(n);
        assert_eq!(
            encoded(&Value::Integer(negative), &INT64),
            hex(&head(1, n)),
            "{negative}"
        );
    }
}

/// The lengths a text, a byte string, a list and a map are written at. A length of 2^32 - 1 or 2^32
/// is 4 GiB of value, which no test holds; its head is the integer's, written by the one head
/// writer, and held at those two boundaries above.
const LENGTHS: [u64; 7] = [0, 23, 24, 255, 256, 65535, 65536];

#[test]
fn a_text_and_a_byte_string_have_the_shortest_length_on_both_sides_of_every_boundary() {
    for n in LENGTHS {
        let len = usize::try_from(n).expect("a length fits usize");
        let written = encode(
            &text(&"a".repeat(len)),
            &required(Shape::Text { max: 65536 }),
        )
        .expect("a text within its bound");
        let head_text = head(3, n);
        assert_eq!(written[..head_text.len()], head_text[..], "text of {n}");
        assert_eq!(written.len(), head_text.len() + len, "text of {n}");

        let written = encode(
            &Value::Bytes(vec![0xff; len]),
            &required(Shape::Bytes { max: 65536 }),
        )
        .expect("bytes within their bound");
        let head_bytes = head(2, n);
        assert_eq!(written[..head_bytes.len()], head_bytes[..], "bytes of {n}");
        assert_eq!(written.len(), head_bytes.len() + len, "bytes of {n}");
    }
}

#[test]
fn a_list_and_a_record_have_the_shortest_count_on_both_sides_of_every_boundary() {
    for n in LENGTHS {
        let len = usize::try_from(n).expect("a length fits usize");
        let written = encode(
            &Value::List(vec![Value::Bool(false); len]),
            &required(Shape::List {
                elem: &BOOL,
                max: 65536,
            }),
        )
        .expect("a list within its bound");
        let head_list = head(4, n);
        assert_eq!(written[..head_list.len()], head_list[..], "list of {n}");
        assert_eq!(written.len(), head_list.len() + len, "list of {n}");

        let ids: Vec<&'static str> = (0..len).map(|i| leak_str(format!("f{i:05}"))).collect();
        let fields = leak_slice(ids.iter().map(|&id| field(id, BOOL)).collect());
        let value = Value::Record(
            ids.iter()
                .map(|id| (id.to_string(), Value::Bool(true)))
                .collect(),
        );
        let written = encode(
            &value,
            &required(Shape::Record(leak(RecordType { fields }))),
        )
        .expect("a record of every field");
        let head_map = head(5, n);
        assert_eq!(written[..head_map.len()], head_map[..], "map of {n}");
        // Each key is `f` and five digits, a text of 6 bytes behind its 1-byte head, then `f5`.
        assert_eq!(written.len(), head_map.len() + len * 8, "map of {n}");
    }
}

#[test]
fn a_float_is_the_shortest_of_half_single_and_double_that_holds_it() {
    // RFC 8949 Appendix A, and the edges of each width.
    let rows: [(f64, &str); 22] = [
        (0.0, "f9 00 00"),
        (-0.0, "f9 80 00"),
        (1.0, "f9 3c 00"),
        (1.1, "fb 3f f1 99 99 99 99 99 9a"),
        (1.5, "f9 3e 00"),
        (65504.0, "f9 7b ff"),
        (100000.0, "fa 47 c3 50 00"),
        (f64::from(f32::MAX), "fa 7f 7f ff ff"),
        (1.0e300, "fb 7e 37 e4 3c 88 00 75 9c"),
        // The least half, a subnormal.
        (2f64.powi(-24), "f9 00 01"),
        // The greatest half subnormal, and the least half normal.
        (1023.0 * 2f64.powi(-24), "f9 03 ff"),
        (2f64.powi(-14), "f9 04 00"),
        // Half the least half: a single holds it, no half does.
        (2f64.powi(-25), "fa 33 00 00 00"),
        // One past the greatest half, in a half's spacing there.
        (65505.0, "fa 47 7f e1 00"),
        // One bit below a single's last.
        (1.0 + 2f64.powi(-24), "fb 3f f0 00 00 10 00 00 00"),
        (-4.0, "f9 c4 00"),
        (-4.1, "fb c0 10 66 66 66 66 66 66"),
        (f64::INFINITY, "f9 7c 00"),
        (f64::NEG_INFINITY, "f9 fc 00"),
        (f64::NAN, "f9 7e 00"),
        // Every NaN is the one NaN, whatever its sign and payload.
        (f64::from_bits(0x7ff0_0000_0000_0001), "f9 7e 00"),
        (f64::from_bits(0xfff8_0000_0000_0000), "f9 7e 00"),
    ];
    for (x, expected) in rows {
        assert_eq!(
            encoded(&Value::Float(x), &FLOAT64),
            expected,
            "{x:e} as float64"
        );
    }
    assert_eq!(encoded(&Value::Float(100000.0), &FLOAT32), "fa 47 c3 50 00");
    assert_eq!(encoded(&Value::Float(1.5), &FLOAT32), "f9 3e 00");
    assert_eq!(encoded(&Value::Float(f64::INFINITY), &FLOAT32), "f9 7c 00");
    assert_eq!(encoded(&Value::Float(f64::NAN), &FLOAT32), "f9 7e 00");
}

#[test]
fn a_bool_an_optional_and_an_enum_are_what_their_type_says() {
    assert_eq!(encoded(&Value::Bool(false), &BOOL), "f4");
    assert_eq!(encoded(&Value::Bool(true), &BOOL), "f5");
    let maybe = ValueType::Optional(Shape::Integer(Integer::Int64));
    assert_eq!(encoded(&Value::Integer(5), &maybe), "05");
    assert_eq!(encoded(&Value::Absent, &maybe), "f6");
    let maybe_each = required(Shape::List {
        elem: &ValueType::Optional(Shape::Integer(Integer::Uint16)),
        max: 2,
    });
    assert_eq!(
        encoded(
            &Value::List(vec![Value::Absent, Value::Integer(7)]),
            &maybe_each
        ),
        "82 f6 07"
    );
    let maybe_all = ValueType::Optional(Shape::List {
        elem: &UINT8,
        max: 2,
    });
    assert_eq!(encoded(&Value::Absent, &maybe_all), "f6");
    assert_eq!(encoded(&Value::List(Vec::new()), &maybe_all), "80");
    // An open enum holds every number of its underlying type, a variant's or not.
    let open = required(Shape::Enum(&OPEN_PERIOD));
    assert_eq!(encoded(&Value::Integer(200), &open), "18 c8");
    assert_eq!(encoded(&Value::Integer(2), &open), "02");
}

// ── A record ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_records_keys_are_in_the_order_of_their_encodings() {
    // Declared in no order, and held by the value in the order of their texts; written shorter
    // first and, among keys of one length, by their bytes, which is what bytewise order of the
    // encodings gives (RFC 8949 section 4.2.1). The 23-byte key has a head of one byte and the
    // 24-byte key a head of two, so it comes after it although `x` is before `y`.
    const KEYS: RecordType = RecordType {
        fields: &[
            field("ccc", BOOL),
            field("xxxxxxxxxxxxxxxxxxxxxxxx", BOOL),
            field("aa", BOOL),
            field("yyyyyyyyyyyyyyyyyyyyyyy", BOOL),
            field("b", BOOL),
            field("ab", BOOL),
            field("a", BOOL),
            field("B", BOOL),
        ],
    };
    let value = Value::Record(
        KEYS.fields
            .iter()
            .map(|f| (f.id.to_string(), Value::Bool(true)))
            .collect(),
    );
    let expected = [
        "a8",
        "61 42 f5",
        "61 61 f5",
        "61 62 f5",
        "62 61 61 f5",
        "62 61 62 f5",
        "63 63 63 63 f5",
        &format!("77 {} f5", ["79"; 23].join(" ")),
        &format!("78 18 {} f5", ["78"; 24].join(" ")),
    ]
    .join(" ");
    assert_eq!(encoded(&value, &required(Shape::Record(&KEYS))), expected);
}

#[test]
fn a_key_the_schema_does_not_declare_is_not_written() {
    let declared = record(&[("id", Value::Integer(1)), ("label", text("a"))]);
    let more = record(&[
        ("id", Value::Integer(1)),
        ("label", text("a")),
        ("note", text("not signed")),
        ("a", Value::Bool(true)),
    ]);
    let ty = required(Shape::Record(&ID_LABEL));
    assert_eq!(encoded(&more, &ty), encoded(&declared, &ty));
}

#[test]
fn a_field_a_later_revision_added_is_written_as_its_default_when_it_is_lacking() {
    const REVISED: RecordType = RecordType {
        fields: &[
            field("id", UINT8),
            later(
                "late",
                required(Shape::List {
                    elem: &UINT8,
                    max: 2,
                }),
                None,
            ),
            later(
                "maybe",
                ValueType::Optional(Shape::Integer(Integer::Uint16)),
                None,
            ),
            later("mode", UINT8, Some(FieldDefault::Integer(7))),
            later(
                "tag",
                required(Shape::Text { max: 4 }),
                Some(FieldDefault::Text("x")),
            ),
        ],
    };
    let ty = required(Shape::Record(&REVISED));
    let earlier = record(&[("id", Value::Integer(1))]);
    // id, tag, late, mode, maybe: by length, then by bytes.
    let expected = "a5 62 69 64 01 63 74 61 67 61 78 64 6c 61 74 65 80 \
                    64 6d 6f 64 65 07 65 6d 61 79 62 65 f6";
    assert_eq!(encoded(&earlier, &ty), expected);
    // A delivery of the later revision that carries the defaults means the same, and is written
    // alike.
    let newer = record(&[
        ("id", Value::Integer(1)),
        ("late", Value::List(Vec::new())),
        ("maybe", Value::Absent),
        ("mode", Value::Integer(7)),
        ("tag", text("x")),
    ]);
    assert_eq!(encoded(&newer, &ty), expected);
    // A default stated for an optional is what it reads, not absence.
    const STATED: RecordType = RecordType {
        fields: &[later(
            "level",
            ValueType::Optional(Shape::Enum(&PERIOD)),
            Some(FieldDefault::Integer(1)),
        )],
    };
    assert_eq!(
        encoded(&record(&[]), &required(Shape::Record(&STATED))),
        "a1 65 6c 65 76 65 6c 01"
    );
    // Each kind of default, written as the field's type.
    const KINDS: RecordType = RecordType {
        fields: &[
            later("f", FLOAT64, Some(FieldDefault::Float(1.5))),
            later("t", BOOL, Some(FieldDefault::Bool(true))),
            later(
                "b",
                required(Shape::Bytes { max: 2 }),
                Some(FieldDefault::Bytes(&[0x01])),
            ),
        ],
    };
    assert_eq!(
        encoded(&record(&[]), &required(Shape::Record(&KINDS))),
        "a3 61 62 41 01 61 66 f9 3e 00 61 74 f5"
    );
}

// ── Refusals ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_value_not_of_its_shape_is_refused() {
    let wrong = refused(&text("1"), &UINT8);
    assert_eq!(wrong.why, Why::NotOf { expected: "uint8" });
    assert_eq!(wrong.to_string(), "'' is not a value of `uint8`");
    assert_eq!(
        refused(&Value::Integer(1), &BOOL).why,
        Why::NotOf { expected: "bool" }
    );
    assert_eq!(
        refused(&Value::Integer(1), &FLOAT64).why,
        Why::NotOf {
            expected: "float64"
        }
    );
    assert_eq!(
        refused(&Value::Bytes(vec![1]), &required(Shape::Text { max: 4 })).why,
        Why::NotOf { expected: "string" }
    );
    assert_eq!(
        refused(&text("a"), &required(Shape::Bytes { max: 4 })).why,
        Why::NotOf { expected: "bytes" }
    );
    assert_eq!(
        refused(&text("a"), &required(Shape::Enum(&PERIOD))).why,
        Why::NotOf { expected: "enum" }
    );
    assert_eq!(
        refused(
            &Value::List(Vec::new()),
            &required(Shape::Record(&ID_LABEL))
        )
        .why,
        Why::NotOf { expected: "record" }
    );
    assert_eq!(
        refused(
            &record(&[]),
            &required(Shape::List {
                elem: &UINT8,
                max: 2
            })
        )
        .why,
        Why::NotOf { expected: "list" }
    );
}

#[test]
fn an_integer_outside_its_width_is_refused() {
    let cases: [(ValueType, i128, Integer); 10] = [
        (UINT8, 256, Integer::Uint8),
        (UINT8, -1, Integer::Uint8),
        (UINT16, 65536, Integer::Uint16),
        (UINT32, 1 << 32, Integer::Uint32),
        (UINT64, 1 << 64, Integer::Uint64),
        (UINT64, -1, Integer::Uint64),
        (INT8, -129, Integer::Int8),
        (INT16, 32768, Integer::Int16),
        (INT32, -(1 << 31) - 1, Integer::Int32),
        (INT64, 1 << 63, Integer::Int64),
    ];
    for (ty, value, integer) in cases {
        assert_eq!(
            refused(&Value::Integer(value), &ty).why,
            Why::OutOfRange { value, integer },
            "{value} as {integer}"
        );
    }
    // The greatest and the least of each width are values of it.
    assert_eq!(encoded(&Value::Integer(255), &UINT8), "18 ff");
    assert_eq!(encoded(&Value::Integer(-128), &INT8), "38 7f");
    assert_eq!(encoded(&Value::Integer(32767), &INT16), "19 7f ff");
    let open = required(Shape::Enum(&OPEN_PERIOD));
    assert_eq!(
        refused(&Value::Integer(256), &open).why,
        Why::OutOfRange {
            value: 256,
            integer: Integer::Uint8
        }
    );
    assert_eq!(
        refused(&Value::Integer(500), &UINT8).to_string(),
        "'' is 500, outside `uint8`"
    );
}

#[test]
fn a_number_a_closed_enum_does_not_declare_is_refused() {
    let closed = refused(&Value::Integer(3), &required(Shape::Enum(&PERIOD)));
    assert_eq!(closed.why, Why::Undeclared { value: 3 });
    assert_eq!(
        closed.to_string(),
        "'' is 3, which no variant of its enum declares"
    );
}

#[test]
fn a_float32_no_single_holds_is_refused() {
    let wrong = refused(&Value::Float(0.1), &FLOAT32);
    assert_eq!(wrong.why, Why::NotSingle { value: 0.1 });
    assert_eq!(
        wrong.to_string(),
        "'' is 0.1, which no float32 holds exactly"
    );
}

#[test]
fn a_text_a_byte_string_and_a_list_past_their_bound_are_refused() {
    let note = refused(&text("abcde"), &required(Shape::Text { max: 4 }));
    assert_eq!(
        note.why,
        Why::PastBound {
            len: 5,
            max: 4,
            unit: Unit::Bytes
        }
    );
    assert_eq!(
        note.to_string(),
        "'' is 5 bytes, past the 4 its type declares"
    );
    // A bound counts bytes, not characters: two characters of two bytes each.
    assert_eq!(
        refused(&text("\u{e9}\u{e9}"), &required(Shape::Text { max: 3 })).why,
        Why::PastBound {
            len: 4,
            max: 3,
            unit: Unit::Bytes
        }
    );
    assert_eq!(
        encoded(&text("\u{e9}"), &required(Shape::Text { max: 2 })),
        "62 c3 a9"
    );
    assert_eq!(
        refused(
            &Value::Bytes(vec![0; 5]),
            &required(Shape::Bytes { max: 4 })
        )
        .why,
        Why::PastBound {
            len: 5,
            max: 4,
            unit: Unit::Bytes
        }
    );
    // The list is counted before an element is looked at: the fourth is not even of its type, and
    // the refusal is the count's.
    let list = refused(
        &Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            text("x"),
        ]),
        &required(Shape::List {
            elem: &UINT8,
            max: 3,
        }),
    );
    assert_eq!(
        list.why,
        Why::PastBound {
            len: 4,
            max: 3,
            unit: Unit::Elements
        }
    );
    assert_eq!(list.at, Vec::new());
    assert_eq!(
        list.to_string(),
        "'' holds 4 elements, past the 3 its type declares"
    );
}

#[test]
fn an_absent_value_whose_type_is_not_optional_is_refused() {
    let absent = refused(&Value::Absent, &INT64);
    assert_eq!(absent.why, Why::Absent);
    assert_eq!(
        absent.to_string(),
        "'' is null, and its type is not optional"
    );
}

#[test]
fn a_record_that_lacks_a_field_with_no_default_is_refused() {
    let missing = refused(
        &record(&[("label", text("a"))]),
        &required(Shape::Record(&ID_LABEL)),
    );
    assert_eq!(missing.why, Why::Missing);
    assert_eq!(missing.at, vec![Step::Field("id")]);
    assert_eq!(
        missing.to_string(),
        "'id' is missing, and every value holds a field of the first revision"
    );
    // A later field that is neither optional nor a list, and is given no default.
    const NO_DEFAULT: RecordType = RecordType {
        fields: &[later("count", UINT8, None)],
    };
    let lacking = refused(&record(&[]), &required(Shape::Record(&NO_DEFAULT)));
    assert_eq!(lacking.why, Why::NoDefault);
    assert_eq!(lacking.at, vec![Step::Field("count")]);
    assert_eq!(
        lacking.to_string(),
        "'count' is missing, and its schema gives no default"
    );
}

#[test]
fn a_record_type_that_declares_an_id_twice_is_refused() {
    const TWICE: RecordType = RecordType {
        fields: &[field("a", UINT8), field("b", UINT8), field("a", BOOL)],
    };
    let twice = refused(
        &record(&[("a", Value::Integer(1)), ("b", Value::Integer(2))]),
        &required(Shape::Record(&TWICE)),
    );
    assert_eq!(twice.why, Why::DeclaredTwice);
    assert_eq!(twice.at, vec![Step::Field("a")]);
    assert_eq!(twice.to_string(), "'a' is declared twice by its record");
}

#[test]
fn a_refusal_names_the_path_through_every_depth() {
    // `batch.sent`'s `items`, each with its `tags` (docs/adr/0086, section 14.1).
    const ITEM: RecordType = RecordType {
        fields: &[
            field("id", UINT32),
            field(
                "tags",
                required(Shape::List {
                    elem: &ValueType::Required(Shape::Text { max: 8 }),
                    max: 4,
                }),
            ),
        ],
    };
    const BATCH: RecordType = RecordType {
        fields: &[field(
            "items",
            required(Shape::List {
                elem: &ValueType::Required(Shape::Record(&ITEM)),
                max: 3,
            }),
        )],
    };
    let item = |tag: &str| {
        record(&[
            ("id", Value::Integer(1)),
            ("tags", Value::List(vec![text(tag)])),
        ])
    };
    let batch = record(&[(
        "items",
        Value::List(vec![item("a"), item("b"), item("ninebytes")]),
    )]);
    let deep = refused(&batch, &required(Shape::Record(&BATCH)));
    assert_eq!(
        deep.at,
        vec![
            Step::Field("items"),
            Step::Element(2),
            Step::Field("tags"),
            Step::Element(0)
        ]
    );
    assert_eq!(
        deep.to_string(),
        "'items/2/tags/0' is 9 bytes, past the 8 its type declares"
    );
    // A null inside an element whose type is not optional.
    let null = refused(
        &record(&[(
            "items",
            Value::List(vec![
                item("a"),
                record(&[("id", Value::Absent), ("tags", Value::List(Vec::new()))]),
            ]),
        )]),
        &required(Shape::Record(&BATCH)),
    );
    assert_eq!(
        null.to_string(),
        "'items/1/id' is null, and its type is not optional"
    );
    // An id is escaped as a JSON Pointer escapes it (RFC 6901 section 3).
    const ODD: RecordType = RecordType {
        fields: &[field("a/b~c", UINT8)],
    };
    assert_eq!(
        refused(&record(&[]), &required(Shape::Record(&ODD))).pointer(),
        "a~1b~0c"
    );
}

// ── The largest value of a type is as long as the calculus says (docs/adr/0086, section 2.1) ─────

/// The value of `ty` whose encoding is longest: every integer at the end of its width whose head is
/// longest, every float one only the widest width holds, every text, byte string and list at its
/// bound, every optional present and every enum at its greatest number.
fn largest(ty: &ValueType) -> Value {
    let (ValueType::Required(shape) | ValueType::Optional(shape)) = ty;
    match shape {
        Shape::Integer(integer) => Value::Integer(widest(*integer)),
        Shape::Float32 => Value::Float(f64::from(0.1f32)),
        Shape::Float64 => Value::Float(0.1),
        Shape::Bool => Value::Bool(false),
        Shape::Enum(enumeration) if enumeration.strict => Value::Integer(
            enumeration
                .variants
                .iter()
                .map(|variant| variant.value)
                .max()
                .expect("an enum declares a variant"),
        ),
        Shape::Enum(enumeration) => Value::Integer(widest(enumeration.underlying)),
        Shape::Text { max } => text(&"t".repeat(*max as usize)),
        Shape::Bytes { max } => Value::Bytes(vec![0xff; *max as usize]),
        Shape::Record(record) => Value::Record(
            record
                .fields
                .iter()
                .map(|field| (field.id.to_string(), largest(&field.ty)))
                .collect(),
        ),
        Shape::List { elem, max } => Value::List(vec![largest(elem); *max as usize]),
    }
}

/// The value of `integer` with the longest head: the greatest of an unsigned width, the least of a
/// signed one.
fn widest(integer: Integer) -> i128 {
    match integer {
        Integer::Uint8 => i128::from(u8::MAX),
        Integer::Uint16 => i128::from(u16::MAX),
        Integer::Uint32 => i128::from(u32::MAX),
        Integer::Uint64 => i128::from(u64::MAX),
        Integer::Int8 => i128::from(i8::MIN),
        Integer::Int16 => i128::from(i16::MIN),
        Integer::Int32 => i128::from(i32::MIN),
        Integer::Int64 => i128::from(i64::MIN),
    }
}

#[test]
fn the_largest_value_of_each_row_of_the_calculus_is_as_long_as_its_row_says() {
    const OPEN_WIDE: EnumType = EnumType {
        underlying: Integer::Uint16,
        variants: &[Variant {
            name: "one",
            value: 1,
        }],
        strict: false,
    };
    // `C(T)` of section 2.1, row by row, `h(n)` written out.
    let rows: [(ValueType, usize); 21] = [
        (UINT8, 2),
        (UINT16, 3),
        (UINT32, 5),
        (UINT64, 9),
        (INT8, 2),
        (INT16, 3),
        (INT32, 5),
        (INT64, 9),
        (FLOAT32, 5),
        (FLOAT64, 9),
        (BOOL, 1),
        // `C` of the underlying type.
        (required(Shape::Enum(&OPEN_WIDE)), 3),
        // h(B) + B.
        (required(Shape::Text { max: 8 }), 1 + 8),
        (required(Shape::Text { max: 24 }), 2 + 24),
        (required(Shape::Bytes { max: 300 }), 3 + 300),
        // max(1, C(T)).
        (ValueType::Optional(Shape::Integer(Integer::Int64)), 9),
        (ValueType::Optional(Shape::Bool), 1),
        // h(n) + the sum of C(id) + C(Tf).
        (
            required(Shape::Record(&ID_LABEL)),
            1 + (3 + 5) + (6 + 1 + 8),
        ),
        // h(B) + B * C(T).
        (
            required(Shape::List {
                elem: &UINT16,
                max: 24,
            }),
            2 + 24 * 3,
        ),
        (
            required(Shape::List {
                elem: &ValueType::Required(Shape::List {
                    elem: &UINT8,
                    max: 2,
                }),
                max: 2,
            }),
            1 + 2 * (1 + 2 * 2),
        ),
        (
            required(Shape::List {
                elem: &ValueType::Optional(Shape::Text { max: 30 }),
                max: 3,
            }),
            1 + 3 * (2 + 30),
        ),
    ];
    for (ty, size) in &rows {
        let value = largest(ty);
        let written = encode(&value, ty).expect("the largest value is of its type");
        assert_eq!(written.len(), *size, "{ty:?}");
    }
}

// ── Read back by a decoder this repository did not write ────────────────────────────────────────

/// A generator of numbers that is the same on every run (SplitMix64), so a failure is replayed by
/// running the test again.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len() as u64) as usize]
    }

    fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }
}

const INTEGERS: [Integer; 8] = [
    Integer::Uint8,
    Integer::Uint16,
    Integer::Uint32,
    Integer::Uint64,
    Integer::Int8,
    Integer::Int16,
    Integer::Int32,
    Integer::Int64,
];

/// Ids of every length class a key's head has, and of one character past ASCII.
const IDS: [&str; 10] = [
    "a",
    "b",
    "B",
    "aa",
    "id",
    "label",
    "\u{e9}t\u{e9}",
    "xxxxxxxxxxxxxxxxxxxxxxx",
    "yyyyyyyyyyyyyyyyyyyyyyyy",
    "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
];

fn range(integer: Integer) -> (i128, i128) {
    match integer {
        Integer::Uint8 => (0, i128::from(u8::MAX)),
        Integer::Uint16 => (0, i128::from(u16::MAX)),
        Integer::Uint32 => (0, i128::from(u32::MAX)),
        Integer::Uint64 => (0, i128::from(u64::MAX)),
        Integer::Int8 => (i128::from(i8::MIN), i128::from(i8::MAX)),
        Integer::Int16 => (i128::from(i16::MIN), i128::from(i16::MAX)),
        Integer::Int32 => (i128::from(i32::MIN), i128::from(i32::MAX)),
        Integer::Int64 => (i128::from(i64::MIN), i128::from(i64::MAX)),
    }
}

fn draw_integer(draw: &mut Draw, integer: Integer) -> i128 {
    let (least, greatest) = range(integer);
    if draw.one_in(2) {
        // A boundary of a head, on either side of zero, where the width holds it.
        let (n, _) = draw.pick(&HEADS);
        let near = [
            i128::from(n),
            -1 - i128::from(n),
            least,
            greatest,
            i128::from(n) - 1,
        ];
        let candidate = draw.pick(&near);
        if (least..=greatest).contains(&candidate) {
            return candidate;
        }
    }
    let span = (greatest - least) as u128 + 1;
    let wide = (u128::from(draw.next()) << 64) | u128::from(draw.next());
    least + (wide % span) as i128
}

fn draw_float64(draw: &mut Draw) -> f64 {
    match draw.below(4) {
        0 => f64::from_bits(draw.next()),
        1 => f64::from(f32::from_bits(draw.next() as u32)),
        // A number of few bits, which a half often holds.
        2 => {
            let sign = if draw.one_in(2) { -1.0 } else { 1.0 };
            sign * (draw.below(2048) as f64) * 2f64.powi(draw.below(48) as i32 - 30)
        }
        _ => draw.pick(&[
            0.0,
            -0.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            65504.0,
            65505.0,
            1.5,
            0.1,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
        ]),
    }
}

fn draw_text(draw: &mut Draw, max: u32) -> String {
    let target = if draw.one_in(3) {
        max as usize
    } else {
        draw.below(u64::from(max) + 1) as usize
    };
    let mut out = String::new();
    loop {
        let c = draw.pick(&['a', 'Z', '~', '/', '"', '\u{0}', '\u{e9}', '\u{20ac}']);
        if out.len() + c.len_utf8() > target {
            return out;
        }
        out.push(c);
    }
}

fn draw_shape(draw: &mut Draw, depth: u32) -> Shape {
    let kinds = if depth < 3 { 16 } else { 14 };
    match draw.below(kinds) {
        k @ 0..=7 => Shape::Integer(INTEGERS[k as usize]),
        8 => Shape::Float32,
        9 => Shape::Float64,
        10 => Shape::Bool,
        11 => {
            let underlying = draw.pick(&INTEGERS);
            let mut values: Vec<i128> = (0..=draw.below(3))
                .map(|_| draw_integer(draw, underlying))
                .collect();
            values.sort_unstable();
            values.dedup();
            let variants = values
                .into_iter()
                .map(|value| Variant {
                    name: leak_str(format!("v{value}")),
                    value,
                })
                .collect();
            Shape::Enum(leak(EnumType {
                underlying,
                variants: leak_slice(variants),
                strict: draw.one_in(2),
            }))
        }
        12 => Shape::Text {
            max: draw.pick(&[1, 2, 23, 24, 30, 300]),
        },
        13 => Shape::Bytes {
            max: draw.pick(&[1, 4, 23, 24, 300]),
        },
        14 => {
            let mut ids: Vec<&'static str> = (0..draw.below(5)).map(|_| draw.pick(&IDS)).collect();
            ids.sort_unstable();
            ids.dedup();
            let fields = ids
                .into_iter()
                .map(|id| field(id, draw_type(draw, depth + 1)))
                .collect();
            Shape::Record(leak(RecordType {
                fields: leak_slice(fields),
            }))
        }
        _ => Shape::List {
            elem: leak(draw_type(draw, depth + 1)),
            max: draw.pick(&[1, 2, 3, 5]),
        },
    }
}

fn draw_type(draw: &mut Draw, depth: u32) -> ValueType {
    let shape = draw_shape(draw, depth);
    if draw.one_in(4) {
        ValueType::Optional(shape)
    } else {
        ValueType::Required(shape)
    }
}

fn draw_value(draw: &mut Draw, ty: &ValueType) -> Value {
    let shape = match ty {
        ValueType::Optional(_) if draw.one_in(3) => return Value::Absent,
        ValueType::Required(shape) | ValueType::Optional(shape) => shape,
    };
    match shape {
        Shape::Integer(integer) => Value::Integer(draw_integer(draw, *integer)),
        Shape::Float32 => Value::Float(f64::from(draw_float64(draw) as f32)),
        Shape::Float64 => Value::Float(draw_float64(draw)),
        Shape::Bool => Value::Bool(draw.one_in(2)),
        Shape::Enum(enumeration) if enumeration.strict => {
            Value::Integer(draw.pick(enumeration.variants).value)
        }
        Shape::Enum(enumeration) => Value::Integer(draw_integer(draw, enumeration.underlying)),
        Shape::Text { max } => Value::Text(draw_text(draw, *max)),
        Shape::Bytes { max } => {
            let len = draw.below(u64::from(*max) + 1);
            Value::Bytes((0..len).map(|_| draw.next() as u8).collect())
        }
        Shape::Record(record) => Value::Record(
            record
                .fields
                .iter()
                .map(|field| (field.id.to_string(), draw_value(draw, &field.ty)))
                .collect(),
        ),
        Shape::List { elem, max } => {
            let len = draw.below(u64::from(*max) + 1);
            Value::List((0..len).map(|_| draw_value(draw, elem)).collect())
        }
    }
}

/// The encoding of a text key, written here apart from the encoder: what the order of a map's keys
/// is judged by.
fn key_encoding(id: &str) -> Vec<u8> {
    let len = id.len();
    let mut out = match len {
        0..=23 => vec![0x60 | len as u8],
        24..=0xff => vec![0x78, len as u8],
        _ => {
            let mut head = vec![0x79];
            head.extend_from_slice(&(len as u16).to_be_bytes());
            head
        }
    };
    out.extend_from_slice(id.as_bytes());
    out
}

/// The value a decoded item is, read by `ty`: a map's keys must be the record's ids, in the order
/// of their encodings.
fn read_back(item: &Cbor, ty: &ValueType) -> Value {
    match (ty, item) {
        (ValueType::Optional(_), Cbor::Null) => Value::Absent,
        (ValueType::Required(shape) | ValueType::Optional(shape), item) => read_shape(item, shape),
    }
}

fn read_shape(item: &Cbor, shape: &Shape) -> Value {
    match (shape, item) {
        (Shape::Integer(_) | Shape::Enum(_), Cbor::Integer(n)) => Value::Integer(i128::from(*n)),
        (Shape::Float32 | Shape::Float64, Cbor::Float(x)) => Value::Float(*x),
        (Shape::Bool, Cbor::Bool(b)) => Value::Bool(*b),
        (Shape::Text { .. }, Cbor::Text(t)) => Value::Text(t.clone()),
        (Shape::Bytes { .. }, Cbor::Bytes(b)) => Value::Bytes(b.clone()),
        (Shape::List { elem, .. }, Cbor::Array(items)) => {
            Value::List(items.iter().map(|item| read_back(item, elem)).collect())
        }
        (Shape::Record(record), Cbor::Map(entries)) => {
            let keys: Vec<&str> = entries
                .iter()
                .map(|(key, _)| match key {
                    Cbor::Text(id) => id.as_str(),
                    other => panic!("a key {other:?} that is not a text"),
                })
                .collect();
            assert!(
                keys.windows(2)
                    .all(|pair| key_encoding(pair[0]) < key_encoding(pair[1])),
                "the keys {keys:?} are in the order of their encodings"
            );
            assert_eq!(keys.len(), record.fields.len(), "one key per field");
            let mut fields = BTreeMap::new();
            for (id, (_, value)) in keys.iter().zip(entries) {
                let field = record
                    .fields
                    .iter()
                    .find(|field| field.id == *id)
                    .unwrap_or_else(|| panic!("{id} is a field of the record"));
                fields.insert(id.to_string(), read_back(value, &field.ty));
            }
            Value::Record(fields)
        }
        (shape, item) => panic!("{item:?} is not of {shape:?}"),
    }
}

/// One value: every float equal by its bits, any NaN equal to any NaN (the one NaN is written).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => {
            (x.is_nan() && y.is_nan()) || x.to_bits() == y.to_bits()
        }
        (Value::List(xs), Value::List(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same(x, y))
        }
        (Value::Record(xs), Value::Record(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .zip(ys)
                    .all(|((kx, x), (ky, y))| kx == ky && same(x, y))
        }
        (a, b) => a == b,
    }
}

fn holds_as_half(x: f64) -> bool {
    half::f16::from_f64(x).to_f64() == x
}

fn holds_as_single(x: f64) -> bool {
    f64::from(x as f32) == x
}

/// The `n` bytes of `bytes` from `at` as a big-endian number, `at` moved past them.
fn take(bytes: &[u8], at: &mut usize, n: usize) -> u64 {
    let mut value = 0u64;
    for byte in &bytes[*at..*at + n] {
        value = (value << 8) | u64::from(*byte);
    }
    *at += n;
    value
}

/// Walk one item of `bytes` from `at`, holding every head and float to its shortest form and every
/// length to definite (RFC 8949 section 4.2.1), apart from the encoder.
fn walk_preferred(bytes: &[u8], at: &mut usize) {
    let initial = bytes[*at];
    *at += 1;
    let (major, info) = (initial >> 5, initial & 0x1f);
    if major == 7 {
        match info {
            20..=22 => {}
            25 => {
                let bits = take(bytes, at, 2);
                let nan = (bits & 0x7c00) == 0x7c00 && (bits & 0x03ff) != 0;
                assert!(!nan || bits == 0x7e00, "the one NaN, not {bits:04x}");
            }
            26 => {
                let x = f64::from(f32::from_bits(take(bytes, at, 4) as u32));
                assert!(!x.is_nan(), "a NaN in a single");
                assert!(!holds_as_half(x), "{x:e} in a single, which a half holds");
            }
            27 => {
                let x = f64::from_bits(take(bytes, at, 8));
                assert!(!x.is_nan(), "a NaN in a double");
                assert!(
                    !holds_as_half(x) && !holds_as_single(x),
                    "{x:e} in a double, which a narrower float holds"
                );
            }
            _ => panic!("simple value {info} is not written"),
        }
        return;
    }
    let argument = match info {
        0..=23 => u64::from(info),
        24 => take(bytes, at, 1),
        25 => take(bytes, at, 2),
        26 => take(bytes, at, 4),
        27 => take(bytes, at, 8),
        _ => panic!("an indefinite length or a reserved head: {initial:02x}"),
    };
    let least = match info {
        24 => 24,
        25 => 0x100,
        26 => 0x1_0000,
        27 => 0x1_0000_0000,
        _ => 0,
    };
    assert!(argument >= least, "{argument} in a head of {info}");
    let items = match major {
        0 | 1 => 0,
        2 | 3 => {
            *at += argument as usize;
            0
        }
        4 => argument,
        5 => argument * 2,
        _ => panic!("major type {major} is not written"),
    };
    for _ in 0..items {
        walk_preferred(bytes, at);
    }
}

#[test]
fn a_value_of_any_type_reads_back_through_another_decoder_and_writes_again_alike() {
    let mut draw = Draw(0x05ce_0086);
    for case in 0..3000 {
        let ty = draw_type(&mut draw, 0);
        let value = draw_value(&mut draw, &ty);
        let bytes = encode(&value, &ty)
            .unwrap_or_else(|refused| panic!("case {case}: {value:?} is refused: {refused}"));

        let mut at = 0;
        walk_preferred(&bytes, &mut at);
        assert_eq!(
            at,
            bytes.len(),
            "case {case}: one item, and nothing after it"
        );

        let decoded: Cbor = ciborium::de::from_reader(bytes.as_slice())
            .unwrap_or_else(|e| panic!("case {case}: {} does not decode: {e:?}", hex(&bytes)));
        let back = read_back(&decoded, &ty);
        assert!(
            same(&back, &value),
            "case {case}: {value:?} reads back as {back:?}"
        );
        let again = encode(&back, &ty).expect("what was read back is of its type");
        assert_eq!(hex(&again), hex(&bytes), "case {case}: written again");
    }
}

#[test]
fn a_float_is_written_at_the_width_another_reckoning_of_exactness_gives() {
    let mut draw = Draw(0x000f_10a7);
    for _ in 0..100_000 {
        let x = draw_float64(&mut draw);
        let bytes = encode(&Value::Float(x), &FLOAT64).expect("every float64");
        let expected = if x.is_nan() || holds_as_half(x) {
            3
        } else if holds_as_single(x) {
            5
        } else {
            9
        };
        assert_eq!(bytes.len(), expected, "{x:e}: {}", hex(&bytes));
        let decoded: Cbor = ciborium::de::from_reader(bytes.as_slice()).expect("a float decodes");
        let y = match decoded {
            Cbor::Float(y) => y,
            other => panic!("{x:e} decodes as {other:?}"),
        };
        assert!(
            (x.is_nan() && y.is_nan()) || x.to_bits() == y.to_bits(),
            "{x:e} reads back as {y:e}"
        );
    }
}
