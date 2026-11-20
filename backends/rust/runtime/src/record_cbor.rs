// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The record codec: a record a COSE operation of the standard host signs or
//! seals by name crosses as its deterministic CBOR, and one it verified or
//! opened is read back from those bytes (docs/adr/0109, D9; docs/adr/0101,
//! section 2; docs/SCE_HOST_FEATURES.md, "Signing and sealing").
//!
//! A machine holds a record in its own types and writes it, as every payload,
//! as JSON. The two forms are one tree (docs/adr/0101, section 2.4): a record
//! a map of its fields, a list an array, a union a map of one entry, and the
//! leaves related one by one, by the record's type. This module is that one
//! map over the type, both ways:
//!
//! | Type | JSON (a payload's) | CBOR (deterministic, RFC 8949 section 4.2.1) |
//! |---|---|---|
//! | an integer of 8 to 32 bits | a number | major type 0, or 1 below zero |
//! | an integer of 64 bits | the text of its digits (a number up to 2^53 - 1 read too) | the same |
//! | a float | a number | the shortest of half, single and double that holds it |
//! | `bool` | `true`, `false` | `f5`, `f4` |
//! | a text | a text | a text string |
//! | a byte string | its Latin-1 text, one character a byte | a byte string |
//! | an enum | the name of its variant | the integer of the variant's number |
//! | a record | an object of every field, `null` for one absent | a map of the fields present, each keyed by its id |
//! | a union | an object of its tag beside the case's fields | a map of one entry, the case's name to its record |
//! | a list | an array | an array |
//! | an absent element of a list | `null` | `null` (`f6`) |
//!
//! The type is the description the build writes of it
//! (`sce_build::forge::payload_record::describe`), JSON text, each record's
//! fields in the order their keys encode, so this module sorts nothing. Every
//! CBOR item is written and read by the forge runtime's items
//! (`sce_forge_runtime::cbor`), so no CBOR is spelled here.
//!
//! A decode reads the profile of definite lengths, and refuses what is no
//! value of the record (docs/adr/0101, section 3.3): an item of another type,
//! a key given twice, a field that is not optional missing, a value outside
//! its type, a variant no enum or union declares, and bytes after the record.
//! It also refuses what the machine's payload holds no spelling of: a float
//! that is not finite and a text that carries U+0000.

use core::fmt;

use sce_forge_runtime::cbor;
use sce_forge_runtime::codec::{CodecError, SceCursor, VecSink};

use crate::json::{self, Value};

/// Why a record has no encoding, or why bytes are no value of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordCborRefusal {
    /// The strict RFC 6901 pointer to the place refused, empty for the record
    /// itself.
    pub pointer: String,
    /// What was refused there.
    pub reason: String,
}

impl RecordCborRefusal {
    fn at(pointer: &str, reason: impl Into<String>) -> Self {
        Self {
            pointer: pointer.to_string(),
            reason: reason.into(),
        }
    }

    fn codec(pointer: &str, kind: CodecError) -> Self {
        Self::at(pointer, format!("is no value of its type ({kind:?})"))
    }
}

impl fmt::Display for RecordCborRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.pointer.is_empty() {
            write!(f, "the record {}", self.reason)
        } else {
            write!(f, "'{}' {}", self.pointer, self.reason)
        }
    }
}

impl std::error::Error for RecordCborRefusal {}

/// A field of a record, as the description names it.
#[derive(Debug)]
struct Field {
    id: String,
    optional: bool,
    shape: Shape,
}

/// A type, as the description names it.
#[derive(Debug)]
enum Shape {
    Unsigned(u32),
    Signed(u32),
    Float(u32),
    Bool,
    Text(u64),
    Bytes(u64),
    Enum(Vec<(String, i128)>),
    Record(Vec<Field>),
    Union {
        tag: String,
        cases: Vec<(String, Option<Vec<Field>>)>,
    },
    List {
        min: u64,
        max: u64,
        of: Box<Shape>,
    },
    Nullable(Box<Shape>),
}

/// The description `text` (JSON), read as a type.
fn read_description(text: &str) -> Result<Shape, RecordCborRefusal> {
    let value = json::parse(text.trim_matches(json::is_json_whitespace)).map_err(|why| {
        RecordCborRefusal::at("", format!("has a description that is not JSON ({why})"))
    })?;
    shape_of(&value).map_err(|why| {
        RecordCborRefusal::at(
            "",
            format!("has a description the codec cannot read: {why}"),
        )
    })
}

