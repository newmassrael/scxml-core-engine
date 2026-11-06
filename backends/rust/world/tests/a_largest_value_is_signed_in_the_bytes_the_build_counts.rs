// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The build computes, by one recursion over a type, how long the deterministic CBOR of the type's
// largest value is (`sce_build::forge::value_type::cbor_max_len`, docs/adr/0086 section 2.1); the
// trust layer writes that CBOR (`sce_rust_trust::cbor`, section 9). The ADR holds the two to one
// number (section 2.3: a size is tight, not only an upper bound): this test writes the largest value
// of every type the conformance fixtures of section 14.1 declare, and of the records they make,
// through the encoder, and holds its length to the build's size.
//
// It lives in this crate because this crate reads both: the types through `sce-build`, which reads
// and sizes them as every generator does, and the encoder through `sce-rust-trust`. Each type is
// mirrored into the descriptor a Rust schema unit declares as its `TYPE` (section 9.4).

use sce_build::forge::model::SceType;
use sce_build::forge::value_type::{
    cbor_max_len, ScopedEnum, ScopedRecord, ScopedVariant, TypeScope, ValueType as BuildType,
    WrittenType,
};
use sce_rust_trust::cbor::{
    encode, EnumType, Field, Integer, RecordType, Shape, Value, ValueType, Variant,
};

/// `enum_signal_level.scxml`, which the batch item's `level` names as `SignalLevel`: a closed enum,
/// so its values are its declared numbers, each one byte, and not every number of its `uint8`.
const LEVEL: EnumType = EnumType {
    underlying: Integer::Uint8,
    variants: &[
        Variant {
            name: "low",
            value: 0,
        },
        Variant {
            name: "high",
            value: 1,
        },
        Variant {
            name: "critical",
            value: 2,
        },
    ],
    strict: true,
};

/// A closed enum whose longest number is a negative one: major type 1, with the argument 24.
const READING: EnumType = EnumType {
    underlying: Integer::Int8,
    variants: &[
        Variant {
            name: "none",
            value: -25,
        },
        Variant {
            name: "some",
            value: 3,
        },
    ],
    strict: true,
};

/// An open enum (`sce:strict-variants="false"`): it holds every number of its underlying type, so
/// its largest value is one no variant declares.
const CODE: EnumType = EnumType {
    underlying: Integer::Uint16,
    variants: &[
        Variant {
            name: "ok",
            value: 0,
        },
        Variant {
            name: "busy",
            value: 1,
        },
    ],
    strict: false,
};

/// Every enum the documents here name, by its alias.
const ENUMS: [(&str, &EnumType); 3] = [
    ("SignalLevel", &LEVEL),
    ("Reading", &READING),
    ("Code", &CODE),
];

/// `event_batch_item.scxml` (docs/adr/0086, section 14.1): a record that holds a list.
const ITEM_FIELDS: [(&str, &str); 7] = [
    ("id", "uint32"),
    ("amount", "int64"),
    ("label", "string[8]"),
    ("level", "enum:SignalLevel"),
    ("due", "optional<int64>"),
    ("blob", "bytes[4]"),
    ("tags", "list<string[8]>[4]"),
];

/// `event_batch.scxml`'s `batch.sent`, revision 2.
const BATCH_FIELDS: [(&str, &str); 7] = [
    ("items", "list<record:Item>[batch_items=3]"),
    ("grid", "list<list<uint8>[2]>[2]"),
    ("levels", "list<enum:SignalLevel>[2]"),
    ("maybe", "list<optional<uint16>>[2]"),
    ("spare", "optional<list<uint8>[2]>"),
    ("note", "string[4]"),
    ("late", "list<uint8>[2]"),
];

/// Compositions past the fixtures': the ADR's nested example, a limit name, an optional record,
/// lengths at the boundary where a head grows by a byte, and an enum of each kind.
const MORE: [&str; 10] = [
    "list<list<string[8]>[4]>[3]",
    "string[merge_text=96]",
    "optional<record:Item>",
    "list<optional<list<bytes[24]>[24]>>[2]",
    "bytes[23]",
    "list<float32>[23]",
    "enum:Reading",
    "optional<enum:Reading>",
    "enum:Code",
    "list<enum:Code>[2]",
];

