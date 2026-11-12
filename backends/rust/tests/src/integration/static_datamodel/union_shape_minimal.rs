// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
#![doc = "SCE-MAP: union_shape_minimal.scxml:4 :: _forge_body"]
// SCE-MAP: union_shape_minimal.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::shape_kind;
use super::shape_radius_minimal;
use super::shape_rect_minimal;

/// The revision of the union: its tag enum's, `shape_kind`'s (a union declares none of its own).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes a value's JSON text takes: its largest case, every list full,
/// every text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 44;
/// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:union_shape_minimal:1";
/// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag
/// `kind` beside the fields of its record, every bound as this build resolved it.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_shape_minimal:1\",\"x-sce-source-hash\":\"6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false}]}";

/// A value of `union_shape_minimal`: exactly one case, told apart by its tag `kind`, a
/// value of `shape_kind::ShapeKind`. Two values are equal when they are in one case and
/// their records are equal.
#[derive(Debug, Clone, PartialEq)]
pub enum UnionShapeMinimal {
    /// The case `dot`, its tag alone.
    Dot,
    /// The case `circle`, holding its record.
    Circle(shape_radius_minimal::ShapeRadiusMinimalPayload),
    /// The case `rect`, holding its record.
    Rect(shape_rect_minimal::ShapeRectMinimalPayload),
}

#[allow(dead_code)]
impl UnionShapeMinimal {
    /// The case this value is in: the tag its text writes under `kind`.
    pub fn kind(&self) -> shape_kind::ShapeKind {
        match self {
            Self::Dot => shape_kind::ShapeKind::Dot,
            Self::Circle(_) => shape_kind::ShapeKind::Circle,
            Self::Rect(_) => shape_kind::ShapeKind::Rect,
        }
    }

    /// The record of the case `circle`, read where a test of the tag proved
    /// the value is in it.
    pub fn proven_circle(&self) -> &shape_radius_minimal::ShapeRadiusMinimalPayload {
        match self {
            Self::Circle(record) => record,
            _ => unreachable!(
                "a value of {} read as {} is in another case",
                "union_shape_minimal", "circle"
            ),
        }
    }

    /// The record of the case `rect`, read where a test of the tag proved
    /// the value is in it.
    pub fn proven_rect(&self) -> &shape_rect_minimal::ShapeRectMinimalPayload {
        match self {
            Self::Rect(record) => record,
            _ => unreachable!(
                "a value of {} read as {} is in another case",
                "union_shape_minimal", "rect"
            ),
        }
    }
}