fn whole(value: &Value) -> Result<u64, String> {
    match value {
        Value::Number(text) => text
            .parse::<u64>()
            .map_err(|_| format!("`{text}` is no count")),
        _ => Err("a count is no number".to_string()),
    }
}

fn shape_of(value: &Value) -> Result<Shape, String> {
    let Value::Object(members) = value else {
        return Err("a type is described by an object".to_string());
    };
    let [(kind, argument)] = members.as_slice() else {
        return Err("a type is described by an object of one member".to_string());
    };
    let bits = || {
        whole(argument).and_then(|bits| match bits {
            8 | 16 | 32 | 64 => Ok(bits as u32),
            other => Err(format!("{other} is no width of a number")),
        })
    };
    Ok(match kind.as_str() {
        "uint" => Shape::Unsigned(bits()?),
        "int" => Shape::Signed(bits()?),
        "float" => {
            let width = bits()?;
            if width != 32 && width != 64 {
                return Err(format!("{width} is no width of a float"));
            }
            Shape::Float(width)
        }
        "bool" => Shape::Bool,
        "text" => Shape::Text(whole(argument)?),
        "bytes" => Shape::Bytes(whole(argument)?),
        "enum" => {
            let Some(Value::Array(variants)) = argument.member("variants") else {
                return Err("an enum names its variants".to_string());
            };
            let mut named = Vec::with_capacity(variants.len());
            for variant in variants {
                let Value::Array(pair) = variant else {
                    return Err("a variant is a name and a number".to_string());
                };
                let [Value::Text(name), Value::Number(number)] = pair.as_slice() else {
                    return Err("a variant is a name and a number".to_string());
                };
                let number = number
                    .parse::<i128>()
                    .map_err(|_| format!("`{number}` is no number of a variant"))?;
                named.push((name.clone(), number));
            }
            Shape::Enum(named)
        }
        "record" => Shape::Record(fields_of(argument)?),
        "union" => {
            let Some(Value::Text(tag)) = argument.member("tag") else {
                return Err("a union names its tag".to_string());
            };
            let Some(Value::Array(cases)) = argument.member("cases") else {
                return Err("a union names its cases".to_string());
            };
            let mut read = Vec::with_capacity(cases.len());
            for case in cases {
                let Value::Array(pair) = case else {
                    return Err("a case is a name and its fields".to_string());
                };
                let [Value::Text(name), fields] = pair.as_slice() else {
                    return Err("a case is a name and its fields".to_string());
                };
                let fields = match fields {
                    Value::Null => None,
                    fields => Some(fields_of(fields)?),
                };
                read.push((name.clone(), fields));
            }
            Shape::Union {
                tag: tag.clone(),
                cases: read,
            }
        }
        "list" => {
            let (Some(min), Some(max), Some(of)) = (
                argument.member("min"),
                argument.member("max"),
                argument.member("of"),
            ) else {
                return Err("a list names its counts and its element".to_string());
            };
            Shape::List {
                min: whole(min)?,
                max: whole(max)?,
                of: Box::new(shape_of(of)?),
            }
        }
        "nullable" => Shape::Nullable(Box::new(shape_of(argument)?)),
        other => return Err(format!("`{other}` is no kind of type")),
    })
}

fn fields_of(value: &Value) -> Result<Vec<Field>, String> {
    let Value::Array(fields) = value else {
        return Err("a record's fields are an array".to_string());
    };
    let mut read = Vec::with_capacity(fields.len());
    for field in fields {
        let Value::Array(triple) = field else {
            return Err("a field is an id, whether it is optional, and a type".to_string());
        };
        let [Value::Text(id), Value::Bool(optional), shape] = triple.as_slice() else {
            return Err("a field is an id, whether it is optional, and a type".to_string());
        };
        read.push(Field {
            id: id.clone(),
            optional: *optional,
            shape: shape_of(shape)?,
        });
    }
    Ok(read)
}

/// `token` below `pointer`, as RFC 6901 joins reference tokens.
fn below(pointer: &str, token: &str) -> String {
    let escaped = token.replace('~', "~0").replace('/', "~1");
    format!("{pointer}/{escaped}")
}

/// A whole number a JSON value spells: a number, or the text of its digits,
/// which a 64-bit integer is written as.
fn whole_number(value: &Value) -> Option<i128> {
    match value {
        Value::Number(text) | Value::Text(text) => text.parse::<i128>().ok(),
        _ => None,
    }
}

