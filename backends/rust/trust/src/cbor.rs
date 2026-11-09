// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The deterministic CBOR of a typed value: the bytes a signature covers from ADR 0085's domain
//! bump on (docs/adr/0086, section 9). [`crate::message`] writes a change, `sce-trust/event/3`, and
//! the operation that carries it with this encoding: the bump switched the domain and the encoding
//! at once.
//!
//! A value is written by its type, not by its own shape: [`encode`] takes a [`Value`] and the
//! [`ValueType`] it is a value of, and writes the core deterministic encoding of RFC 8949 section
//! 4.2.1, so two hosts that hold one typed value write one string of bytes whatever text their
//! delivery carried it in, and any conforming CBOR decoder reads it back.
//!
//! * An integer is major type 0, or major type 1 below zero, with the shortest head that holds it
//!   (preferred serialization, RFC 8949 sections 4.1 and 4.2.1).
//! * A `float32` or `float64` is the shortest of half, single and double precision that holds it
//!   exactly (RFC 8949 section 4.2.1 with section 4.1); a NaN is `f9 7e 00`, the one NaN section
//!   4.2.2 leaves a protocol to choose.
//! * `bool` is `f4` or `f5`; an enum is its declared number, an integer of its underlying type.
//! * A text is major type 3 and a byte string major type 2, its bytes as they are.
//! * An absent optional is `null` (`f6`), and its key stays in its record's map.
//! * A record is a map of every field its schema declares, keyed by the field's id as text and
//!   sorted by the bytes of each key's encoding (RFC 8949 section 4.2.1); a field a later revision
//!   added and the value lacks is written as its default (docs/adr/0086, section 10), and a key the
//!   schema does not declare is not written.
//! * A list is an array of definite length, its elements in their order.
//!
//! The type is the one a schema unit declares as a constant ([`ValueType`], re-exported from
//! `sce_forge_runtime::value_type`, which a generated unit depends on without depending on this
//! crate): the runtime's mirror of the build's one value type, in which `optional<optional<T>>`
//! has no spelling.
//!
//! A value outside its type is refused before any of it is signed ([`Refused`]), naming the place by
//! its path as a lift of a delivery names it (docs/adr/0086, section 4.2).

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt;

use sce_forge_runtime::cbor::{
    write_bool, write_bytes, write_head, write_map_head, write_text, MAJOR_SIMPLE, MAJOR_UNSIGNED,
};
use sce_forge_runtime::codec::{widen_usize, SceSink, VecSink};
pub use sce_forge_runtime::value_type::{
    EnumType, Field, FieldDefault, Integer, RecordType, Shape, ValueType, Variant,
};

/// Major type 1: the negative integer `-1 - n` for the argument `n` (RFC 8949 section 3.1).
const MAJOR_NEGATIVE: u8 = 1;

/// Major type 4: an array of the argument's count of items (RFC 8949 section 3.1).
const MAJOR_ARRAY: u8 = 4;

/// `null`, simple value 22 (RFC 8949 section 3.3).
const NULL: u8 = (MAJOR_SIMPLE << 5) | 22;

/// The initial byte of a half, a single and a double precision float (RFC 8949 section 3.3).
const HALF: u8 = (MAJOR_SIMPLE << 5) | 25;
const SINGLE: u8 = (MAJOR_SIMPLE << 5) | 26;
const DOUBLE: u8 = (MAJOR_SIMPLE << 5) | 27;

/// The one NaN written: the quiet NaN of half precision (docs/adr/0086, section 9.2).
const NAN: [u8; 3] = [HALF, 0x7e, 0x00];

/// A value of a type of the grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A value of an integer type, or an enum's number: one carrier for `uint64`'s greatest and
    /// `int64`'s least, as the build holds a variant's number.
    Integer(i128),
    /// A value of `float32` or `float64`.
    Float(f64),
    Bool(bool),
    Text(String),
    Bytes(Vec<u8>),
    /// An optional that holds no value.
    Absent,
    /// A record: each field's value by its id.
    Record(BTreeMap<String, Value>),
    List(Vec<Value>),
}

