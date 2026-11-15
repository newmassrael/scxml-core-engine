// SCE-GENERATED — DO NOT EDIT
// source-hash: b33495759a4b590a534d88ab746efc69c5f9bf7afc18c08fa7dbc03fb93635f1
#![doc = "SCE-MAP: event-schema.scxml:4 :: _forge_body"]
// SCE-MAP: event-schema.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.completed
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "job.completed";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 25;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:event-schema:1";
/// The payload as a JSON Schema (2020-12): one spelling of each value, every
/// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event-schema:1\",\"x-sce-source-hash\":\"b33495759a4b590a534d88ab746efc69c5f9bf7afc18c08fa7dbc03fb93635f1\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"elapsed_ms\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"elapsed_ms\"],\"additionalProperties\":false}";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventSchemaPayload {
    pub elapsed_ms: u32,
}