fn ty(text: &str) -> BuildType {
    WrittenType::read(text)
        .unwrap_or_else(|refusal| panic!("{text} is read: {refusal:?}"))
        .bounded()
        .unwrap_or_else(|refusal| panic!("{text} states every bound: {refusal:?}"))
}

fn fields(table: &[(&str, &str)]) -> Vec<(String, BuildType)> {
    table
        .iter()
        .map(|(id, text)| (id.to_string(), ty(text)))
        .collect()
}

/// The enum the build reads for `descriptor`.
fn scoped(descriptor: &EnumType) -> ScopedEnum {
    ScopedEnum {
        variants: descriptor
            .variants
            .iter()
            .map(|variant| ScopedVariant {
                name: variant.name.to_string(),
                value: variant.value,
            })
            .collect(),
        underlying: match descriptor.underlying {
            Integer::Uint8 => SceType::Uint8,
            Integer::Uint16 => SceType::Uint16,
            Integer::Uint32 => SceType::Uint32,
            Integer::Uint64 => SceType::Uint64,
            Integer::Int8 => SceType::Int8,
            Integer::Int16 => SceType::Int16,
            Integer::Int32 => SceType::Int32,
            Integer::Int64 => SceType::Int64,
        },
        strict: descriptor.strict,
    }
}

/// The descriptor of the enum `alias` names.
fn enumeration(alias: &str) -> &'static EnumType {
    ENUMS
        .iter()
        .find(|(name, _)| *name == alias)
        .map(|(_, descriptor)| *descriptor)
        .unwrap_or_else(|| panic!("`{alias}` names an enum here"))
}

/// The aliases the batch document reads: `SignalLevel`, `Item`, and the payload as the record
/// `Batch`; and the enums of the compositions past the fixtures'.
fn scope() -> TypeScope {
    let mut item_scope = TypeScope::default();
    item_scope
        .enums
        .insert("SignalLevel".to_string(), scoped(&LEVEL));
    let mut batch_scope = TypeScope::default();
    for (alias, descriptor) in ENUMS {
        batch_scope
            .enums
            .insert(alias.to_string(), scoped(descriptor));
    }
    batch_scope.records.insert(
        "Item".to_string(),
        ScopedRecord {
            identity: "event_batch_item".to_string(),
            name: "event_batch_item".to_string(),
            fields: fields(&ITEM_FIELDS),
            scope: item_scope,
        },
    );
    let mut outer = batch_scope.clone();
    outer.records.insert(
        "Batch".to_string(),
        ScopedRecord {
            identity: "event_batch".to_string(),
            name: "event_batch".to_string(),
            fields: fields(&BATCH_FIELDS),
            scope: batch_scope,
        },
    );
    outer
}

