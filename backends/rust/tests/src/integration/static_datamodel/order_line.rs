// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: order_line.scxml:4 :: _forge_body"]
// SCE-MAP: order_line.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: order.line
// Do not edit — regenerate from the source SCXML file.

use super::paint_color;

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "order.line";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `sku` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const SKU_MAX_SIZE: u32 = 16;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 139;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:order_line:1";
/// The payload as a JSON Schema (2020-12): one spelling of each value, every
/// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:order_line:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sku\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"quantity\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"color\":{\"enum\":[\"red\",\"green\",\"blue\"]}},\"required\":[\"sku\",\"quantity\",\"color\"],\"additionalProperties\":false}";

#[derive(Debug, Clone, PartialEq)]
pub struct OrderLinePayload {
    pub sku: ::sce_portable_bytes::SceString<16>,
    pub quantity: u16,
    pub color: paint_color::PaintColor,
}
