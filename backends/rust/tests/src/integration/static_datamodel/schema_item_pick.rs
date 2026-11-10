// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
#![doc = "SCE-MAP: schema_item_pick.scxml:10 :: _forge_body"]
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "item.picked";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const TITLE_MAX_SIZE: u32 = 16;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 144;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:schema_item_pick:1";
/// The payload as a JSON Schema (2020-12): one spelling of each value, every
/// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_item_pick:1\",\"x-sce-source-hash\":\"43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"id\",\"title\",\"count\"],\"additionalProperties\":false}";

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaItemPickPayload {
    pub id: u32,
    pub title: ::sce_portable_bytes::SceString<16>,
    pub count: i32,
}