/// The least and the greatest integer of `bits` bits, of either sign.
fn range(bits: u32, signed: bool) -> (i128, i128) {
    match (signed, bits) {
        (false, 64) => (0, i128::from(u64::MAX)),
        (false, bits) => (0, (1i128 << bits) - 1),
        (true, bits) => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
    }
}

/// Write the integer `n`, of either sign, in its shortest head.
fn write_integer(w: &mut VecSink<'_>, n: i128) {
    let written = if n < 0 {
        cbor::write_int(w, n as i64)
    } else {
        cbor::write_uint(w, n as u64)
    };
    infallible(written);
}

/// A write into a growing vector, which refuses nothing.
fn infallible(written: Result<(), core::convert::Infallible>) {
    if let Err(never) = written {
        match never {}
    }
}

/// The deterministic CBOR of the record `record_json` holds, the record's JSON
/// as a machine writes it, under the description `description` of its type.
///
/// # Errors
///
/// A description the codec cannot read, and a record that is no value of its
/// type (a list below its lower count, a value past a bound, a variant its
/// enum does not declare), are refused, naming the place.
pub fn encode(record_json: &str, description: &str) -> Result<Vec<u8>, RecordCborRefusal> {
    let shape = read_description(description)?;
    let value = json::parse(record_json.trim_matches(json::is_json_whitespace))
        .map_err(|why| RecordCborRefusal::at("", format!("is not JSON ({why})")))?;
    let mut out: Vec<u8> = Vec::new();
    let mut w = VecSink::new(&mut out);
    write_value(&shape, &value, "", &mut w)?;
    Ok(out)
}

/// [`encode`], as the byte string's Latin-1 text: each byte the character of
/// that code point, the text a request's byte string crosses as (SCE Accepted
/// Subset §2.12).
///
/// # Errors
///
/// What [`encode`] refuses.
pub fn encode_text(record_json: &str, description: &str) -> Result<String, RecordCborRefusal> {
    encode(record_json, description)
        .map(|bytes| crate::event_payload::bytes_as_payload_text(&bytes))
}

fn write_value(
    shape: &Shape,
    value: &Value,
    at: &str,
    w: &mut VecSink<'_>,
) -> Result<(), RecordCborRefusal> {
    let wrong = |what: &str| RecordCborRefusal::at(at, format!("is not {what}"));
    match shape {
        Shape::Unsigned(bits) | Shape::Signed(bits) => {
            let signed = matches!(shape, Shape::Signed(_));
            let n = whole_number(value).ok_or_else(|| wrong("a whole number"))?;
            let (least, greatest) = range(*bits, signed);
            if !(least..=greatest).contains(&n) {
                return Err(RecordCborRefusal::at(
                    at,
                    format!("({n}) does not fit the width its type declares"),
                ));
            }
            write_integer(w, n);
        }
        Shape::Float(bits) => {
            let Value::Number(text) = value else {
                return Err(wrong("a number"));
            };
            let x: f64 = text.parse().map_err(|_| wrong("a number"))?;
            let x = if *bits == 32 { f64::from(x as f32) } else { x };
            if !x.is_finite() {
                return Err(RecordCborRefusal::at(at, "is not a finite number"));
            }
            infallible(cbor::write_float(w, x));
        }
        Shape::Bool => {
            let Value::Bool(b) = value else {
                return Err(wrong("a truth value"));
            };
            infallible(cbor::write_bool(w, *b));
        }
        Shape::Text(max) => {
            let Value::Text(text) = value else {
                return Err(wrong("a text"));
            };
            if text.len() as u64 > *max {
                return Err(RecordCborRefusal::at(
                    at,
                    format!("is {} bytes, past the {max} its type declares", text.len()),
                ));
            }
            infallible(cbor::write_text(w, text));
        }
        Shape::Bytes(max) => {
            let Value::Text(text) = value else {
                return Err(wrong("a byte string"));
            };
            let bytes = latin1_bytes(text).ok_or_else(|| {
                RecordCborRefusal::at(at, "carries a character above U+00FF, which no byte spells")
            })?;
            if bytes.len() as u64 > *max {
                return Err(RecordCborRefusal::at(
                    at,
                    format!("is {} bytes, past the {max} its type declares", bytes.len()),
                ));
            }
            infallible(cbor::write_bytes(w, &bytes));
        }
        Shape::Enum(variants) => {
            let Value::Text(name) = value else {
                return Err(wrong("the name of a variant"));
            };
            let number = variants
                .iter()
                .find(|(declared, _)| declared == name)
                .map(|(_, number)| *number)
                .ok_or_else(|| RecordCborRefusal::at(at, format!("({name}) names no variant")))?;
            write_integer(w, number);
        }
        Shape::Record(fields) => {
            let Value::Object(members) = value else {
                return Err(wrong("a record"));
            };
            write_record(fields, members, at, w)?;
        }
        Shape::Union { tag, cases } => {
            let Value::Object(members) = value else {
                return Err(wrong("a union"));
            };
            let Some(Value::Text(variant)) = member(members, tag) else {
                return Err(RecordCborRefusal::at(
                    &below(at, tag),
                    "is not the name of a case",
                ));
            };
            let (name, fields) =
                cases
                    .iter()
                    .find(|(name, _)| name == variant)
                    .ok_or_else(|| {
                        RecordCborRefusal::at(&below(at, tag), format!("({variant}) names no case"))
                    })?;
            infallible(cbor::write_variant(w, name));
            match fields {
                Some(fields) => write_record(fields, members, at, w)?,
                None => infallible(cbor::write_map_head(w, 0)),
            }
        }
        Shape::List { min, max, of } => {
            let Value::Array(items) = value else {
                return Err(wrong("a list"));
            };
            let count = items.len() as u64;
            if !(*min..=*max).contains(&count) {
                return Err(RecordCborRefusal::at(
                    at,
                    format!("holds {count} elements, outside the {min} to {max} its type declares"),
                ));
            }
            infallible(cbor::write_array_head(w, count));
            for (index, item) in items.iter().enumerate() {
                write_value(of, item, &below(at, &index.to_string()), w)?;
            }
        }
        Shape::Nullable(of) => {
            if matches!(value, Value::Null) {
                infallible(cbor::write_null(w));
            } else {
                write_value(of, value, at, w)?;
            }
        }
    }
    Ok(())
}

