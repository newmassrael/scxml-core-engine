// SCE-GENERATED — DO NOT EDIT
// source-hash: e939b2700c7d391ee77811c0c5056f1bd75fa31a962245a8882cc047f3a138a0
#![doc = "SCE-MAP: schema_entry.scxml:7 :: _forge_body"]
// SCE-MAP: schema_entry.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ledger.entry
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "ledger.entry";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 38;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:schema_entry:1";
/// The payload as a JSON Schema (2020-12): one spelling of each value, every
/// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$id\":\"urn:sce:schema:schema_entry:1\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"amount\":{\"maximum\":2147483647,\"minimum\":-2147483648,\"type\":\"integer\"},\"id\":{\"maximum\":4294967295,\"minimum\":0,\"type\":\"integer\"}},\"required\":[\"amount\",\"id\"],\"type\":\"object\",\"x-sce-source-hash\":\"e939b2700c7d391ee77811c0c5056f1bd75fa31a962245a8882cc047f3a138a0\"}";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchemaEntryPayload {
    pub id: u32,
    pub amount: i32,
}