impl Value {
    /// The value a field's `sce:default` states.
    pub fn of_default(default: FieldDefault) -> Self {
        match default {
            FieldDefault::Integer(n) => Self::Integer(n),
            FieldDefault::Float(x) => Self::Float(x),
            FieldDefault::Bool(b) => Self::Bool(b),
            FieldDefault::Text(text) => Self::Text(text.to_string()),
            FieldDefault::Bytes(bytes) => Self::Bytes(bytes.to_vec()),
        }
    }
}

/// A step from a value to one of its parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The field of a record with this id.
    Field(&'static str),
    /// The element of a list at this index.
    Element(usize),
}

/// What a bound counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// The bytes of a text or a byte string.
    Bytes,
    /// The elements of a list.
    Elements,
}

/// Why a value is not encoded.
#[derive(Debug, Clone, PartialEq)]
pub enum Why {
    /// The value is not of the shape its type names (`expected`, as the grammar spells it).
    NotOf { expected: &'static str },
    /// An integer, or an enum's number, outside its integer.
    OutOfRange { value: i128, integer: Integer },
    /// A `float32` that no single precision float holds exactly.
    NotSingle { value: f64 },
    /// A number no variant of a closed enum declares.
    Undeclared { value: i128 },
    /// A text, byte string or list longer than its bound.
    PastBound { len: usize, max: u32, unit: Unit },
    /// Absent, where the type is not optional.
    Absent,
    /// A record lacks a field of the first revision of its schema.
    Missing,
    /// A record lacks a field a later revision added that is neither optional, a list, nor given
    /// a default.
    NoDefault,
    /// The record's type declares the field's id twice, which no map may hold.
    DeclaredTwice,
}

/// A value refused, with the path from the value given to the place that is not of its type.
#[derive(Debug, Clone, PartialEq)]
pub struct Refused {
    pub at: Vec<Step>,
    pub why: Why,
}

impl Refused {
    /// The path as a JSON Pointer (RFC 6901) without its leading `/`, as a refusal of a delivery
    /// spells it: `items/2/tags/0`, and the empty text for the value itself.
    pub fn pointer(&self) -> String {
        let steps: Vec<String> = self
            .at
            .iter()
            .map(|step| match step {
                Step::Field(id) => id.replace('~', "~0").replace('/', "~1"),
                Step::Element(index) => index.to_string(),
            })
            .collect();
        steps.join("/")
    }
}

impl fmt::Display for Refused {
    /// The reason as a sentence, its place first (docs/adr/0086, section 4.2).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let place = self.pointer();
        match &self.why {
            Why::NotOf { expected } => write!(f, "'{place}' is not a value of `{expected}`"),
            Why::OutOfRange { value, integer } => {
                write!(f, "'{place}' is {value}, outside `{integer}`")
            }
            Why::NotSingle { value } => {
                write!(f, "'{place}' is {value}, which no float32 holds exactly")
            }
            Why::Undeclared { value } => {
                write!(
                    f,
                    "'{place}' is {value}, which no variant of its enum declares"
                )
            }
            Why::PastBound {
                len,
                max,
                unit: Unit::Bytes,
            } => write!(
                f,
                "'{place}' is {len} bytes, past the {max} its type declares"
            ),
            Why::PastBound {
                len,
                max,
                unit: Unit::Elements,
            } => write!(
                f,
                "'{place}' holds {len} elements, past the {max} its type declares"
            ),
            Why::Absent => write!(f, "'{place}' is null, and its type is not optional"),
            Why::Missing => write!(
                f,
                "'{place}' is missing, and every value holds a field of the first revision"
            ),
            Why::NoDefault => write!(f, "'{place}' is missing, and its schema gives no default"),
            Why::DeclaredTwice => write!(f, "'{place}' is declared twice by its record"),
        }
    }
}

impl std::error::Error for Refused {}

/// The core deterministic CBOR encoding (RFC 8949 section 4.2.1) of `value` as a value of `ty`, or
/// the refusal of the first place where it is not one.
pub fn encode(value: &Value, ty: &ValueType) -> Result<Vec<u8>, Refused> {
    let mut out = Vec::new();
    {
        let mut encoder = Encoder {
            sink: VecSink::new(&mut out),
            path: Vec::new(),
        };
        encoder.value(value, ty)?;
    }
    Ok(out)
}

/// What a write into a growing vector returns: it cannot fail.
fn put(written: Result<(), Infallible>) {
    written.unwrap_or_else(|never| match never {});
}