fn leak_str(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

/// The descriptor of a build's type, every record resolved in the scope of its own document.
fn mirror(ty: &BuildType, scope: &TypeScope) -> ValueType {
    match ty {
        BuildType::Optional { elem } => ValueType::Optional(shape(elem, scope)),
        other => ValueType::Required(shape(other, scope)),
    }
}

fn shape(ty: &BuildType, scope: &TypeScope) -> Shape {
    match ty {
        BuildType::Scalar(scalar) => {
            match scalar {
                SceType::Uint8 => Shape::Integer(Integer::Uint8),
                SceType::Uint16 => Shape::Integer(Integer::Uint16),
                SceType::Uint32 => Shape::Integer(Integer::Uint32),
                SceType::Uint64 => Shape::Integer(Integer::Uint64),
                SceType::Int8 => Shape::Integer(Integer::Int8),
                SceType::Int16 => Shape::Integer(Integer::Int16),
                SceType::Int32 => Shape::Integer(Integer::Int32),
                SceType::Int64 => Shape::Integer(Integer::Int64),
                SceType::Float32 => Shape::Float32,
                SceType::Float64 => Shape::Float64,
                SceType::Bool => Shape::Bool,
                SceType::Enum(reference) => Shape::Enum(enumeration(&reference.alias)),
                SceType::String | SceType::Bytes => {
                    panic!("a value type holds a text or a byte string as Text or Bytes, with its bound")
                }
            }
        }
        BuildType::Text { max } => Shape::Text { max: max.max },
        BuildType::Bytes { max } => Shape::Bytes { max: max.max },
        BuildType::Optional { .. } => panic!("the grammar refuses an optional of an optional"),
        BuildType::Record { alias } => {
            let record = &scope.records[alias];
            let fields: Vec<Field> = record
                .fields
                .iter()
                .map(|(id, field)| Field {
                    id: leak_str(id),
                    ty: mirror(field, &record.scope),
                    since: 1,
                    default: None,
                })
                .collect();
            Shape::Record(Box::leak(Box::new(RecordType {
                fields: Box::leak(fields.into_boxed_slice()),
            })))
        }
        BuildType::List { elem, max } => Shape::List {
            elem: Box::leak(Box::new(mirror(elem, scope))),
            max: max.max,
        },
        // The encoder maps a union once its deterministic CBOR lands
        // (docs/adr/0092, section 5.3); no type of this test holds one.
        BuildType::Union { .. } => panic!("the encoder holds no union yet"),
    }
}

/// The value of `ty` whose encoding is longest: every integer at the end of its width whose head
/// is longest, every float one only the widest width holds, every text, byte string and list at its
/// bound, every optional present, and every enum at the number it holds whose head is longest.
fn largest(ty: &ValueType) -> Value {
    let (ValueType::Required(shape) | ValueType::Optional(shape)) = ty;
    match shape {
        Shape::Integer(integer) => Value::Integer(longest_integer(*integer)),
        Shape::Float32 => Value::Float(f64::from(0.1f32)),
        Shape::Float64 => Value::Float(0.1),
        Shape::Bool => Value::Bool(true),
        // A closed enum holds only its declared numbers: the one whose head is longest, which is
        // the one whose argument (`n`, or `-1 - n` below zero) is greatest.
        Shape::Enum(descriptor) if descriptor.strict => Value::Integer(
            descriptor
                .variants
                .iter()
                .map(|variant| variant.value)
                .max_by_key(|&n| if n < 0 { -1 - n } else { n })
                .expect("an enum declares a variant"),
        ),
        // An open enum holds every number of its underlying type.
        Shape::Enum(descriptor) => Value::Integer(longest_integer(descriptor.underlying)),
        Shape::Text { max } => Value::Text("\u{0}".repeat(*max as usize)),
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

/// The number of `integer` whose head is longest: the end of its width furthest from zero.
fn longest_integer(integer: Integer) -> i128 {
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

/// The largest value of `build` is written in exactly the bytes the build counts for it.
fn holds(what: &str, build: &BuildType, scope: &TypeScope) {
    let size = cbor_max_len(build, scope).unwrap_or_else(|e| panic!("{what} has a size: {e}"));
    let mirrored = mirror(build, scope);
    let written = encode(&largest(&mirrored), &mirrored)
        .unwrap_or_else(|refused| panic!("{what}: its largest value is refused: {refused}"));
    assert_eq!(
        written.len() as u64,
        size,
        "{what}: the bytes its largest value is written in"
    );
}

#[test]
fn the_largest_value_of_every_field_of_the_batch_fixtures_is_as_long_as_the_build_counts() {
    let scope = scope();
    for (id, text) in ITEM_FIELDS.iter().chain(&BATCH_FIELDS) {
        holds(&format!("{id}: {text}"), &ty(text), &scope);
    }
    for text in MORE {
        holds(text, &ty(text), &scope);
    }
}

#[test]
fn the_largest_item_and_the_largest_payload_are_as_long_as_the_build_counts() {
    let scope = scope();
    for alias in ["Item", "Batch"] {
        holds(
            alias,
            &BuildType::Record {
                alias: alias.to_string(),
            },
            &scope,
        );
    }
}
