// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The type of a value, as generated code declares it: the runtime's mirror of the build's one value
//! type (`sce-build`'s `forge::value_type::ValueType`, docs/adr/0086 section 1).
//!
//! ```text
//! T ::= uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64 | float32 | float64 | bool
//!     | enum:<alias>
//!     | string[B] | bytes[B]
//!     | optional<T>
//!     | record:<alias>
//!     | list<T>[B]
//! ```
//!
//! One constructor per line of the grammar, each taking every type at any depth, and every bound the
//! number a build holds after its limit name and `--limit`. A type is built only of `&'static`
//! parts, numbers and texts, with no heap, so a schema unit declares its type as a constant on every
//! profile this crate builds for, the bare-metal one included; what reads a value by it (the
//! deterministic CBOR a signature covers, docs/adr/0086 section 9) reads that constant.
//!
//! `optional<optional<T>>`, which the grammar refuses because no wire tells its two absences apart
//! (docs/adr/0086, section 1.5), has no spelling here: [`ValueType::Optional`] holds a [`Shape`],
//! and no shape is optional.

use core::fmt;

/// One of the eight fixed-width integers of the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integer {
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Int8,
    Int16,
    Int32,
    Int64,
}

impl Integer {
    /// The least and the greatest value of the integer, in the one carrier that holds both ends of
    /// every width.
    pub fn range(self) -> (i128, i128) {
        match self {
            Self::Uint8 => (0, i128::from(u8::MAX)),
            Self::Uint16 => (0, i128::from(u16::MAX)),
            Self::Uint32 => (0, i128::from(u32::MAX)),
            Self::Uint64 => (0, i128::from(u64::MAX)),
            Self::Int8 => (i128::from(i8::MIN), i128::from(i8::MAX)),
            Self::Int16 => (i128::from(i16::MIN), i128::from(i16::MAX)),
            Self::Int32 => (i128::from(i32::MIN), i128::from(i32::MAX)),
            Self::Int64 => (i128::from(i64::MIN), i128::from(i64::MAX)),
        }
    }

    /// The integer as the grammar spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uint8 => "uint8",
            Self::Uint16 => "uint16",
            Self::Uint32 => "uint32",
            Self::Uint64 => "uint64",
            Self::Int8 => "int8",
            Self::Int16 => "int16",
            Self::Int32 => "int32",
            Self::Int64 => "int64",
        }
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A type of the grammar: a shape that every value has, or a shape that a value may also be absent
/// from (`optional<T>`).
///
/// An optional holds a [`Shape`], never another optional, so the type the grammar refuses has no
/// spelling:
///
/// ```
/// use sce_forge_runtime::value_type::{Integer, Shape, ValueType};
/// const MAYBE: ValueType = ValueType::Optional(Shape::Integer(Integer::Int64));
/// ```
///
/// ```compile_fail
/// use sce_forge_runtime::value_type::{Integer, Shape, ValueType};
/// const TWICE: ValueType =
///     ValueType::Optional(ValueType::Optional(Shape::Integer(Integer::Int64)));
/// ```
///
/// A list of optionals and an optional list are both types, as the grammar's are:
///
/// ```
/// use sce_forge_runtime::value_type::{Integer, Shape, ValueType};
/// const MAYBE_EACH: ValueType = ValueType::Required(Shape::List {
///     elem: &ValueType::Optional(Shape::Integer(Integer::Uint16)),
///     max: 2,
/// });
/// const MAYBE_ALL: ValueType = ValueType::Optional(Shape::List {
///     elem: &ValueType::Required(Shape::Integer(Integer::Uint8)),
///     max: 2,
/// });
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    /// A value of the shape, always present.
    Required(Shape),
    /// `optional<T>`: absent, or a value of the shape.
    Optional(Shape),
}

impl ValueType {
    /// The shape a present value has.
    pub fn shape(&self) -> &Shape {
        match self {
            Self::Required(shape) | Self::Optional(shape) => shape,
        }
    }
}