/// Write the map of the fields of `members` a record of `fields` holds present,
/// in the order the description gives them, which is the order their keys
/// encode.
fn write_record(
    fields: &[Field],
    members: &[(String, Value)],
    at: &str,
    w: &mut VecSink<'_>,
) -> Result<(), RecordCborRefusal> {
    let mut present: Vec<(&Field, &Value)> = Vec::with_capacity(fields.len());
    for field in fields {
        match member(members, &field.id) {
            Some(Value::Null) if field.optional => {}
            Some(value) => present.push((field, value)),
            None if field.optional => {}
            None => return Err(RecordCborRefusal::at(&below(at, &field.id), "is missing")),
        }
    }
    infallible(cbor::write_map_head(w, present.len() as u64));
    for (field, value) in present {
        infallible(cbor::write_text(w, &field.id));
        write_value(&field.shape, value, &below(at, &field.id), w)?;
    }
    Ok(())
}

fn member<'v>(members: &'v [(String, Value)], name: &str) -> Option<&'v Value> {
    members
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

/// The bytes a Latin-1 text spells, one a character, or `None` for a text
/// with a character past U+00FF.
fn latin1_bytes(text: &str) -> Option<Vec<u8>> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).ok())
        .collect()
}

/// The record `bytes` hold, the deterministic CBOR or any encoding of it within
/// the profile of definite lengths, under the description `description` of its
/// type, as the JSON a machine reads a record from.
///
/// # Errors
///
/// A description the codec cannot read, and bytes that are no value of the
/// record, bytes after it included, are refused, naming the place.
pub fn decode(bytes: &[u8], description: &str) -> Result<String, RecordCborRefusal> {
    let shape = read_description(description)?;
    let mut cursor = SceCursor::new(bytes);
    let value = read_value(&shape, &mut cursor, "")?;
    if cursor.remaining() != 0 {
        return Err(RecordCborRefusal::at(
            "",
            format!(
                "is followed by {} bytes that are no part of it",
                cursor.remaining()
            ),
        ));
    }
    Ok(json::write(&value))
}