/// The encoding of a record's key: the field's id as a text (RFC 8949 section 3.1, major type 3).
fn key_of(id: &str) -> Vec<u8> {
    let mut key = Vec::new();
    put(write_text(&mut VecSink::new(&mut key), id));
    key
}

/// The half precision float that is `x` exactly, as its bits, or `None` when no half is (IEEE 754
/// binary16: 5 bits of exponent, 10 of fraction, subnormal below 2^-14). `x` is not a NaN.
fn half_of(x: f64) -> Option<u16> {
    let bits = x.to_bits();
    let sign = ((bits >> 48) & 0x8000) as u16;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    if exponent == 0x7ff {
        // An infinity; a NaN is written before this is asked.
        return (fraction == 0).then_some(sign | 0x7c00);
    }
    if exponent == 0 {
        // Zero, or a double's subnormal, which is far below a half's least.
        return (fraction == 0).then_some(sign);
    }
    let power = exponent - 1023;
    if (-14..=15).contains(&power) {
        // A half's normal number keeps the top 10 bits of the fraction and nothing below them.
        if (fraction & ((1 << 42) - 1)) != 0 {
            return None;
        }
        return Some(sign | (((power + 15) as u16) << 10) | (fraction >> 42) as u16);
    }
    if (-24..=-15).contains(&power) {
        // A half's subnormal number is k * 2^-24 for k from 1 to 1023.
        let significand = fraction | (1 << 52);
        let shift = (28 - power) as u32;
        if (significand & ((1 << shift) - 1)) != 0 {
            return None;
        }
        return Some(sign | (significand >> shift) as u16);
    }
    None
}

/// The single precision float that is `x` exactly, or `None` when no single is. `x` is not a NaN.
fn single_of(x: f64) -> Option<f32> {
    let single = x as f32;
    (f64::from(single) == x).then_some(single)
}

/// One encoding in progress: the bytes written so far and the path to the value being written.
struct Encoder<'a> {
    sink: VecSink<'a>,
    path: Vec<Step>,
}

