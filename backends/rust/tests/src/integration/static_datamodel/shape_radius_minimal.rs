// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
#![doc = "SCE-MAP: shape_radius_minimal.scxml:3 :: _forge_body"]
// SCE-MAP: shape_radius_minimal.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.radius
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "shape.radius";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 16;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:shape_radius_minimal:1";
/// The payload as a JSON Schema (2020-12): one spelling of each value, every
/// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:shape_radius_minimal:1\",\"x-sce-source-hash\":\"6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"radius\"],\"additionalProperties\":false}";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeRadiusMinimalPayload {
    pub radius: u16,
}