fn read_value(shape: &Shape, c: &mut SceCursor<'_>, at: &str) -> Result<Value, RecordCborRefusal> {
    let refuse = |kind: CodecError| RecordCborRefusal::codec(at, kind);
    Ok(match shape {
        Shape::Unsigned(64) => Value::Text(cbor::read_uint(c).map_err(refuse)?.to_string()),
        Shape::Unsigned(bits) => {
            let (_, greatest) = range(*bits, false);
            Value::Number(
                cbor::read_uint_upto(c, greatest as u64)
                    .map_err(refuse)?
                    .to_string(),
            )
        }
        Shape::Signed(bits) => {
            let (least, greatest) = range(*bits, true);
            let n = cbor::read_int_within(c, least as i64, greatest as i64).map_err(refuse)?;
            if *bits == 64 {
                Value::Text(n.to_string())
            } else {
                Value::Number(n.to_string())
            }
        }
        Shape::Float(bits) => {
            let x = if *bits == 32 {
                f64::from(cbor::read_float32(c).map_err(refuse)?)
            } else {
                cbor::read_float(c).map_err(refuse)?
            };
            if !x.is_finite() {
                return Err(RecordCborRefusal::at(
                    at,
                    "is a float that is not finite, which no payload spells",
                ));
            }
            Value::Number(json::number_text(x))
        }
        Shape::Bool => Value::Bool(cbor::read_bool(c).map_err(refuse)?),
        Shape::Text(max) => {
            let text = cbor::read_text(c, Some(*max as usize)).map_err(refuse)?;
            if text.contains('\0') {
                return Err(RecordCborRefusal::at(
                    at,
                    "carries U+0000, which no text holds",
                ));
            }
            Value::Text(text.to_string())
        }
        Shape::Bytes(max) => {
            let bytes = cbor::read_bytes(c, Some(*max as usize)).map_err(refuse)?;
            Value::Text(crate::event_payload::bytes_as_payload_text(bytes))
        }
        Shape::Enum(variants) => {
            let (major, argument) = cbor::read_head(c).map_err(refuse)?;
            let number = match major {
                0 => i128::from(argument),
                1 => -1 - i128::from(argument),
                _ => return Err(refuse(CodecError::CborWrongType)),
            };
            let name = variants
                .iter()
                .find(|(_, declared)| *declared == number)
                .map(|(name, _)| name.clone())
                .ok_or_else(|| refuse(CodecError::UndeclaredEnumValue))?;
            Value::Text(name)
        }
        Shape::Record(fields) => Value::Object(read_record(fields, c, at)?),
        Shape::Union { tag, cases } => {
            if cbor::read_map_len(c).map_err(refuse)? != 1 {
                return Err(refuse(CodecError::CborUnionEntries));
            }
            let variant = cbor::read_text(c, None).map_err(refuse)?.to_string();
            let (_, fields) = cases
                .iter()
                .find(|(name, _)| *name == variant)
                .ok_or_else(|| refuse(CodecError::CborUnknownVariant))?;
            let mut members = match fields {
                Some(fields) => read_record(fields, c, &below(at, &variant))?,
                None => {
                    cbor::read_empty_case(c).map_err(refuse)?;
                    Vec::new()
                }
            };
            members.push((tag.clone(), Value::Text(variant)));
            Value::Object(members)
        }
        Shape::List { min, max, of } => {
            let count = cbor::read_array_len(c).map_err(refuse)?;
            if !(*min..=*max).contains(&count) {
                return Err(refuse(CodecError::CborOutOfRange));
            }
            let mut items = Vec::new();
            for index in 0..count {
                items.push(read_value(of, c, &below(at, &index.to_string()))?);
            }
            Value::Array(items)
        }
        Shape::Nullable(of) => {
            if cbor::take_null(c).map_err(refuse)? {
                Value::Null
            } else {
                read_value(of, c, at)?
            }
        }
    })
}