/// Every constructor of the grammar but `optional<T>` (docs/adr/0086, section 1.1), each over every
/// type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A fixed-width integer.
    Integer(Integer),
    Float32,
    Float64,
    Bool,
    /// `enum:<alias>`: the enum's number, an integer of its underlying type.
    Enum(&'static EnumType),
    /// `string[B]`: a UTF-8 text of at most `max` bytes.
    Text {
        max: u32,
    },
    /// `bytes[B]`: a byte string of at most `max` bytes.
    Bytes {
        max: u32,
    },
    /// `record:<alias>`: a value of the struct an event schema declares.
    Record(&'static RecordType),
    /// `list<T>[B]`: an ordered sequence of at most `max` values of `elem`.
    List {
        elem: &'static ValueType,
        max: u32,
    },
}

impl Shape {
    /// The constructor at the top of the shape, as the grammar spells it and a refusal names it.
    pub fn constructor(&self) -> &'static str {
        match self {
            Self::Integer(integer) => integer.as_str(),
            Self::Float32 => "float32",
            Self::Float64 => "float64",
            Self::Bool => "bool",
            Self::Enum(_) => "enum",
            Self::Text { .. } => "string",
            Self::Bytes { .. } => "bytes",
            Self::Record(_) => "record",
            Self::List { .. } => "list",
        }
    }
}

/// An enum document (`sce:kind="enum"`), as a value of it is held: its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumType {
    /// The integer its numbers are (`sce:underlying-type`).
    pub underlying: Integer,
    /// Its variants, each with its declared name and number (`value=`).
    pub variants: &'static [Variant],
    /// Whether a number no variant declares is refused (`sce:strict-variants`, true unless the
    /// document says otherwise). An open enum holds every number of its underlying type, which is
    /// why it is its number, not its name, that a signature covers (docs/adr/0086, section 9.2).
    pub strict: bool,
}

/// A variant of an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    pub name: &'static str,
    /// The declared number, as the build holds it: one carrier for every underlying type.
    pub value: i128,
}

/// The struct an event schema declares, as the type of a record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecordType {
    /// Every field of the revision of the schema the build holds.
    pub fields: &'static [Field],
}

/// A field of a record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    /// The field's id: its key in the record's object or map.
    pub id: &'static str,
    pub ty: ValueType,
    /// The revision of the schema that added the field (`sce:since`), 1 for the first. Every value
    /// holds a field of the first revision; one a later revision added may be lacking, from a
    /// delivery of an earlier revision, and is then read as its default.
    pub since: u32,
    /// What a value that lacks the field holds (`sce:default`). Without one, a later field reads
    /// absent when it is optional and empty when it is a list (docs/adr/0086, section 10.2).
    pub default: Option<FieldDefault>,
}

/// A field's `sce:default`: a value of a type with no parts, which is all a default may be (a field
/// that holds a list or a record declares none).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldDefault {
    /// An integer, or an enum's number.
    Integer(i128),
    Float(f64),
    Bool(bool),
    Text(&'static str),
    Bytes(&'static [u8]),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_integer_spans_its_width_and_is_spelled_as_the_grammar_spells_it() {
        let widths = [
            (Integer::Uint8, "uint8", 0, 255),
            (Integer::Uint16, "uint16", 0, 65_535),
            (Integer::Uint32, "uint32", 0, 4_294_967_295),
            (Integer::Uint64, "uint64", 0, 18_446_744_073_709_551_615),
            (Integer::Int8, "int8", -128, 127),
            (Integer::Int16, "int16", -32_768, 32_767),
            (Integer::Int32, "int32", -2_147_483_648, 2_147_483_647),
            (
                Integer::Int64,
                "int64",
                -9_223_372_036_854_775_808,
                9_223_372_036_854_775_807,
            ),
        ];
        for (integer, spelled, least, greatest) in widths {
            assert_eq!(integer.range(), (least, greatest), "{spelled}");
            assert_eq!(integer.as_str(), spelled);
        }
    }

    #[test]
    fn a_shape_names_its_constructor_and_a_type_its_present_shape() {
        const LEVEL: EnumType = EnumType {
            underlying: Integer::Uint8,
            variants: &[Variant {
                name: "low",
                value: 0,
            }],
            strict: true,
        };
        const ITEM: RecordType = RecordType { fields: &[] };
        let shapes = [
            (Shape::Integer(Integer::Int32), "int32"),
            (Shape::Float32, "float32"),
            (Shape::Float64, "float64"),
            (Shape::Bool, "bool"),
            (Shape::Enum(&LEVEL), "enum"),
            (Shape::Text { max: 8 }, "string"),
            (Shape::Bytes { max: 4 }, "bytes"),
            (Shape::Record(&ITEM), "record"),
            (
                Shape::List {
                    elem: &ValueType::Required(Shape::Bool),
                    max: 2,
                },
                "list",
            ),
        ];
        for (shape, constructor) in shapes {
            assert_eq!(shape.constructor(), constructor);
            assert_eq!(*ValueType::Required(shape).shape(), shape);
            assert_eq!(*ValueType::Optional(shape).shape(), shape);
        }
    }
}