impl Encoder<'_> {
    fn refuse(&self, why: Why) -> Refused {
        Refused {
            at: self.path.clone(),
            why,
        }
    }

    fn value(&mut self, value: &Value, ty: &ValueType) -> Result<(), Refused> {
        match (ty, value) {
            (ValueType::Optional(_), Value::Absent) => {
                put(self.sink.write_u8(NULL));
                Ok(())
            }
            (ValueType::Required(_), Value::Absent) => Err(self.refuse(Why::Absent)),
            (ValueType::Required(shape) | ValueType::Optional(shape), present) => {
                self.shape(present, shape)
            }
        }
    }

    fn shape(&mut self, value: &Value, shape: &Shape) -> Result<(), Refused> {
        match (shape, value) {
            (Shape::Integer(integer), Value::Integer(n)) => self.integer(*n, *integer),
            (Shape::Float32, Value::Float(x)) => {
                if !x.is_nan() && single_of(*x).is_none() {
                    return Err(self.refuse(Why::NotSingle { value: *x }));
                }
                self.float(*x);
                Ok(())
            }
            (Shape::Float64, Value::Float(x)) => {
                self.float(*x);
                Ok(())
            }
            (Shape::Bool, Value::Bool(b)) => {
                put(write_bool(&mut self.sink, *b));
                Ok(())
            }
            (Shape::Enum(enumeration), Value::Integer(n)) => {
                if enumeration.strict && !enumeration.variants.iter().any(|v| v.value == *n) {
                    return Err(self.refuse(Why::Undeclared { value: *n }));
                }
                self.integer(*n, enumeration.underlying)
            }
            (Shape::Text { max }, Value::Text(text)) => {
                self.within(text.len(), *max, Unit::Bytes)?;
                put(write_text(&mut self.sink, text));
                Ok(())
            }
            (Shape::Bytes { max }, Value::Bytes(bytes)) => {
                self.within(bytes.len(), *max, Unit::Bytes)?;
                put(write_bytes(&mut self.sink, bytes));
                Ok(())
            }
            (Shape::Record(record), Value::Record(fields)) => self.record(fields, record),
            (Shape::List { elem, max }, Value::List(items)) => {
                // Counted before any element is written, so an element past the bound is never
                // looked at (docs/adr/0086, section 4.1).
                self.within(items.len(), *max, Unit::Elements)?;
                put(write_head(
                    &mut self.sink,
                    MAJOR_ARRAY,
                    widen_usize(items.len()),
                ));
                for (index, item) in items.iter().enumerate() {
                    self.path.push(Step::Element(index));
                    self.value(item, elem)?;
                    self.path.pop();
                }
                Ok(())
            }
            (shape, _) => Err(self.refuse(Why::NotOf {
                expected: shape.constructor(),
            })),
        }
    }

    /// Refuse a length past its bound.
    fn within(&self, len: usize, max: u32, unit: Unit) -> Result<(), Refused> {
        if widen_usize(len) > u64::from(max) {
            return Err(self.refuse(Why::PastBound { len, max, unit }));
        }
        Ok(())
    }

    /// An integer: major type 0 for `n` of zero or more, major type 1 with the argument `-1 - n`
    /// below zero, each with its shortest head (RFC 8949 sections 3.1 and 4.2.1).
    fn integer(&mut self, n: i128, integer: Integer) -> Result<(), Refused> {
        let (least, greatest) = integer.range();
        if n < least || n > greatest {
            return Err(self.refuse(Why::OutOfRange { value: n, integer }));
        }
        let (major, argument) = if n < 0 {
            (MAJOR_NEGATIVE, -1 - n)
        } else {
            (MAJOR_UNSIGNED, n)
        };
        // Within `int64` and `uint64`, so the argument is below 2^64.
        put(write_head(&mut self.sink, major, argument as u64));
        Ok(())
    }

    /// A float in the shortest of half, single and double precision that holds it exactly, and a
    /// NaN as `f9 7e 00` (RFC 8949 sections 4.1 and 4.2.1).
    fn float(&mut self, x: f64) {
        if x.is_nan() {
            put(self.sink.write_bytes(&NAN));
        } else if let Some(half) = half_of(x) {
            put(self.sink.write_u8(HALF));
            put(self.sink.write_bytes(&half.to_be_bytes()));
        } else if let Some(single) = single_of(x) {
            put(self.sink.write_u8(SINGLE));
            put(self.sink.write_bytes(&single.to_bits().to_be_bytes()));
        } else {
            put(self.sink.write_u8(DOUBLE));
            put(self.sink.write_bytes(&x.to_bits().to_be_bytes()));
        }
    }

    /// A record: a map of every field its type declares, in the order of the bytes of each key's
    /// encoding (RFC 8949 section 4.2.1), which puts a shorter key first.
    fn record(
        &mut self,
        fields: &BTreeMap<String, Value>,
        record: &RecordType,
    ) -> Result<(), Refused> {
        let mut keyed: Vec<(Vec<u8>, &Field)> = record
            .fields
            .iter()
            .map(|field| (key_of(field.id), field))
            .collect();
        keyed.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        if let Some(pair) = keyed.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            self.path.push(Step::Field(pair[0].1.id));
            return Err(self.refuse(Why::DeclaredTwice));
        }
        put(write_map_head(&mut self.sink, widen_usize(keyed.len())));
        for (key, field) in &keyed {
            put(self.sink.write_bytes(key));
            self.path.push(Step::Field(field.id));
            match fields.get(field.id) {
                Some(value) => self.value(value, &field.ty)?,
                None => self.lacked(field)?,
            }
            self.path.pop();
        }
        Ok(())
    }

    /// A field the value lacks: its default, when a later revision added it (docs/adr/0086,
    /// section 10), so that a delivery of an earlier revision and one of the later that carries the
    /// default are written alike.
    fn lacked(&mut self, field: &Field) -> Result<(), Refused> {
        if field.since <= 1 {
            return Err(self.refuse(Why::Missing));
        }
        let implied = match (field.default, &field.ty) {
            (Some(default), _) => Value::of_default(default),
            (None, ValueType::Optional(_)) => Value::Absent,
            (None, ValueType::Required(Shape::List { .. })) => Value::List(Vec::new()),
            (None, ValueType::Required(_)) => return Err(self.refuse(Why::NoDefault)),
        };
        self.value(&implied, &field.ty)
    }
}