/// Read the map of a record of `fields`: its keys in any order, a key the
/// record does not declare stepped over with its value, a declared key given
/// twice and a field that is not optional missing refused. Every field is in
/// the object read, one absent `null`.
fn read_record(
    fields: &[Field],
    c: &mut SceCursor<'_>,
    at: &str,
) -> Result<Vec<(String, Value)>, RecordCborRefusal> {
    let refuse = |kind: CodecError| RecordCborRefusal::codec(at, kind);
    let count = cbor::read_map_len(c).map_err(refuse)?;
    let mut held: Vec<Option<Value>> = fields.iter().map(|_| None).collect();
    for _ in 0..count {
        let key = cbor::read_text(c, None).map_err(refuse)?;
        match fields.iter().position(|field| field.id == key) {
            Some(index) => {
                if held[index].is_some() {
                    return Err(RecordCborRefusal::codec(
                        &below(at, key),
                        CodecError::CborDuplicateKey,
                    ));
                }
                held[index] = Some(read_value(&fields[index].shape, c, &below(at, key))?);
            }
            None => cbor::skip(c).map_err(refuse)?,
        }
    }
    let mut members = Vec::with_capacity(fields.len());
    for (field, value) in fields.iter().zip(held) {
        let value = match value {
            Some(value) => value,
            None if field.optional => Value::Null,
            None => {
                return Err(RecordCborRefusal::codec(
                    &below(at, &field.id),
                    CodecError::CborRequiredKeyMissing,
                ))
            }
        };
        members.push((field.id.clone(), value));
    }
    Ok(members)
}

/// The completion `data` of a verification or an opening whose invoke names a
/// record, with the bytes it carries read as that record: what the machine's
/// lift of the completion reads (docs/adr/0109, D9).
///
/// `field` is the result's byte string (`payload`, `plaintext`), `answered`
/// its truth field that says the bytes are there (`valid`, `opened`),
/// `description` the record's type, and `failure` the completion the
/// operation's failure `not_a_record` is, which the build writes.
///
/// - A completion whose `error` is not empty, or whose `answered` is not
///   `true`, carries no record: `field` is `null`, the rest as it came.
/// - One that answered: `field` is the record its bytes hold, as JSON.
/// - One whose bytes are no value of the record: `failure`, whole.
///
/// A completion that is no JSON object, or that carries no text under `field`,
/// is handed on as it came, for the lift to refuse as it refuses any data its
/// record does not fit.
pub fn open_completion(
    data: &str,
    field: &str,
    answered: &str,
    description: &str,
    failure: &str,
) -> String {
    let Ok(Value::Object(mut members)) = json::parse(data.trim_matches(json::is_json_whitespace))
    else {
        return data.to_string();
    };
    let failed = member(&members, "error")
        .is_some_and(|error| !matches!(error, Value::Text(code) if code.is_empty()));
    let took = matches!(member(&members, answered), Some(Value::Bool(true)));
    let Some(slot) = members.iter().position(|(key, _)| key == field) else {
        return data.to_string();
    };
    if failed || !took {
        members[slot].1 = Value::Null;
        return json::write(&Value::Object(members));
    }
    let Value::Text(text) = &members[slot].1 else {
        return data.to_string();
    };
    let Some(bytes) = latin1_bytes(text) else {
        return failure.to_string();
    };
    match decode(&bytes, description).and_then(|record| {
        json::parse(&record).map_err(|why| RecordCborRefusal::at("", why.to_string()))
    }) {
        Ok(record) => {
            members[slot].1 = record;
            json::write(&Value::Object(members))
        }
        Err(_) => failure.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, encode_text, open_completion};

    /// A record of an id, an optional text, a level of an enum of a signed
    /// carrier, a list of byte strings and a nested record, its fields in the
    /// order their keys encode.
    const NOTE: &str = r#"{"record":[["id",false,{"uint":32}],["keys",false,{"list":{"max":2,"min":0,"of":{"bytes":4}}}],["memo",true,{"text":8}],["level",false,{"enum":{"closed":true,"variants":[["low",0],["fault",-1]]}}],["title",false,{"text":16}]]}"#;

    #[test]
    fn a_record_is_written_in_the_order_its_keys_encode() {
        let bytes = encode(
            r#"{"title":"Hi","memo":null,"level":"fault","keys":["\u0001"],"id":500}"#,
            NOTE,
        )
        .expect("a record of its type");
        // a4: four fields present (memo absent); "id" 500; "keys" [h'01'];
        // "level" -1; "title" "Hi".
        assert_eq!(
            bytes,
            vec![
                0xa4, 0x62, b'i', b'd', 0x19, 0x01, 0xf4, 0x64, b'k', b'e', b'y', b's', 0x81, 0x41,
                0x01, 0x65, b'l', b'e', b'v', b'e', b'l', 0x20, 0x65, b't', b'i', b't', b'l', b'e',
                0x62, b'H', b'i',
            ]
        );
    }

    #[test]
    fn a_record_reads_back_as_the_json_it_was_written_from() {
        let written = r#"{"id":7,"keys":[],"memo":"m","level":"low","title":""}"#;
        let bytes = encode(written, NOTE).expect("a record of its type");
        let read = decode(&bytes, NOTE).expect("the bytes of a record");
        assert_eq!(read, written);
    }

    #[test]
    fn a_value_outside_its_type_has_no_encoding() {
        for (record, place) in [
            (
                r#"{"id":-1,"keys":[],"memo":null,"level":"low","title":""}"#,
                "/id",
            ),
            (
                r#"{"id":1,"keys":["a","b","c"],"memo":null,"level":"low","title":""}"#,
                "/keys",
            ),
            (
                r#"{"id":1,"keys":[],"memo":null,"level":"high","title":""}"#,
                "/level",
            ),
            (r#"{"id":1,"keys":[],"memo":null,"level":"low"}"#, "/title"),
            (
                r#"{"id":1,"keys":["Ā"],"memo":null,"level":"low","title":""}"#,
                "/keys/0",
            ),
        ] {
            let refused = encode(record, NOTE).expect_err("no value of its type");
            assert_eq!(refused.pointer, place, "{refused}");
        }
    }

    #[test]
    fn bytes_that_are_no_value_of_the_record_are_refused() {
        let good = encode(
            r#"{"id":1,"keys":[],"memo":null,"level":"low","title":"t"}"#,
            NOTE,
        )
        .expect("a record of its type");
        // A trailing byte, a map cut short, a text where the id stands, the id
        // given twice and a level no variant declares.
        let mut trailing = good.clone();
        trailing.push(0x00);
        let cut = good[..good.len() - 1].to_vec();
        let wrong = vec![0xa1, 0x62, b'i', b'd', 0x61, b'x'];
        let twice = vec![0xa2, 0x62, b'i', b'd', 0x01, 0x62, b'i', b'd', 0x02];
        let undeclared = vec![
            0xa4, 0x62, b'i', b'd', 0x01, 0x64, b'k', b'e', b'y', b's', 0x80, 0x65, b'l', b'e',
            b'v', b'e', b'l', 0x05, 0x65, b't', b'i', b't', b'l', b'e', 0x60,
        ];
        for bytes in [trailing, cut, wrong, twice, undeclared] {
            assert!(decode(&bytes, NOTE).is_err(), "{bytes:02x?} was read");
        }
    }

    #[test]
    fn the_text_of_the_bytes_is_their_latin1_spelling() {
        let text = encode_text(
            r#"{"id":1,"keys":[],"memo":null,"level":"low","title":""}"#,
            NOTE,
        )
        .expect("a record of its type");
        assert!(text.starts_with('\u{a4}'), "{text:?}");
        assert!(text.chars().all(|c| u32::from(c) <= 0xff));
    }

    const FAILURE: &str = r#"{"error":"not_a_record","key_id":"","payload":null,"valid":false}"#;

    #[test]
    fn a_completion_that_verified_reads_its_payload_as_the_record() {
        let text = encode_text(
            r#"{"id":3,"keys":[],"memo":null,"level":"low","title":"x"}"#,
            NOTE,
        )
        .expect("a record of its type");
        let data = format!(
            r#"{{"error":"","valid":true,"key_id":"k","payload":{}}}"#,
            crate::json::quote(&text)
        );
        let opened = open_completion(&data, "payload", "valid", NOTE, FAILURE);
        assert_eq!(
            opened,
            r#"{"error":"","valid":true,"key_id":"k","payload":{"id":3,"keys":[],"memo":null,"level":"low","title":"x"}}"#
        );
    }

    #[test]
    fn a_completion_that_did_not_verify_carries_no_record() {
        let data = r#"{"error":"","valid":false,"key_id":"","payload":""}"#;
        assert_eq!(
            open_completion(data, "payload", "valid", NOTE, FAILURE),
            r#"{"error":"","valid":false,"key_id":"","payload":null}"#
        );
        let failed = r#"{"error":"unknown_key","valid":false,"key_id":"","payload":""}"#;
        assert_eq!(
            open_completion(failed, "payload", "valid", NOTE, FAILURE),
            r#"{"error":"unknown_key","valid":false,"key_id":"","payload":null}"#
        );
    }

    #[test]
    fn bytes_that_verified_and_are_no_record_are_the_failure() {
        let data = r#"{"error":"","valid":true,"key_id":"k","payload":"hello"}"#;
        assert_eq!(
            open_completion(data, "payload", "valid", NOTE, FAILURE),
            FAILURE
        );
    }
}
